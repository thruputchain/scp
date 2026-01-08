use xdr_common::*;
use xdrs::read_xdr::ReadXDR;
use xdrs::serialize::reader;
use xdrs::serialize::writer;
use xdrs::serialize_txs;
use xdrs::write_xdr::WriteXDR;

pub fn marshal_xdr<T>(v: &T) -> writer::Result<Vec<u8>>
where
    T: WriteXDR + ?Sized,
{
    serialize_txs::encode(v)
}

pub fn unmarshal_xdr<'r, 'a, T>(bytes: &'a [u8]) -> reader::Result<T>
where
    'a: 'r,
    T: ReadXDR<'r>,
{
    serialize_txs::decode(bytes)
}
