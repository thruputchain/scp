use crate as proto;
use ed25519_dalek::{Signature, SignatureError, Signer, SigningKey, Verifier, VerifyingKey};

/// Creates an ed25519_dalek::SigningKey from a proto::Ed25519SecretKey
fn signing_key_from_secret_key(secret_key: &proto::Ed25519SecretKey) -> SigningKey {
    // Ed25519SecretKey contains 64 bytes, but SigningKey expects 32 bytes
    // The first 32 bytes are the secret key, the last 32 bytes are typically the public key
    let secret_bytes: &[u8; 32] = &secret_key[..32].try_into().unwrap();
    SigningKey::from_bytes(&secret_bytes)
}

fn verifying_key_from_public_key(public_key: &proto::Ed25519PublicKey) -> VerifyingKey {
    VerifyingKey::from_bytes(public_key[..].try_into().unwrap()).unwrap()
}

fn sign_bytes_ed25519(secret_key: &proto::Ed25519SecretKey, msg: &[u8]) -> proto::Ed25519Signature {
    let signature = signing_key_from_secret_key(secret_key).sign(msg);
    proto::Ed25519Signature::from(signature.to_bytes())
}

fn verify_bytes_ed25519(
    public_key: &proto::Ed25519PublicKey,
    msg: &[u8],
    signature: &proto::Ed25519Signature,
) -> Result<(), SignatureError> {
    verifying_key_from_public_key(public_key).verify(msg, &Signature::from_bytes(signature))
}

pub fn verify_bytes(
    public_key: &proto::PublicKey,
    msg: &[u8],
    signature: &proto::Signature,
) -> Result<(), SignatureError> {
    let ed25519_key = match public_key {
        proto::PublicKey::Ed25519 { ed25519 } => Some(ed25519),
    }
    .unwrap();

    let ed25519_signature = match signature {
        proto::Signature::Ed25519 { ed25519 } => Some(ed25519),
    }
    .unwrap();

    verify_bytes_ed25519(ed25519_key, msg, ed25519_signature)
}

pub fn sign_bytes(secret_key: &proto::SecretKey, msg: &[u8]) -> proto::Signature {
    let ed25519_key = match secret_key {
        proto::SecretKey::Ed25519 { ed25519 } => Some(ed25519),
    }
    .unwrap();

    proto::Signature::Ed25519 {
        ed25519: sign_bytes_ed25519(ed25519_key, msg),
    }
}
