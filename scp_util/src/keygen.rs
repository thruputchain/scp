use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};

/// Derives an Ed25519 secret key deterministically from `seed`: the key's
/// 32-byte seed is `SHA-256(seed)`.
///
/// The same input always gives the same key and nothing random is added, so
/// this is neither random key generation nor a password-based derivation: a
/// short or guessable `seed` gives a guessable key. It is meant for tests and
/// fixtures. For a production key, pass 32 bytes from a cryptographically
/// secure random source, or build the `SecretKey` from an Ed25519 key
/// generated elsewhere (64 bytes: the seed followed by the public key).
pub fn generate_secret_key(seed: &[u8]) -> thruput_xdr::types::SecretKey {
    let mut hasher = Sha256::new();
    hasher.update(seed);
    let hash = hasher.finalize();

    let secret_bytes: &[u8; 32] = &hash[..32].try_into().unwrap();
    let signing_key = SigningKey::from_bytes(secret_bytes);

    let mut secret_key_bytes = [0u8; 64];
    secret_key_bytes[..32].copy_from_slice(&signing_key.to_bytes());
    secret_key_bytes[32..]
        .copy_from_slice(&signing_key.verifying_key().to_bytes());

    thruput_xdr::types::SecretKey::Ed25519 {
        ed25519: thruput_xdr::types::Ed25519SecretKey::from(secret_key_bytes),
    }
}

pub fn derive_public_key(
    secret_key: &thruput_xdr::types::SecretKey,
) -> thruput_xdr::types::PublicKey {
    match secret_key {
        thruput_xdr::types::SecretKey::Ed25519 { ed25519 } => {
            let secret_bytes: &[u8; 32] = &ed25519[..32].try_into().unwrap();
            let signing_key = SigningKey::from_bytes(secret_bytes);
            let public_bytes = signing_key.verifying_key().to_bytes();

            thruput_xdr::types::PublicKey::Ed25519 {
                ed25519: thruput_xdr::types::Ed25519PublicKey::from(
                    public_bytes,
                ),
            }
        }
        _ => panic!("Unsupported secret key type"),
    }
}

/// The raw bytes of a public key, whatever its scheme.
pub fn public_key_bytes(public_key: &thruput_xdr::types::PublicKey) -> &[u8] {
    match public_key {
        thruput_xdr::types::PublicKey::Ed25519 { ed25519 } => &ed25519[..],
        thruput_xdr::types::PublicKey::MlDsa65 { mldsa65 } => &mldsa65[..],
        _ => &[],
    }
}

pub fn derive_node_id(
    sk: &thruput_xdr::types::SecretKey,
) -> thruput_scp::SCPNodeID {
    derive_public_key(sk)
}

/// Parse a SecretKey from a Base64 standard encoded string of the 64-byte secret key
pub fn parse_secret_key_from_base64(
    base64_str: &str,
) -> Result<thruput_xdr::types::SecretKey, String> {
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

    Ok(thruput_xdr::types::SecretKey::Ed25519 {
        ed25519: thruput_xdr::types::Ed25519SecretKey::from(secret_key_bytes),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secret_bytes(sk: &thruput_xdr::types::SecretKey) -> &[u8; 64] {
        match sk {
            thruput_xdr::types::SecretKey::Ed25519 { ed25519 } => ed25519,
            _ => panic!("unexpected secret key variant"),
        }
    }

    fn public_bytes(pk: &thruput_xdr::types::PublicKey) -> &[u8; 32] {
        match pk {
            thruput_xdr::types::PublicKey::Ed25519 { ed25519 } => ed25519,
            _ => panic!("unexpected public key variant"),
        }
    }

    #[test]
    fn public_key_bytes_covers_every_scheme() {
        let pk = derive_public_key(&generate_secret_key(b"seed-bytes"));
        assert_eq!(public_key_bytes(&pk), &public_bytes(&pk)[..]);
        let mldsa65 = thruput_xdr::types::PublicKey::MlDsa65 {
            mldsa65: Box::new([7u8; 1952].into()),
        };
        assert_eq!(public_key_bytes(&mldsa65), &[7u8; 1952][..]);
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
