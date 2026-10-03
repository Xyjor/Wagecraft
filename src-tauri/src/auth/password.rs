//! Argon2id hashing for passwords and kiosk PINs.
//!
//! Hashing takes about 100 ms on purpose, so callers run these inside
//! `tauri::async_runtime::spawn_blocking`, never directly on the async runtime.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

/// Hashes `password` with a fresh random salt. The result is a PHC string that
/// carries its own salt and parameters, so it is all we store.
pub fn hash(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hashed = Argon2::default() // Argon2id
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("hash failed: {e}"))?;
    Ok(hashed.to_string())
}

/// True only when `password` matches `stored`. A malformed `stored` value is a mismatch.
pub fn verify(password: &str, stored: &str) -> bool {
    PasswordHash::new(stored)
        .map(|h| {
            Argon2::default()
                .verify_password(password.as_bytes(), &h)
                .is_ok()
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify() {
        let stored = hash("correct horse battery").expect("hash");
        assert!(stored.starts_with("$argon2id$"));
        assert!(verify("correct horse battery", &stored));
        assert!(!verify("wrong password", &stored));
    }

    #[test]
    fn same_password_gets_a_different_salt_each_time() {
        let a = hash("123456").expect("hash");
        let b = hash("123456").expect("hash");
        assert_ne!(a, b);
        assert!(verify("123456", &a) && verify("123456", &b));
    }

    #[test]
    fn garbage_stored_value_never_verifies() {
        assert!(!verify("anything", "not a hash"));
        assert!(!verify("", ""));
    }
}
