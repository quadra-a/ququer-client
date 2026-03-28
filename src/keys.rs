use anyhow::Result;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::fs;

use crate::config::ququer_dir;

#[derive(Serialize, Deserialize)]
pub struct StoredKeys {
    pub public_key: String,
    pub secret_key: String,
    pub agent_id: Option<String>,
}

pub fn keys_path() -> Result<std::path::PathBuf> {
    Ok(ququer_dir()?.join("keys.json"))
}

pub fn generate_keypair() -> SigningKey {
    SigningKey::generate(&mut OsRng)
}

pub fn save_keys(key: &SigningKey, agent_id: Option<&str>) -> Result<()> {
    let stored = StoredKeys {
        public_key: hex::encode(key.verifying_key().as_bytes()),
        secret_key: hex::encode(key.to_bytes()),
        agent_id: agent_id.map(String::from),
    };
    let path = keys_path()?;
    fs::write(&path, serde_json::to_string_pretty(&stored)?)?;
    Ok(())
}

pub fn load_keys() -> Result<(SigningKey, StoredKeys)> {
    let path = keys_path()?;
    let content = fs::read_to_string(&path)
        .map_err(|_| anyhow::anyhow!("no keys found — run `ququer register` first"))?;
    let stored: StoredKeys = serde_json::from_str(&content)?;
    let bytes = hex::decode(&stored.secret_key)?;
    let key_bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid secret key length"))?;
    let key = SigningKey::from_bytes(&key_bytes);
    Ok((key, stored))
}

pub fn load_or_generate() -> Result<(SigningKey, bool)> {
    match load_keys() {
        Ok((key, _)) => Ok((key, false)),
        Err(_) => {
            let key = generate_keypair();
            save_keys(&key, None)?;
            Ok((key, true))
        }
    }
}

pub fn public_key_hex(key: &SigningKey) -> String {
    hex::encode(key.verifying_key().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_key_hex_format() {
        let key = SigningKey::from_bytes(&[42u8; 32]);
        let hex_str = public_key_hex(&key);
        assert_eq!(hex_str.len(), 64);
        assert!(hex_str.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn keypair_save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keys.json");

        let key = SigningKey::from_bytes(&[7u8; 32]);
        let stored = StoredKeys {
            public_key: hex::encode(key.verifying_key().as_bytes()),
            secret_key: hex::encode(key.to_bytes()),
            agent_id: Some("agent-1".to_string()),
        };
        std::fs::write(&path, serde_json::to_string_pretty(&stored).unwrap()).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let loaded: StoredKeys = serde_json::from_str(&content).unwrap();
        let bytes = hex::decode(&loaded.secret_key).unwrap();
        let key_bytes: [u8; 32] = bytes.try_into().unwrap();
        let loaded_key = SigningKey::from_bytes(&key_bytes);

        assert_eq!(public_key_hex(&key), public_key_hex(&loaded_key));
        assert_eq!(loaded.agent_id, Some("agent-1".to_string()));
    }

    #[test]
    fn stored_keys_serde_roundtrip() {
        let stored = StoredKeys {
            public_key: "aabb".to_string(),
            secret_key: "ccdd".to_string(),
            agent_id: None,
        };
        let json = serde_json::to_string(&stored).unwrap();
        let loaded: StoredKeys = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.public_key, "aabb");
        assert!(loaded.agent_id.is_none());
    }
}
