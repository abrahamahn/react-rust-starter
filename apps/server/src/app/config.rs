// Adapted from BSLT's injectable, fail-closed local configuration.
use std::{env, net::{IpAddr, SocketAddr}, time::Duration};
use tokio_postgres::config::{Host, SslMode};
use crate::platform::http::Error;

// No Debug: contains database and SMTP credentials.
#[derive(Clone)]
pub struct Config {
    pub bind: SocketAddr,
    pub database: tokio_postgres::Config,
    pub origin: String,
    pub production: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_user: Option<String>,
    pub smtp_password: Option<String>,
    pub mail_from: String,
}
impl Config {
    pub fn from_env() -> Result<Self, Error> { Self::from_lookup(|key| env::var(key).ok()) }
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, Error> {
        let mode = get("APP_MODE").unwrap_or_else(|| "development".into());
        if !matches!(mode.as_str(), "development" | "test" | "production") {
            return Err(Error::Config("invalid APP_MODE"));
        }
        let production = mode == "production";
        let bind = get("APP_BIND").unwrap_or_else(|| "127.0.0.1:8088".into())
            .parse::<SocketAddr>().map_err(|_| Error::Config("invalid APP_BIND"))?;
        if !bind.ip().is_loopback() || bind.port() == 0 {
            return Err(Error::Config("bind must be loopback; use the documented HTTPS reverse proxy"));
        }
        let origin = get("APP_ORIGIN").unwrap_or_else(|| "http://127.0.0.1:5178".into());
        let parsed = url::Url::parse(&origin).map_err(|_| Error::Config("invalid APP_ORIGIN"))?;
        if parsed.origin().ascii_serialization() != origin || parsed.path() != "/" || parsed.query().is_some() || parsed.fragment().is_some() || !parsed.username().is_empty() || parsed.password().is_some() {
            return Err(Error::Config("APP_ORIGIN must be an exact origin without a path or credentials"));
        }
        if production {
            if parsed.scheme() != "https" { return Err(Error::Config("production requires HTTPS APP_ORIGIN")); }
        } else if parsed.scheme() != "http" || !parsed.host_str().unwrap_or_default().trim_matches(['[', ']']).parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback()) {
            return Err(Error::Config("development origin must be a loopback HTTP IP"));
        }
        let dsn = get("APP_DATABASE_URL").ok_or(Error::Config("APP_DATABASE_URL is required"))?;
        let mut database = dsn.parse::<tokio_postgres::Config>().map_err(|_| Error::Config("invalid database configuration"))?;
        let local = database.get_hosts().len() == 1 && match &database.get_hosts()[0] {
            Host::Tcp(host) => host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback()),
            _ => false,
        };
        if !local || !database.get_hostaddrs().is_empty() || database.get_options().is_some() || database.get_ssl_mode() != SslMode::Disable || database.get_user().is_none() {
            return Err(Error::Config("database requires explicit loopback IP, user and sslmode=disable; no overrides"));
        }
        if !database.get_dbname().is_some_and(|name| name.starts_with("starter_") && name.len() > 8) {
            return Err(Error::Config("use a separate database named starter_<name>"));
        }
        database.connect_timeout(Duration::from_secs(3));
        database.application_name("react-rust-starter");
        let smtp_host = get("SMTP_HOST").unwrap_or_else(|| "127.0.0.1".into());
        let smtp_port = get("SMTP_PORT").unwrap_or_else(|| if production { "587" } else { "1025" }.into())
            .parse::<u16>().map_err(|_| Error::Config("invalid SMTP_PORT"))?;
        let smtp_user = get("SMTP_USERNAME").filter(|s| !s.is_empty());
        let smtp_password = get("SMTP_PASSWORD").filter(|s| !s.is_empty());
        let mail_from = get("MAIL_FROM").unwrap_or_else(|| "Starter <noreply@example.test>".into());
        if smtp_port == 0 || smtp_user.is_some() != smtp_password.is_some() {
            return Err(Error::Config("invalid SMTP credentials or port"));
        }
        if production {
            if smtp_user.is_none() || get("MAIL_FROM").is_none() || get("SMTP_HOST").is_none() {
                return Err(Error::Config("production requires explicit SMTP host, credentials and MAIL_FROM"));
            }
        } else if !smtp_host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback()) {
            return Err(Error::Config("development SMTP must be loopback"));
        }
        Ok(Self { bind, database, origin, production, smtp_host, smtp_port, smtp_user, smtp_password, mail_from })
    }
    pub fn cookie_name(&self) -> &'static str {
        if self.production { "__Host-starter_session" } else { "starter_session" }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config(overrides: &[(&str, &str)]) -> Result<Config, Error> {
        Config::from_lookup(|key| overrides.iter().find(|(k, _)| *k == key).map(|(_, v)| (*v).into()).or_else(||
            (key == "APP_DATABASE_URL").then(|| "postgresql://tester:local-test@127.0.0.1/starter_test?sslmode=disable".into())))
    }
    #[test] fn accepts_local_and_hides_secrets() {
        assert_eq!(config(&[]).unwrap().bind.port(), 8088);
        let error = config(&[("APP_DATABASE_URL", "private-secret-invalid")]).err().unwrap();
        assert!(!error.to_string().contains("private-secret"));
    }
    #[test] fn rejects_unsafe_configuration() {
        for values in [vec![("APP_BIND", "0.0.0.0:8088")], vec![("APP_MODE", "production")], vec![("APP_ORIGIN", "http://127.0.0.1:5178/path")], vec![("APP_ORIGIN", "http://127.0.0.1:5178#token")], vec![("APP_DATABASE_URL", "host=127.0.0.1 hostaddr=10.0.0.1 user=u dbname=starter_test sslmode=disable")], vec![("SMTP_HOST", "smtp.example.com")]] {
            assert!(config(&values).is_err());
        }
    }
}
