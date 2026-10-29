use ed25519_dalek::{
    Digest, Sha512, Signature, SignatureError, SigningKey, VerifyingKey,
};
use thiserror::Error;
use xdrs::txs;
use xdrs::write_xdr::WriteXDR;

/// Creates an ed25519_dalek::SigningKey from a thruput_xdr::types::Ed25519SecretKey
fn signing_key_from_secret_key(
    secret_key: &thruput_xdr::types::Ed25519SecretKey,
) -> SigningKey {
    // Ed25519SecretKey contains 64 bytes, but SigningKey expects 32 bytes
    // The first 32 bytes are the secret key, the last 32 bytes are typically the public key
    let secret_bytes: &[u8; 32] = &secret_key[..32].try_into().unwrap();
    SigningKey::from_bytes(secret_bytes)
}

fn verifying_key_from_public_key(
    public_key: &thruput_xdr::types::Ed25519PublicKey,
) -> VerifyingKey {
    VerifyingKey::from_bytes(public_key[..].try_into().unwrap()).unwrap()
}

fn sign_bytes_ed25519(
    secret_key: &thruput_xdr::types::Ed25519SecretKey,
    msg: &[u8],
) -> thruput_xdr::types::Ed25519Signature {
    let prehash = Sha512::new_with_prefix(msg);
    let signature = signing_key_from_secret_key(secret_key)
        .sign_prehashed(prehash, None)
        .expect("sign_prehashed should not fail");
    thruput_xdr::types::Ed25519Signature::from(signature.to_bytes())
}

fn verify_bytes_ed25519(
    public_key: &thruput_xdr::types::Ed25519PublicKey,
    msg: &[u8],
    signature: &thruput_xdr::types::Ed25519Signature,
) -> Result<(), SignatureError> {
    let prehash = Sha512::new_with_prefix(msg);
    verifying_key_from_public_key(public_key).verify_prehashed(
        prehash,
        None,
        &Signature::from_bytes(signature),
    )
}

pub fn verify_bytes(
    public_key: &thruput_xdr::types::PublicKey,
    msg: &[u8],
    signature: &thruput_xdr::types::Signature,
) -> Result<(), SignatureError> {
    match (public_key, signature) {
        (
            thruput_xdr::types::PublicKey::Ed25519 { ed25519: key },
            thruput_xdr::types::Signature::Ed25519 { ed25519: sig },
        ) => verify_bytes_ed25519(key, msg, sig),
        // Only Ed25519 is supported. The key and signature types also decode
        // other schemes, which a peer can send, so those fail verification.
        _ => Err(SignatureError::new()),
    }
}

pub fn sign_bytes(
    secret_key: &thruput_xdr::types::SecretKey,
    msg: &[u8],
) -> thruput_xdr::types::Signature {
    let ed25519_key = match secret_key {
        thruput_xdr::types::SecretKey::Ed25519 { ed25519 } => Some(ed25519),
        _ => panic!("Unsupported secret key type"),
    }
    .unwrap();

    thruput_xdr::types::Signature::Ed25519 {
        ed25519: sign_bytes_ed25519(ed25519_key, msg),
    }
}

#[derive(Copy, Clone, Debug, Error)]
pub enum SigningError {
    #[error("encoding error")]
    EncodingError(xdrs::serialize::writer::WriteError),
}

#[derive(Debug, Error)]
pub enum VerifyingError {
    #[error("encoding error")]
    EncodingError(xdrs::serialize::writer::WriteError),

    #[error("verifying error")]
    VerifyingError(SignatureError),
}

pub fn sign_xdr<T>(
    secret_key: &thruput_xdr::types::SecretKey,
    v: &T,
) -> Result<thruput_xdr::types::Signature, SigningError>
where
    T: WriteXDR + ?Sized,
{
    let x = txs::encode(v).map_err(SigningError::EncodingError)?;
    Ok(sign_bytes(secret_key, x.as_slice()))
}

pub fn verify_xdr<T>(
    public_key: &thruput_xdr::types::PublicKey,
    v: &T,
    signature: &thruput_xdr::types::Signature,
) -> Result<(), VerifyingError>
where
    T: WriteXDR + ?Sized,
{
    let x = txs::encode(v).map_err(VerifyingError::EncodingError)?;
    verify_bytes(public_key, x.as_slice(), signature)
        .map_err(VerifyingError::VerifyingError)
}

/// Verifies that `envelope.signature` is a valid signature over the XDR
/// encoding of `envelope.statement` produced by the secret key matching
/// `envelope.statement.node_id`.
pub fn verify_envelope(
    envelope: &thruput_scp::SCPEnvelope,
) -> Result<(), VerifyingError> {
    verify_xdr(
        &envelope.statement.node_id,
        &envelope.statement,
        &envelope.signature,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keygen::{
        derive_node_id, derive_public_key, generate_secret_key,
    };
    use xdr_common::{BytesM, VecM};

    fn make_statement(
        node_id: thruput_scp::SCPNodeID,
    ) -> thruput_scp::SCPStatement {
        thruput_scp::SCPStatement {
            node_id,
            slot_index: 42,
            pledges: thruput_scp::SCPStatement_pledges::SCP_ST_NOMINATE {
                nominate: thruput_scp::SCPStatementNominate {
                    quorum_set_hash: [7u8; 32],
                    votes: VecM(vec![BytesM(b"vote-1".to_vec())]),
                    accepted: VecM(vec![]),
                },
            },
        }
    }

    #[test]
    fn sign_bytes_verify_bytes_roundtrip() {
        let sk = generate_secret_key(b"sign-roundtrip");
        let pk = derive_public_key(&sk);
        let msg = b"the quick brown fox jumps over the lazy dog";
        let sig = sign_bytes(&sk, msg);
        verify_bytes(&pk, msg, &sig).expect("signature should verify");
    }

    #[test]
    fn sign_bytes_is_deterministic_for_ed25519() {
        let sk = generate_secret_key(b"sign-deterministic");
        let msg = b"some payload";
        let sig1 = sign_bytes(&sk, msg);
        let sig2 = sign_bytes(&sk, msg);
        let bytes1 = match &sig1 {
            thruput_xdr::types::Signature::Ed25519 { ed25519 } => ed25519,
            _ => panic!("unexpected signature variant"),
        };
        let bytes2 = match &sig2 {
            thruput_xdr::types::Signature::Ed25519 { ed25519 } => ed25519,
            _ => panic!("unexpected signature variant"),
        };
        assert_eq!(&bytes1[..], &bytes2[..]);
    }

    #[test]
    fn verify_bytes_rejects_modified_message() {
        let sk = generate_secret_key(b"sign-tamper-msg");
        let pk = derive_public_key(&sk);
        let sig = sign_bytes(&sk, b"original message");
        assert!(verify_bytes(&pk, b"tampered message", &sig).is_err());
    }

    #[test]
    fn verify_bytes_rejects_wrong_public_key() {
        let sk1 = generate_secret_key(b"sign-key-1");
        let sk2 = generate_secret_key(b"sign-key-2");
        let pk2 = derive_public_key(&sk2);
        let msg = b"hello";
        let sig = sign_bytes(&sk1, msg);
        assert!(verify_bytes(&pk2, msg, &sig).is_err());
    }

    #[test]
    fn verify_bytes_rejects_unsupported_schemes() {
        let sk = generate_secret_key(b"sign-unsupported");
        let pk = derive_public_key(&sk);
        let msg = b"hello";
        let sig = sign_bytes(&sk, msg);
        let mldsa_pk = thruput_xdr::types::PublicKey::MlDsa65 {
            mldsa65: Box::new([0u8; 1952].into()),
        };
        let mldsa_sig = thruput_xdr::types::Signature::MlDsa65 {
            mldsa65: Box::new([0u8; 3309].into()),
        };
        assert!(verify_bytes(&mldsa_pk, msg, &sig).is_err());
        assert!(verify_bytes(&pk, msg, &mldsa_sig).is_err());
        assert!(verify_bytes(&mldsa_pk, msg, &mldsa_sig).is_err());
    }

    #[test]
    fn verify_envelope_roundtrip() {
        let sk = generate_secret_key(b"envelope-roundtrip");
        let node_id = derive_node_id(&sk);
        let statement = make_statement(node_id);
        let signature =
            sign_xdr(&sk, &statement).expect("sign_xdr should succeed");
        let envelope = thruput_scp::SCPEnvelope {
            statement,
            signature,
        };
        verify_envelope(&envelope).expect("envelope should verify");
    }

    #[test]
    fn verify_envelope_rejects_wrong_signer() {
        let signer_sk = generate_secret_key(b"envelope-signer");
        let claimed_sk = generate_secret_key(b"envelope-claimed");
        let claimed_node_id = derive_node_id(&claimed_sk);
        let statement = make_statement(claimed_node_id);
        let signature =
            sign_xdr(&signer_sk, &statement).expect("sign_xdr should succeed");
        let envelope = thruput_scp::SCPEnvelope {
            statement,
            signature,
        };
        assert!(verify_envelope(&envelope).is_err());
    }

    #[test]
    fn verify_envelope_rejects_tampered_statement() {
        let sk = generate_secret_key(b"envelope-tamper");
        let node_id = derive_node_id(&sk);
        let statement = make_statement(node_id);
        let signature =
            sign_xdr(&sk, &statement).expect("sign_xdr should succeed");
        let mut envelope = thruput_scp::SCPEnvelope {
            statement,
            signature,
        };
        envelope.statement.slot_index += 1;
        assert!(verify_envelope(&envelope).is_err());
    }
}
