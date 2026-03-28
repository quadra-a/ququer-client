use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub fn generate_nonce() -> String {
    Uuid::new_v4().to_string()
}

/// SHA-256(JSON.stringify(data) + ":" + nonce) → hex string
/// Matches platform's hashCommitData in shared/utils.ts
pub fn commit_hash(data: &str, nonce: &str) -> String {
    let input = format!("{}:{}", data, nonce);
    let hash = Sha256::digest(input.as_bytes());
    hex::encode(hash)
}

/// Ed25519 sign → base64 (platform expects base64 signatures)
pub fn sign_bytes(key: &SigningKey, msg: &[u8]) -> String {
    let sig = key.sign(msg);
    BASE64.encode(sig.to_bytes())
}

/// Export public key as base64 SPKI DER (platform format)
pub fn public_key_to_spki_base64(key: &SigningKey) -> String {
    // Ed25519 SPKI DER prefix (30 2a 30 05 06 03 2b 65 70 03 21 00) + 32 bytes raw key
    let vk = key.verifying_key();
    let raw = vk.as_bytes();
    let mut spki = vec![
        0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
    ];
    spki.extend_from_slice(raw);
    BASE64.encode(&spki)
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
    fn sign_bytes_is_base64() {
        let key = SigningKey::from_bytes(&[2u8; 32]);
        let sig = sign_bytes(&key, b"hello");
        // base64 decode should succeed and produce 64 bytes (Ed25519 signature)
        let decoded = BASE64.decode(&sig).unwrap();
        assert_eq!(decoded.len(), 64);
    }

    #[test]
    fn sign_bytes_verifiable() {
        let key = SigningKey::from_bytes(&[2u8; 32]);
        let sig_b64 = sign_bytes(&key, b"hello world");
        let sig_bytes = BASE64.decode(&sig_b64).unwrap();
        let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes.try_into().unwrap());
        assert!(key.verifying_key().verify(b"hello world", &sig).is_ok());
    }

    #[test]
    fn sign_bytes_wrong_message_fails() {
        let key = SigningKey::from_bytes(&[3u8; 32]);
        let sig_b64 = sign_bytes(&key, b"correct");
        let sig_bytes = BASE64.decode(&sig_b64).unwrap();
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
        assert_eq!(n.len(), 36);
        assert_eq!(n.chars().filter(|&c| c == '-').count(), 4);
    }

    #[test]
    fn public_key_spki_base64_format() {
        let key = SigningKey::from_bytes(&[42u8; 32]);
        let spki = public_key_to_spki_base64(&key);
        let decoded = BASE64.decode(&spki).unwrap();
        // SPKI DER for Ed25519: 12 byte prefix + 32 byte key = 44 bytes
        assert_eq!(decoded.len(), 44);
        // Prefix should match Ed25519 SPKI OID
        assert_eq!(&decoded[..12], &[0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00]);
        // Last 32 bytes should be the raw public key
        assert_eq!(&decoded[12..], key.verifying_key().as_bytes());
    }
}
