use crate as proto;
use sha2::{Digest, Sha256};
use xdrs::serialize::writer;
use xdrs::write_xdr::WriteXDR;

pub fn hash_xdr<T>(v: &T) -> writer::Result<proto::SHA256Hash>
where
    T: WriteXDR + ?Sized,
{
    let bytes = proto::marshal_xdr(v)?;
    let h: [u8; 32] = Sha256::digest(bytes.as_slice()).into();
    Ok(proto::SHA256Hash::from(h))
}
