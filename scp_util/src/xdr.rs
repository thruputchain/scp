use xdr_common::*;
use xdrs::read_xdr::ReadXDR;
use xdrs::serialize::reader;
use xdrs::serialize::writer;
use xdrs::txs;
use xdrs::write_xdr::WriteXDR;

pub fn marshal_xdr<T>(v: &T) -> writer::WriteResult<Vec<u8>>
where
    T: WriteXDR + ?Sized,
{
    txs::encode(v)
}

pub fn unmarshal_xdr<'r, 'a, T>(bytes: &'a [u8]) -> reader::ReadResult<T>
where
    'a: 'r,
    T: ReadXDR<'r>,
{
    txs::decode(bytes)
}
