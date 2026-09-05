// Reused from BSLT R1; only the shared error import changed.
use crate::platform::http::Error;
use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};
use sha2::{Digest, Sha256};
fn argon2() -> Result<Argon2<'static>, Error> {
    let params = Params::new(19 * 1024, 2, 1, None).map_err(|_| Error::Internal)?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}
pub(super) fn hash_password(password: &str) -> Result<String, Error> {
    let mut salt = [0_u8; 16];
    getrandom::fill(&mut salt).map_err(|_| Error::Internal)?;
    let salt = SaltString::encode_b64(&salt).map_err(|_| Error::Internal)?;
    argon2()?
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| Error::Internal)
}
pub(super) fn verify(password: &str, encoded: Option<&str>) -> Result<bool, Error> {
    match encoded {
        Some(encoded) => {
            let parsed = PasswordHash::new(encoded).map_err(|_| Error::Internal)?;
            Ok(argon2()?
                .verify_password(password.as_bytes(), &parsed)
                .is_ok())
        }
        None => {
            let _ = hash_password(password)?;
            Ok(false)
        }
    }
}
pub(super) fn token() -> Result<String, Error> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| Error::Internal)?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}
pub(super) fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn passwords_are_salted_and_verified() {
        let first = hash_password("correct horse test password").unwrap();
        let second = hash_password("correct horse test password").unwrap();
        assert_ne!(first, second);
        assert!(first.starts_with("$argon2id$"));
        assert!(verify("correct horse test password", Some(&first)).unwrap());
        assert!(!verify("wrong password", Some(&first)).unwrap());
        assert!(!verify("wrong password", None).unwrap());
    }
    #[test]
    fn tokens_are_random_and_stored_by_digest() {
        let first = token().unwrap();
        assert_eq!(first.len(), 64);
        assert_ne!(first, token().unwrap());
        assert_ne!(first, digest(&first));
    }
}
