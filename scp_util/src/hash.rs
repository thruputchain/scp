use crate as scp_util;
use sha2::{Digest, Sha256};
use xdrs::serialize::writer;
use xdrs::write_xdr::WriteXDR;

pub fn hash_bytes(bytes: &[u8]) -> scp_proto::SCPHash {
    Sha256::digest(bytes).into()
}

pub fn hash_xdr<T>(v: &T) -> writer::WriteResult<scp_proto::SCPSHA256Hash>
where
    T: WriteXDR + ?Sized,
{
    let bytes = scp_util::marshal_xdr(v)?;
    let h: [u8; 32] = hash_bytes(bytes.as_slice());
    Ok(scp_proto::SCPSHA256Hash::from(h))
}

pub fn hash_xdr_scp<T>(v: &T) -> writer::WriteResult<scp_proto::SCPHash>
where
    T: WriteXDR + ?Sized,
{
    Ok(hash_xdr(v)?.into_inner())
}
