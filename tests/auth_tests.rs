#[cfg(test)]
mod tests {
    use cheraghtunnel::db;

    #[test]
    fn test_argon2id_password_hashing_and_verification() {
        let password = "SuperSecretPassword123!";
        let hashed = db::hash_password(password);

        assert!(hashed.starts_with("$argon2id$"));
        assert!(db::verify_password(password, &hashed));
        assert!(!db::verify_password("WrongPassword", &hashed));
    }

    #[test]
    fn test_legacy_sha256_fallback_verification() {
        let password = "LegacyPassword";
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        let legacy_sha256 = format!("{:x}", hasher.finalize());

        assert_eq!(legacy_sha256.len(), 64);
        assert!(db::verify_password(password, &legacy_sha256));
        assert!(!db::verify_password("WrongLegacyPassword", &legacy_sha256));
    }

    #[test]
    fn test_session_tokens_lifecycle() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_cheragh.db");

        db::init_db(&db_path).unwrap();

        let token = "test_session_token_123456789";
        assert!(!db::is_session_valid(&db_path, token));

        db::create_session(&db_path, token).unwrap();
        assert!(db::is_session_valid(&db_path, token));

        db::delete_session(&db_path, token).unwrap();
        assert!(!db::is_session_valid(&db_path, token));
    }
}
