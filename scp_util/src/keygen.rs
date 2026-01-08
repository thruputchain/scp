use crate as proto;
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};

pub fn generate_secret_key(seed: &[u8]) -> proto::SecretKey {
    let mut hasher = Sha256::new();
    hasher.update(seed);
    let hash = hasher.finalize();

    let secret_bytes: &[u8; 32] = &hash[..32].try_into().unwrap();
    let signing_key = SigningKey::from_bytes(secret_bytes);

    let mut secret_key_bytes = [0u8; 64];
    secret_key_bytes[..32].copy_from_slice(&signing_key.to_bytes());
    secret_key_bytes[32..].copy_from_slice(&signing_key.verifying_key().to_bytes());

    proto::SecretKey::Ed25519 {
        ed25519: proto::Ed25519SecretKey::from(secret_key_bytes),
    }
}

pub fn derive_public_key(secret_key: &proto::SecretKey) -> proto::PublicKey {
    match secret_key {
        proto::SecretKey::Ed25519 { ed25519 } => {
            let secret_bytes: &[u8; 32] = &ed25519[..32].try_into().unwrap();
            let signing_key = SigningKey::from_bytes(secret_bytes);
            let public_bytes = signing_key.verifying_key().to_bytes();

            proto::PublicKey::Ed25519 {
                ed25519: proto::Ed25519PublicKey::from(public_bytes),
            }
        }
    }
}

pub fn derive_node_id(sk: &proto::SecretKey) -> proto::SCPNodeID {
    derive_public_key(sk)
}

/// Parse a SecretKey from a Base64 standard encoded string of the 64-byte secret key
pub fn parse_secret_key_from_base64(base64_str: &str) -> Result<proto::SecretKey, String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    if base64_str.is_empty() {
        return Err("empty base64 string".to_string());
    }

    let decoded = STANDARD
        .decode(base64_str)
        .map_err(|e| format!("failed to decode base64: {}", e))?;

    if decoded.len() != 64 {
        return Err(format!("expected 64 bytes, got {}", decoded.len()));
    }

    let secret_key_bytes: [u8; 64] = decoded
        .try_into()
        .map_err(|_| "failed to convert to 64-byte array".to_string())?;

    Ok(proto::SecretKey::Ed25519 {
        ed25519: proto::Ed25519SecretKey::from(secret_key_bytes),
    })
}
