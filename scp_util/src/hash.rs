use crate as proto;
use sha2::{Digest, Sha256};
use xdrs::serialize::writer;
use xdrs::write_xdr::WriteXDR;

pub fn hash_bytes(bytes: &[u8]) -> proto::SCPHash {
    Sha256::digest(bytes).into()
}

pub fn hash_xdr<T>(v: &T) -> writer::Result<proto::SHA256Hash>
where
    T: WriteXDR + ?Sized,
{
    let bytes = proto::marshal_xdr(v)?;
    let h: [u8; 32] = hash_bytes(bytes.as_slice()).into();
    Ok(proto::SHA256Hash::from(h))
}

pub fn hash_xdr_scp<T>(v: &T) -> writer::Result<proto::SCPHash>
where
    T: WriteXDR + ?Sized,
{
    Ok(hash_xdr(v)?.into_inner())
}
