use super::crypto;
use crate::platform::http::Error;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

pub(super) struct Throttle {
    global: (Instant, usize),
    accounts: HashMap<String, (Instant, usize)>,
}

impl Throttle {
    pub(super) fn new(now: Instant) -> Self {
        Self {
            global: (now, 0),
            accounts: HashMap::new(),
        }
    }

    pub(super) fn admit(&mut self, account: &str, now: Instant) -> Result<(), Error> {
        if now.duration_since(self.global.0) >= Duration::from_secs(60) {
            self.global = (now, 0);
        }
        if self.global.1 >= 60 {
            return Err(Error::RateLimited);
        }
        self.accounts
            .retain(|_, (at, _)| now.duration_since(*at) < Duration::from_secs(300));
        let key = crypto::digest(account);
        if !self.accounts.contains_key(&key) && self.accounts.len() >= 512 {
            return Err(Error::RateLimited);
        }
        let entry = self.accounts.entry(key).or_insert((now, 0));
        if entry.1 >= 10 {
            return Err(Error::RateLimited);
        }
        entry.1 += 1;
        self.global.1 += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn throttle_is_bounded_and_expires() {
        let start = Instant::now();
        let mut throttle = Throttle::new(start);
        for _ in 0..10 {
            throttle.admit("user@example.test", start).unwrap();
        }
        assert!(matches!(
            throttle.admit("user@example.test", start),
            Err(Error::RateLimited)
        ));
        throttle
            .admit("user@example.test", start + Duration::from_secs(301))
            .unwrap();
        assert_eq!(throttle.accounts.len(), 1);
    }
}
