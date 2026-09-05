// Adapted from BSLT R1 session store and TS recovery flow.
// Lock order: user, then sessions/tokens. Recheck after slow KDF work.
use super::{
    domain::{ProfileRequest, SessionView, UserView},
    session_policy,
};
use crate::platform::{db::Database, http::Error};
use tokio_postgres::{Row, Transaction, error::SqlState};
#[derive(Clone)]
pub(super) struct Store(pub Database);
pub(super) struct PasswordRecord {
    pub id: String,
    pub hash: String,
}
fn user(row: &Row) -> UserView {
    UserView {
        id: row.get("id"),
        email: row.get("email"),
        display_name: row.get("display_name"),
        theme: row.get("theme"),
        email_verified: row.get("email_verified"),
    }
}
async fn limits(tx: &Transaction<'_>) -> Result<(), Error> {
    tx.batch_execute("SET LOCAL lock_timeout='3s'; SET LOCAL statement_timeout='5s';")
        .await?;
    Ok(())
}
/// Authorization and a protected write share this short transaction. Revocation
/// serializes with already-authorized writes and blocks later ones.
pub async fn authorize(
    tx: &Transaction<'_>,
    token_hash: &str,
    verified: bool,
) -> Result<SessionView, Error> {
    limits(tx).await?;
    let locked = tx.query_opt("SELECT u.id FROM starter.users u JOIN starter.sessions s ON s.user_id=u.id WHERE s.token_hash=$1 FOR UPDATE OF u", &[&token_hash]).await?;
    if locked.is_none() {
        return Err(Error::Unauthenticated);
    }
    // now() is the transaction start time, potentially before the lock wait.
    // Check the real clock after acquiring the user lock, including idle expiry.
    let row = tx.query_opt("UPDATE starter.sessions s SET last_seen_at=clock_timestamp() FROM starter.users u WHERE s.token_hash=$1 AND u.id=s.user_id AND NOT u.disabled AND s.auth_epoch=u.auth_epoch AND s.expires_at>clock_timestamp() AND s.last_seen_at>clock_timestamp()-s.idle_seconds*interval '1 second' RETURNING u.id,u.email,u.display_name,u.theme,u.email_verified,extract(epoch FROM s.expires_at)::bigint AS expires_at,s.idle_seconds", &[&token_hash]).await?.ok_or(Error::Unauthenticated)?;
    let view = SessionView {
        user: user(&row),
        expires_at: row.get("expires_at"),
        idle_timeout_seconds: row.get("idle_seconds"),
    };
    if verified && !view.user.email_verified {
        return Err(Error::Unverified);
    }
    Ok(view)
}
async fn insert_session(
    tx: &Transaction<'_>,
    id: &str,
    digest: &str,
    epoch: i64,
    remember: bool,
) -> Result<(), Error> {
    let span = session_policy::span_seconds(remember);
    let idle = session_policy::idle_seconds(span);
    tx.execute("INSERT INTO starter.sessions(token_hash,user_id,auth_epoch,expires_at,idle_seconds) VALUES($1,$2,$3,now()+$4::bigint*interval '1 second',$5)", &[&digest,&id,&epoch,&span,&idle]).await?;
    Ok(())
}
async fn issue_token(
    tx: &Transaction<'_>,
    id: &str,
    purpose: &str,
    digest: &str,
) -> Result<(), Error> {
    tx.execute(
        "DELETE FROM starter.auth_tokens WHERE user_id=$1 AND purpose=$2",
        &[&id, &purpose],
    )
    .await?;
    let seconds: i64 = if purpose == "verify" { 3600 } else { 1800 };
    tx.execute("INSERT INTO starter.auth_tokens(token_hash,user_id,purpose,expires_at) VALUES($1,$2,$3,now()+$4::bigint*interval '1 second')", &[&digest,&id,&purpose,&seconds]).await?;
    Ok(())
}
impl Store {
    pub async fn password(&self, email: &str) -> Result<Option<PasswordRecord>, Error> {
        Ok(self
            .0
            .client()
            .await?
            .query_opt(
                "SELECT id,password_hash FROM starter.users WHERE email=$1 AND NOT disabled",
                &[&email],
            )
            .await?
            .map(|row| PasswordRecord {
                id: row.get(0),
                hash: row.get(1),
            }))
    }
    pub async fn register(
        &self,
        email: &str,
        hash: &str,
        digest: &str,
        verify_digest: &str,
        remember: bool,
    ) -> Result<SessionView, Error> {
        let mut client = self.0.client().await?;
        let tx = client.transaction().await?;
        limits(&tx).await?;
        let id = uuid::Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO starter.users(id,email,password_hash) VALUES($1,$2,$3)",
            &[&id, &email, &hash],
        )
        .await
        .map_err(|e| {
            if e.code() == Some(&SqlState::UNIQUE_VIOLATION) {
                Error::AccountUnavailable
            } else {
                Error::Unavailable
            }
        })?;
        insert_session(&tx, &id, digest, 0, remember).await?;
        issue_token(&tx, &id, "verify", verify_digest).await?;
        let view = authorize(&tx, digest, false).await?;
        tx.commit().await?;
        Ok(view)
    }
    pub async fn login(
        &self,
        record: &PasswordRecord,
        digest: &str,
        old: Option<&str>,
        remember: bool,
    ) -> Result<SessionView, Error> {
        let mut client = self.0.client().await?;
        let tx = client.transaction().await?;
        limits(&tx).await?;
        let row = tx.query_opt("SELECT auth_epoch FROM starter.users WHERE id=$1 AND password_hash=$2 AND NOT disabled FOR UPDATE", &[&record.id,&record.hash]).await?.ok_or(Error::InvalidCredentials)?;
        if let Some(old) = old {
            tx.execute(
                "DELETE FROM starter.sessions WHERE token_hash=$1 AND user_id=$2",
                &[&old, &record.id],
            )
            .await?;
        }
        let epoch: i64 = row.get(0);
        insert_session(&tx, &record.id, digest, epoch, remember).await?;
        let view = authorize(&tx, digest, false).await?;
        tx.commit().await?;
        Ok(view)
    }
    pub async fn current(&self, digest: &str) -> Result<SessionView, Error> {
        let mut client = self.0.client().await?;
        let tx = client.transaction().await?;
        let view = authorize(&tx, digest, false).await?;
        tx.commit().await?;
        Ok(view)
    }
    pub async fn logout(&self, digest: &str, all: bool) -> Result<(), Error> {
        let mut client = self.0.client().await?;
        let tx = client.transaction().await?;
        if all {
            let view = authorize(&tx, digest, false).await?;
            tx.execute(
                "UPDATE starter.users SET auth_epoch=auth_epoch+1 WHERE id=$1",
                &[&view.user.id],
            )
            .await?;
            tx.execute(
                "DELETE FROM starter.sessions WHERE user_id=$1",
                &[&view.user.id],
            )
            .await?;
        } else {
            tx.execute(
                "DELETE FROM starter.sessions WHERE token_hash=$1",
                &[&digest],
            )
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
    pub async fn issue(&self, email: &str, purpose: &str, digest: &str) -> Result<bool, Error> {
        let mut client = self.0.client().await?;
        let tx = client.transaction().await?;
        limits(&tx).await?;
        let row = tx.query_opt("SELECT id,email_verified FROM starter.users WHERE email=$1 AND NOT disabled FOR UPDATE", &[&email]).await?;
        let Some(row) = row else {
            return Ok(false);
        };
        let verified: bool = row.get(1);
        if purpose == "verify" && verified {
            return Ok(false);
        }
        let id: String = row.get(0);
        issue_token(&tx, &id, purpose, digest).await?;
        tx.commit().await?;
        Ok(true)
    }
    pub async fn consume(
        &self,
        digest: &str,
        purpose: &str,
        password_hash: Option<&str>,
    ) -> Result<String, Error> {
        let mut client = self.0.client().await?;
        let tx = client.transaction().await?;
        limits(&tx).await?;
        let row = tx.query_opt("SELECT u.id,u.email FROM starter.users u JOIN starter.auth_tokens t ON t.user_id=u.id WHERE t.token_hash=$1 AND NOT u.disabled FOR UPDATE OF u", &[&digest]).await?.ok_or(Error::InvalidToken)?;
        let id: String = row.get(0);
        let email: String = row.get(1);
        let used = tx.execute("UPDATE starter.auth_tokens SET used_at=now() WHERE token_hash=$1 AND user_id=$2 AND purpose=$3 AND used_at IS NULL AND expires_at>clock_timestamp()", &[&digest,&id,&purpose]).await?;
        if used != 1 {
            return Err(Error::InvalidToken);
        }
        if purpose == "verify" {
            tx.execute(
                "UPDATE starter.users SET email_verified=true WHERE id=$1",
                &[&id],
            )
            .await?;
        } else {
            let hash = password_hash.ok_or(Error::Internal)?;
            tx.execute(
                "UPDATE starter.users SET password_hash=$1,auth_epoch=auth_epoch+1 WHERE id=$2",
                &[&hash, &id],
            )
            .await?;
            tx.execute("DELETE FROM starter.sessions WHERE user_id=$1", &[&id])
                .await?;
            tx.execute("DELETE FROM starter.auth_tokens WHERE user_id=$1", &[&id])
                .await?;
        }
        tx.commit().await?;
        Ok(email)
    }
    pub async fn change_password(
        &self,
        digest: &str,
        record: &PasswordRecord,
        new_hash: &str,
    ) -> Result<(), Error> {
        let mut client = self.0.client().await?;
        let tx = client.transaction().await?;
        let view = authorize(&tx, digest, false).await?;
        if view.user.id != record.id {
            return Err(Error::Unauthenticated);
        }
        let changed = tx.execute("UPDATE starter.users SET password_hash=$1,auth_epoch=auth_epoch+1 WHERE id=$2 AND password_hash=$3", &[&new_hash,&record.id,&record.hash]).await?;
        if changed != 1 {
            return Err(Error::InvalidCredentials);
        }
        tx.execute(
            "DELETE FROM starter.sessions WHERE user_id=$1",
            &[&record.id],
        )
        .await?;
        tx.execute(
            "DELETE FROM starter.auth_tokens WHERE user_id=$1",
            &[&record.id],
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn profile(&self, digest: &str, input: &ProfileRequest) -> Result<UserView, Error> {
        let mut client = self.0.client().await?;
        let tx = client.transaction().await?;
        let view = authorize(&tx, digest, false).await?;
        let row = tx.query_one("UPDATE starter.users SET display_name=$1,theme=$2 WHERE id=$3 RETURNING id,email,display_name,theme,email_verified", &[&input.display_name,&input.theme,&view.user.id]).await?;
        let view = user(&row);
        tx.commit().await?;
        Ok(view)
    }
}
