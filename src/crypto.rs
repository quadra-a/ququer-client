use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub fn generate_nonce() -> String {
    Uuid::new_v4().to_string()
}

pub fn commit_hash(data: &str, nonce: &str) -> String {
    let input = format!("{}:{}", data, nonce);
    let hash = Sha256::digest(input.as_bytes());
    hex::encode(hash)
}

pub fn sign_bytes(key: &SigningKey, msg: &[u8]) -> String {
    let sig = key.sign(msg);
    hex::encode(sig.to_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Verifier;
    use sha2::{Digest, Sha256};

    #[test]
    fn commit_hash_deterministic() {
        let h1 = commit_hash("hello", "nonce123");
        let h2 = commit_hash("hello", "nonce123");
        assert_eq!(h1, h2);
    }

    #[test]
    fn commit_hash_is_64_char_hex() {
        let h = commit_hash("data", "nonce");
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn commit_hash_matches_protocol() {
        let data = r#"{"action":"rock"}"#;
        let nonce = "test-nonce";
        let expected_input = format!("{}:{}", data, nonce);
        let expected = hex::encode(Sha256::digest(expected_input.as_bytes()));
        assert_eq!(commit_hash(data, nonce), expected);
    }

    #[test]
    fn commit_hash_different_inputs_differ() {
        assert_ne!(commit_hash("a", "n"), commit_hash("b", "n"));
        assert_ne!(commit_hash("a", "n1"), commit_hash("a", "n2"));
    }

    #[test]
    fn sign_bytes_deterministic() {
        let key = SigningKey::from_bytes(&[1u8; 32]);
        let s1 = sign_bytes(&key, b"msg");
        let s2 = sign_bytes(&key, b"msg");
        assert_eq!(s1, s2);
    }

    #[test]
    fn sign_bytes_verifiable() {
        let key = SigningKey::from_bytes(&[2u8; 32]);
        let sig_hex = sign_bytes(&key, b"hello world");
        let sig_bytes = hex::decode(&sig_hex).unwrap();
        let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes.try_into().unwrap());
        assert!(key.verifying_key().verify(b"hello world", &sig).is_ok());
    }

    #[test]
    fn sign_bytes_wrong_message_fails() {
        let key = SigningKey::from_bytes(&[3u8; 32]);
        let sig_hex = sign_bytes(&key, b"correct");
        let sig_bytes = hex::decode(&sig_hex).unwrap();
        let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes.try_into().unwrap());
        assert!(key.verifying_key().verify(b"wrong", &sig).is_err());
    }

    #[test]
    fn generate_nonce_unique() {
        let n1 = generate_nonce();
        let n2 = generate_nonce();
        assert_ne!(n1, n2);
    }

    #[test]
    fn generate_nonce_is_uuid_format() {
        let n = generate_nonce();
        assert_eq!(n.len(), 36); // UUID v4 string length
        assert_eq!(n.chars().filter(|&c| c == '-').count(), 4);
    }
}
