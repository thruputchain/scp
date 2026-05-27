use ed25519_dalek::{
    Digest, Sha512, Signature, SignatureError, SigningKey, VerifyingKey,
};
use thiserror::Error;
use xdrs::txs;
use xdrs::write_xdr::WriteXDR;

/// Creates an ed25519_dalek::SigningKey from a scp_proto::SCPEd25519SecretKey
fn signing_key_from_secret_key(
    secret_key: &scp_proto::SCPEd25519SecretKey,
) -> SigningKey {
    // Ed25519SecretKey contains 64 bytes, but SigningKey expects 32 bytes
    // The first 32 bytes are the secret key, the last 32 bytes are typically the public key
    let secret_bytes: &[u8; 32] = &secret_key[..32].try_into().unwrap();
    SigningKey::from_bytes(secret_bytes)
}

fn verifying_key_from_public_key(
    public_key: &scp_proto::SCPEd25519PublicKey,
) -> VerifyingKey {
    VerifyingKey::from_bytes(public_key[..].try_into().unwrap()).unwrap()
}

fn sign_bytes_ed25519(
    secret_key: &scp_proto::SCPEd25519SecretKey,
    msg: &[u8],
) -> scp_proto::SCPEd25519Signature {
    let prehash = Sha512::new_with_prefix(msg);
    let signature = signing_key_from_secret_key(secret_key)
        .sign_prehashed(prehash, None)
        .expect("sign_prehashed should not fail");
    scp_proto::SCPEd25519Signature::from(signature.to_bytes())
}

fn verify_bytes_ed25519(
    public_key: &scp_proto::SCPEd25519PublicKey,
    msg: &[u8],
    signature: &scp_proto::SCPEd25519Signature,
) -> Result<(), SignatureError> {
    let prehash = Sha512::new_with_prefix(msg);
    verifying_key_from_public_key(public_key).verify_prehashed(
        prehash,
        None,
        &Signature::from_bytes(signature),
    )
}

pub fn verify_bytes(
    public_key: &scp_proto::SCPPublicKey,
    msg: &[u8],
    signature: &scp_proto::SCPSignature,
) -> Result<(), SignatureError> {
    let ed25519_key = match public_key {
        scp_proto::SCPPublicKey::Ed25519 { ed25519 } => Some(ed25519),
        _ => panic!("Unsupported public key type"),
    }
    .unwrap();

    let ed25519_signature = match signature {
        scp_proto::SCPSignature::Ed25519 { ed25519 } => Some(ed25519),
        _ => panic!("Unsupported signature type"),
    }
    .unwrap();

    verify_bytes_ed25519(ed25519_key, msg, ed25519_signature)
}

pub fn sign_bytes(
    secret_key: &scp_proto::SCPSecretKey,
    msg: &[u8],
) -> scp_proto::SCPSignature {
    let ed25519_key = match secret_key {
        scp_proto::SCPSecretKey::Ed25519 { ed25519 } => Some(ed25519),
        _ => panic!("Unsupported secret key type"),
    }
    .unwrap();

    scp_proto::SCPSignature::Ed25519 {
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
    secret_key: &scp_proto::SCPSecretKey,
    v: &T,
) -> Result<scp_proto::SCPSignature, SigningError>
where
    T: WriteXDR + ?Sized,
{
    let x = txs::encode(v).map_err(SigningError::EncodingError)?;
    Ok(sign_bytes(secret_key, x.as_slice()))
}

pub fn verify_xdr<T>(
    public_key: &scp_proto::SCPPublicKey,
    v: &T,
    signature: &scp_proto::SCPSignature,
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
    envelope: &scp_proto::SCPEnvelope,
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
        node_id: scp_proto::SCPNodeID,
    ) -> scp_proto::SCPStatement {
        scp_proto::SCPStatement {
            node_id,
            slot_index: 42,
            pledges: scp_proto::SCPStatement_pledges::SCP_ST_NOMINATE {
                nominate: scp_proto::SCPStatementNominate {
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
            scp_proto::SCPSignature::Ed25519 { ed25519 } => ed25519,
            _ => panic!("unexpected signature variant"),
        };
        let bytes2 = match &sig2 {
            scp_proto::SCPSignature::Ed25519 { ed25519 } => ed25519,
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
    fn verify_envelope_roundtrip() {
        let sk = generate_secret_key(b"envelope-roundtrip");
        let node_id = derive_node_id(&sk);
        let statement = make_statement(node_id);
        let signature =
            sign_xdr(&sk, &statement).expect("sign_xdr should succeed");
        let envelope = scp_proto::SCPEnvelope {
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
        let envelope = scp_proto::SCPEnvelope {
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
        let mut envelope = scp_proto::SCPEnvelope {
            statement,
            signature,
        };
        envelope.statement.slot_index += 1;
        assert!(verify_envelope(&envelope).is_err());
    }
}
