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
    use xdrs::serialize_txs;

    #[test]
    fn roundtrip() {
        let v: scp_proto::uint128_xdr = scp_proto::uint128_xdr {
            highbits: 1,
            lowbits: 2,
        };
        let bytes = serialize_txs::encode(&v).expect("encode failed");
        let w: scp_proto::uint128_xdr = serialize_txs::decode(&bytes).expect("decode failed");
        assert!(v == w, "unequal");
    }

    use std::mem::MaybeUninit;

    fn as_maybe_uninit(slice: &mut [u8]) -> &mut [MaybeUninit<u8>] {
        // SAFETY: MaybeUninit<u8> has the same layout as u8
        unsafe {
            std::slice::from_raw_parts_mut(slice.as_mut_ptr() as *mut MaybeUninit<u8>, slice.len())
        }
    }

    #[test]
    fn roundtrip_into() {
        let v: scp_proto::uint128_xdr = scp_proto::uint128_xdr {
            highbits: 1,
            lowbits: 2,
        };
        let mut buf = BytesM::<100>(vec![0; 20]);
        let n = serialize_txs::encode_into(&v, as_maybe_uninit(buf.0.as_mut_slice()))
            .expect("encode failed");
        assert!(n == 16, "size");
        let w: scp_proto::uint128_xdr =
            serialize_txs::decode(&buf.as_slice()[..16]).expect("decode failed");
        assert!(v == w, "unequal");
    }
}
