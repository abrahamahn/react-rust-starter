// BSLT R1 hashing admission and bounded throttle, extended for account lifecycle.
use super::{crypto, domain::*, store::Store};
use crate::{app::config::Config, platform::{db::Database, http::Error, mail::{Delivery, Mailer}}};
use std::{collections::HashMap, sync::{Arc, Mutex}, time::{Duration, Instant}};
use tokio::sync::Semaphore;
#[derive(Clone)]
pub struct Service {
    pub(super) store: Store,
    pub(super) config: Arc<Config>,
    hashing: Arc<Semaphore>,
    throttle: Arc<Mutex<Throttle>>,
    mailer: Mailer,
}
struct Throttle { global: (Instant, usize), accounts: HashMap<String, (Instant, usize)> }
impl Throttle {
    fn admit(&mut self, account: &str, now: Instant) -> Result<(), Error> {
        if now.duration_since(self.global.0)>=Duration::from_secs(60) { self.global=(now,0); }
        if self.global.1>=60 { return Err(Error::RateLimited); }
        self.accounts.retain(|_,(at,_)| now.duration_since(*at)<Duration::from_secs(300));
        let key=crypto::digest(account);
        if !self.accounts.contains_key(&key) && self.accounts.len()>=512 { return Err(Error::RateLimited); }
        let entry=self.accounts.entry(key).or_insert((now,0));
        if entry.1>=10 { return Err(Error::RateLimited); }
        entry.1+=1; self.global.1+=1; Ok(())
    }
}
impl Service {
    pub fn new(database: Database, config: Config, mailer: Mailer) -> Self {
        Self { store: Store(database), config: Arc::new(config), mailer, hashing: Arc::new(Semaphore::new(2)),
            throttle: Arc::new(Mutex::new(Throttle { global:(Instant::now(),0), accounts:HashMap::new() })) }
    }
    pub(super) fn admit(&self, key: &str) -> Result<(), Error> {
        self.throttle.lock().map_err(|_| Error::Internal)?.admit(key,Instant::now())
    }
    async fn hash(&self, password: String) -> Result<String, Error> {
        let permit=self.hashing.clone().try_acquire_owned().map_err(|_| Error::RateLimited)?;
        tokio::task::spawn_blocking(move || { let _permit=permit; crypto::hash_password(&password) }).await.map_err(|_| Error::Internal)?
    }
    async fn verify(&self, password: String, hash: Option<String>) -> Result<(), Error> {
        let permit=self.hashing.clone().try_acquire_owned().map_err(|_| Error::RateLimited)?;
        let valid=tokio::task::spawn_blocking(move || { let _permit=permit; crypto::verify(&password,hash.as_deref()) }).await.map_err(|_| Error::Internal)??;
        if !valid { return Err(Error::InvalidCredentials); } Ok(())
    }
    fn link(&self, to: String, token: &str, purpose: &str) -> Delivery {
        let title=if purpose=="verify" { "Verify your email" } else { "Reset your password" };
        // Fragment is never sent in HTTP request URLs or Referer headers.
        let link=format!("{}/#{}={}",self.config.origin,purpose,token);
        Delivery { to, subject:title.into(), text:format!("{title}\n\n{link}\n\nThis single-use link expires soon. If you did not request it, ignore this email.") }
    }
    pub(super) async fn register(&self, input: Credentials) -> Result<(SessionView,String,bool),Error> {
        let input=input.validate(true)?;
        self.admit(&input.email)?;
        let mail=self.mailer.reserve()?;
        let hash=self.hash(input.password).await?;
        let token=crypto::token()?; let verify=crypto::token()?;
        let view=self.store.register(&input.email,&hash,&crypto::digest(&token),&crypto::digest(&verify),input.remember_me).await?;
        mail.send(self.link(input.email,&verify,"verify"));
        Ok((view,token,input.remember_me))
    }
    pub(super) async fn login(&self,input:Credentials,old:Option<String>)->Result<(SessionView,String,bool),Error> {
        let input=input.validate(false)?; self.admit(&input.email)?;
        let record=self.store.password(&input.email).await?;
        self.verify(input.password,record.as_ref().map(|r| r.hash.clone())).await?;
        let record=record.ok_or(Error::InvalidCredentials)?;
        let token=crypto::token()?;
        let old_hash=old.as_deref().map(crypto::digest);
        let view=self.store.login(&record,&crypto::digest(&token),old_hash.as_deref(),input.remember_me).await?;
        Ok((view,token,input.remember_me))
    }
    pub(super) async fn request_link(&self,input:EmailRequest,purpose:&str)->Result<(),Error> {
        let start=tokio::time::Instant::now();
        let email=normalize_email(&input.email)?; self.admit(&email)?;
        // Reserve the same bounded queue capacity for known and unknown accounts.
        let mail=self.mailer.reserve()?;
        let token=crypto::token()?;
        if self.store.issue(&email,purpose,&crypto::digest(&token)).await? { mail.send(self.link(email,&token,purpose)); }
        tokio::time::sleep_until(start+Duration::from_millis(300)).await;
        Ok(())
    }
    pub(super) async fn confirm(&self,input:TokenRequest)->Result<(),Error> {
        validate_token(&input.token)?; self.admit(&input.token)?;
        self.store.consume(&crypto::digest(&input.token),"verify",None).await?; Ok(())
    }
    pub(super) async fn reset(&self,input:ResetRequest)->Result<(),Error> {
        validate_token(&input.token)?; validate_password(&input.password,true)?;
        self.admit(&input.token)?;
        let mail=self.mailer.reserve()?;
        let hash=self.hash(input.password).await?;
        let email=self.store.consume(&crypto::digest(&input.token),"reset",Some(&hash)).await?;
        mail.send(Delivery { to:email,subject:"Password changed".into(),text:"Your password was reset. All previous sessions were revoked. Sign in normally with your new password.".into() });
        Ok(())
    }
    pub(super) async fn change(&self,digest:&str,input:ChangePassword)->Result<(),Error> {
        validate_password(&input.current_password,false)?; validate_password(&input.password,true)?;
        let view=self.store.current(digest).await?; self.admit(&view.user.email)?;
        let record=self.store.password(&view.user.email).await?.ok_or(Error::Unauthenticated)?;
        self.verify(input.current_password,Some(record.hash.clone())).await?;
        let mail=self.mailer.reserve()?;
        let hash=self.hash(input.password).await?;
        self.store.change_password(digest,&record,&hash).await?;
        mail.send(Delivery { to:view.user.email,subject:"Password changed".into(),text:"Your password was changed. All previous sessions were revoked.".into() });
        Ok(())
    }
    pub fn session_digest(&self,headers:&axum::http::HeaderMap)->Result<String,Error> {
        self.session_token(headers).map(|s| crypto::digest(&s)).ok_or(Error::Unauthenticated)
    }
    pub(super) fn session_token(&self,headers:&axum::http::HeaderMap)->Option<String> {
        let mut tokens=headers.get_all("cookie").iter().filter_map(|v| v.to_str().ok()).flat_map(|v| v.split(';'))
            .filter_map(|item| item.trim().split_once('='))
            .filter(|(name,_)| *name==self.config.cookie_name()).map(|(_,value)| value);
        let first=tokens.next()?;
        if tokens.next().is_some() || validate_token(first).is_err() { return None; }
        Some(first.into())
    }
    pub fn check_origin(&self,headers:&axum::http::HeaderMap)->Result<(),Error> {
        if headers.get_all("origin").iter().count()!=1 || headers.get("origin").and_then(|h|h.to_str().ok())!=Some(self.config.origin.as_str())
            || headers.get("x-starter-client").and_then(|h|h.to_str().ok())!=Some("web-v1") { return Err(Error::Forbidden); }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn throttle_is_bounded_and_expires() {
        let start=Instant::now(); let mut throttle=Throttle{global:(start,0),accounts:HashMap::new()};
        for _ in 0..10 { throttle.admit("user@example.test",start).unwrap(); }
        assert!(matches!(throttle.admit("user@example.test",start),Err(Error::RateLimited)));
        throttle.admit("user@example.test",start+Duration::from_secs(301)).unwrap();
        assert_eq!(throttle.accounts.len(),1);
    }
}
