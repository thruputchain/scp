use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};

pub fn generate_secret_key(seed: &[u8]) -> scp_proto::SCPSecretKey {
    let mut hasher = Sha256::new();
    hasher.update(seed);
    let hash = hasher.finalize();

    let secret_bytes: &[u8; 32] = &hash[..32].try_into().unwrap();
    let signing_key = SigningKey::from_bytes(secret_bytes);

    let mut secret_key_bytes = [0u8; 64];
    secret_key_bytes[..32].copy_from_slice(&signing_key.to_bytes());
    secret_key_bytes[32..].copy_from_slice(&signing_key.verifying_key().to_bytes());

    scp_proto::SCPSecretKey::Ed25519 {
        ed25519: scp_proto::SCPEd25519SecretKey::from(secret_key_bytes),
    }
}

pub fn derive_public_key(secret_key: &scp_proto::SCPSecretKey) -> scp_proto::SCPPublicKey {
    match secret_key {
        scp_proto::SCPSecretKey::Ed25519 { ed25519 } => {
            let secret_bytes: &[u8; 32] = &ed25519[..32].try_into().unwrap();
            let signing_key = SigningKey::from_bytes(secret_bytes);
            let public_bytes = signing_key.verifying_key().to_bytes();

            scp_proto::SCPPublicKey::Ed25519 {
                ed25519: scp_proto::SCPEd25519PublicKey::from(public_bytes),
            }
        }
        _ => panic!("Unsupported secret key type"),
    }
}

pub fn derive_node_id(sk: &scp_proto::SCPSecretKey) -> scp_proto::SCPNodeID {
    derive_public_key(sk)
}

/// Parse a SecretKey from a Base64 standard encoded string of the 64-byte secret key
pub fn parse_secret_key_from_base64(base64_str: &str) -> Result<scp_proto::SCPSecretKey, String> {
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

    Ok(scp_proto::SCPSecretKey::Ed25519 {
        ed25519: scp_proto::SCPEd25519SecretKey::from(secret_key_bytes),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secret_bytes(sk: &scp_proto::SCPSecretKey) -> &[u8; 64] {
        match sk {
            scp_proto::SCPSecretKey::Ed25519 { ed25519 } => ed25519,
            _ => panic!("unexpected secret key variant"),
        }
    }

    fn public_bytes(pk: &scp_proto::SCPPublicKey) -> &[u8; 32] {
        match pk {
            scp_proto::SCPPublicKey::Ed25519 { ed25519 } => ed25519,
            _ => panic!("unexpected public key variant"),
        }
    }

    #[test]
    fn generate_secret_key_is_deterministic() {
        let sk1 = generate_secret_key(b"seed-a");
        let sk2 = generate_secret_key(b"seed-a");
        assert_eq!(secret_bytes(&sk1), secret_bytes(&sk2));
    }

    #[test]
    fn generate_secret_key_differs_for_different_seeds() {
        let sk1 = generate_secret_key(b"seed-a");
        let sk2 = generate_secret_key(b"seed-b");
        assert_ne!(secret_bytes(&sk1), secret_bytes(&sk2));
    }

    #[test]
    fn generate_secret_key_embeds_matching_public_key() {
        let sk = generate_secret_key(b"seed-roundtrip");
        let pk = derive_public_key(&sk);
        let sk_bytes = secret_bytes(&sk);
        let pk_bytes = public_bytes(&pk);
        assert_eq!(&sk_bytes[32..], &pk_bytes[..]);
    }

    #[test]
    fn derive_public_key_is_deterministic() {
        let sk = generate_secret_key(b"seed-derive");
        let pk1 = derive_public_key(&sk);
        let pk2 = derive_public_key(&sk);
        assert_eq!(public_bytes(&pk1), public_bytes(&pk2));
    }

    #[test]
    fn derive_public_key_differs_for_different_secrets() {
        let sk1 = generate_secret_key(b"seed-a");
        let sk2 = generate_secret_key(b"seed-b");
        assert_ne!(
            public_bytes(&derive_public_key(&sk1)),
            public_bytes(&derive_public_key(&sk2)),
        );
    }

    #[test]
    fn derive_node_id_matches_derive_public_key() {
        let sk = generate_secret_key(b"seed-node-id");
        let pk = derive_public_key(&sk);
        let node_id = derive_node_id(&sk);
        assert_eq!(public_bytes(&pk), public_bytes(&node_id));
    }
}
