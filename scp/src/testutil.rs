use crate::*;

pub fn make_envelope(
    secret_key: &proto::SecretKey,
    slot_index: proto::SCPSlotIndex,
    mut statement: proto::SCPStatement,
    public_key: proto::PublicKey,
) -> Result<proto::SCPEnvelope, proto::SigningError> {
    statement.node_id = public_key;
    statement.slot_index = slot_index;
    let signature = proto::sign_xdr(secret_key, &statement)?;
    Ok(proto::SCPEnvelope {
        statement,
        signature,
    })
}

