use ed25519_dalek::{Signature, SignatureError, Signer, SigningKey, Verifier, VerifyingKey};
use thiserror::Error;
use xdrs::serialize_txs;
use xdrs::write_xdr::WriteXDR;

/// Creates an ed25519_dalek::SigningKey from a scp_proto::Ed25519SecretKey
fn signing_key_from_secret_key(secret_key: &scp_proto::Ed25519SecretKey) -> SigningKey {
    // Ed25519SecretKey contains 64 bytes, but SigningKey expects 32 bytes
    // The first 32 bytes are the secret key, the last 32 bytes are typically the public key
    let secret_bytes: &[u8; 32] = &secret_key[..32].try_into().unwrap();
    SigningKey::from_bytes(&secret_bytes)
}

fn verifying_key_from_public_key(public_key: &scp_proto::Ed25519PublicKey) -> VerifyingKey {
    VerifyingKey::from_bytes(public_key[..].try_into().unwrap()).unwrap()
}

fn sign_bytes_ed25519(
    secret_key: &scp_proto::Ed25519SecretKey,
    msg: &[u8],
) -> scp_proto::Ed25519Signature {
    let signature = signing_key_from_secret_key(secret_key).sign(msg);
    scp_proto::Ed25519Signature::from(signature.to_bytes())
}

fn verify_bytes_ed25519(
    public_key: &scp_proto::Ed25519PublicKey,
    msg: &[u8],
    signature: &scp_proto::Ed25519Signature,
) -> Result<(), SignatureError> {
    verifying_key_from_public_key(public_key).verify(msg, &Signature::from_bytes(signature))
}

pub fn verify_bytes(
    public_key: &scp_proto::PublicKey,
    msg: &[u8],
    signature: &scp_proto::Signature,
) -> Result<(), SignatureError> {
    let ed25519_key = match public_key {
        scp_proto::PublicKey::Ed25519 { ed25519 } => Some(ed25519),
    }
    .unwrap();

    let ed25519_signature = match signature {
        scp_proto::Signature::Ed25519 { ed25519 } => Some(ed25519),
    }
    .unwrap();

    verify_bytes_ed25519(ed25519_key, msg, ed25519_signature)
}

pub fn sign_bytes(secret_key: &scp_proto::SecretKey, msg: &[u8]) -> scp_proto::Signature {
    let ed25519_key = match secret_key {
        scp_proto::SecretKey::Ed25519 { ed25519 } => Some(ed25519),
    }
    .unwrap();

    scp_proto::Signature::Ed25519 {
        ed25519: sign_bytes_ed25519(ed25519_key, msg),
    }
}

#[derive(Copy, Clone, Debug, Error)]
pub enum SigningError {
    #[error("encoding error")]
    EncodingError(xdrs::serialize::writer::Error),
}

#[derive(Debug, Error)]
pub enum VerifyingError {
    #[error("encoding error")]
    EncodingError(xdrs::serialize::writer::Error),

    #[error("verifying error")]
    VerifyingError(SignatureError),
}

pub fn sign_xdr<T>(
    secret_key: &scp_proto::SecretKey,
    v: &T,
) -> Result<scp_proto::Signature, SigningError>
where
    T: WriteXDR + ?Sized,
{
    let x = serialize_txs::encode(v).map_err(|e| SigningError::EncodingError(e))?;
    Ok(sign_bytes(secret_key, x.as_slice()))
}

pub fn verify_xdr<T>(
    public_key: &scp_proto::PublicKey,
    v: &T,
    signature: &scp_proto::Signature,
) -> Result<(), VerifyingError>
where
    T: WriteXDR + ?Sized,
{
    let x = serialize_txs::encode(v).map_err(|e| VerifyingError::EncodingError(e))?;
    verify_bytes(public_key, x.as_slice(), signature).map_err(|e| VerifyingError::VerifyingError(e))
}
