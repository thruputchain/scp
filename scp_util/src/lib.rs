//! Helpers shared by the SCP kernel and `scpd`: XDR marshalling
//! ([`marshal_xdr`], [`unmarshal_xdr`]), SHA-256 hashing ([`hash_bytes`],
//! [`hash_xdr`]), Ed25519 signing and verification of XDR structures and SCP
//! envelopes ([`sign_xdr`], [`verify_xdr`], [`verify_envelope`]) and key
//! helpers ([`generate_secret_key`], [`derive_public_key`],
//! [`parse_secret_key_from_base64`]).
//!
//! Keys and signatures are `thruput-xdr`'s
//! [`PublicKey`](thruput_xdr::types::PublicKey) and
//! [`Signature`](thruput_xdr::types::Signature) unions. Only their Ed25519
//! arms are supported: signing with another scheme panics, and verifying a
//! key or signature of another scheme fails like any bad signature.
//! Signatures are Ed25519ph (the message pre-hashed with SHA-512) over the
//! XDR encoding of the signed value.

pub mod xdr;
pub use xdr::*;

pub mod hash;
pub use hash::*;

pub mod sign;
pub use sign::*;

pub mod keygen;
pub use keygen::*;

#[cfg(test)]
mod tests {
    use xdr_common::*;
    use xdrs::txs;

    #[test]
    fn roundtrip() {
        let v: thruput_xdr::types::uint128_xdr =
            thruput_xdr::types::uint128_xdr {
                highbits: 1,
                lowbits: 2,
            };
        let bytes = txs::encode(&v).expect("encode failed");
        let w: thruput_xdr::types::uint128_xdr =
            txs::decode(&bytes).expect("decode failed");
        assert!(v == w, "unequal");
    }

    use std::mem::MaybeUninit;

    fn as_maybe_uninit(slice: &mut [u8]) -> &mut [MaybeUninit<u8>] {
        // SAFETY: MaybeUninit<u8> has the same layout as u8
        unsafe {
            std::slice::from_raw_parts_mut(
                slice.as_mut_ptr() as *mut MaybeUninit<u8>,
                slice.len(),
            )
        }
    }

    #[test]
    fn roundtrip_into() {
        let v: thruput_xdr::types::uint128_xdr =
            thruput_xdr::types::uint128_xdr {
                highbits: 1,
                lowbits: 2,
            };
        let mut buf = BytesM::<100>(vec![0; 20]);
        let n = txs::encode_into(&v, as_maybe_uninit(buf.0.as_mut_slice()))
            .expect("encode failed");
        assert!(n == 16, "size");
        let w: thruput_xdr::types::uint128_xdr =
            txs::decode(&buf.as_slice()[..16]).expect("decode failed");
        assert!(v == w, "unequal");
    }
}
