use crate::platform::http::Error;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Credentials {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub remember_me: bool,
}
impl Credentials {
    pub fn validate(mut self, registering: bool) -> Result<Self, Error> {
        self.email = normalize_email(&self.email)?;
        validate_password(&self.password, registering)?;
        Ok(self)
    }
}
// Port of the selected BSLT local contract. Passwords are never trimmed.
pub(super) fn normalize_email(value: &str) -> Result<String, Error> {
    let email = value.trim().to_ascii_lowercase();
    let mut parts = email.split('@');
    let local = parts.next().unwrap_or_default();
    let domain = parts.next().unwrap_or_default();
    let valid = email.len() <= 254
        && !local.is_empty()
        && !domain.is_empty()
        && parts.next().is_none()
        && email.is_ascii()
        && !email
            .bytes()
            .any(|b| b.is_ascii_whitespace() || b.is_ascii_control())
        && local
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._+-%".contains(&b))
        && domain.split('.').all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        });
    if !valid {
        return Err(Error::InvalidInput);
    }
    Ok(email)
}
pub(super) fn validate_password(value: &str, new: bool) -> Result<(), Error> {
    let length = value.chars().count();
    if value.len() > 512 || length > 128 || length < if new { 15 } else { 1 } {
        return Err(Error::InvalidInput);
    }
    Ok(())
}
pub(super) fn validate_token(token: &str) -> Result<(), Error> {
    if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::InvalidToken);
    }
    Ok(())
}
#[derive(Serialize)]
pub struct UserView {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub theme: String,
    pub email_verified: bool,
}
#[derive(Serialize)]
pub struct SessionView {
    pub user: UserView,
    pub expires_at: i64,
    pub idle_timeout_seconds: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EmailRequest {
    pub email: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TokenRequest {
    pub token: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ResetRequest {
    pub token: String,
    pub password: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ChangePassword {
    pub current_password: String,
    pub password: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProfileRequest {
    pub display_name: String,
    pub theme: String,
}
impl ProfileRequest {
    pub fn validate(mut self) -> Result<Self, Error> {
        self.display_name = self.display_name.trim().into();
        if self.display_name.chars().count() > 80
            || self.display_name.chars().any(char::is_control)
            || !matches!(self.theme.as_str(), "system" | "light" | "dark")
        {
            return Err(Error::InvalidInput);
        }
        Ok(self)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_password_and_normalizes_email() {
        let value = Credentials {
            email: " A+test@Example.test ".into(),
            password: "  sixteen letters  ".into(),
            remember_me: false,
        }
        .validate(true)
        .unwrap();
        assert_eq!(value.email, "a+test@example.test");
        assert_eq!(value.password, "  sixteen letters  ");
    }
    #[test]
    fn rejects_invalid_credentials() {
        for email in ["", "a@@b", "a@", "a\n@b", "a@-b", "a@b..c"] {
            assert!(normalize_email(email).is_err());
        }
        assert!(validate_password("short", true).is_err());
        assert!(validate_password(&"x".repeat(129), true).is_err());
        assert!(validate_password(&"가".repeat(15), true).is_ok());
        assert!(validate_token("not-a-token").is_err());
    }
}
