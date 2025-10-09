use proto::{SCPHash, SCPSlotIndex, SCPValue};
use xdr_common::BytesM;

pub fn hah(bytes: &[u8]) -> String {
    hex_abbrev(&proto::hash_bytes(bytes)[..])
}

pub fn hex_abbrev(bytes: &[u8]) -> String {
    let k = bytes.len().min(3);
    bytes[..k]
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<String>()
}

pub fn hex_full(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<String>()
}

pub fn parse_node_id_from_hex(hex_str: &str) -> Result<proto::SCPNodeID, String> {
    if hex_str.len() != 64 {
        return Err(format!(
            "Expected 64 hex characters for Ed25519 public key, got {}",
            hex_str.len()
        ));
    }

    let mut bytes = [0u8; 32];
    for i in 0..32 {
        let hex_byte = &hex_str[i * 2..i * 2 + 2];
        bytes[i] = u8::from_str_radix(hex_byte, 16)
            .map_err(|_| format!("Invalid hex character in position {}", i * 2))?;
    }

    let ed25519_key = proto::Ed25519PublicKey::from(bytes);
    Ok(proto::PublicKey::Ed25519 {
        ed25519: ed25519_key,
    })
}

pub fn get_hash_of_vec<F>(get_hash_of: F, v: &Vec<BytesM>) -> proto::SCPHash
where
    F: Fn(&[u8]) -> proto::SCPHash,
{
    let mut concatenated = Vec::new();
    for bytes_m in v {
        let hash = get_hash_of(&bytes_m.0);
        concatenated.extend_from_slice(&hash);
    }
    get_hash_of(&concatenated)
}

pub fn hash_helper<F>(
    slot_index: SCPSlotIndex,
    prev: &SCPValue,
    extra: F,
    get_hash_of: impl Fn(&[u8]) -> SCPHash,
) -> u64
where
    F: FnOnce(&mut Vec<BytesM>),
{
    let mut vals = Vec::new();

    vals.push(BytesM(proto::marshal_xdr(&slot_index).unwrap().to_vec()));
    vals.push(BytesM(proto::marshal_xdr(prev).unwrap().to_vec()));

    extra(&mut vals);

    let hash = get_hash_of_vec(get_hash_of, &vals);

    let mut res = 0u64;
    for &byte in hash.iter().take(8) {
        res = (res << 8) | (byte as u64);
    }
    res
}

pub fn dbg_node_ids_to_str(ns: &[proto::SCPNodeID]) -> String {
    let mut s = "[".to_string();
    for (i, n) in ns.iter().enumerate() {
        s.push_str(&format!("{}", to_str_key(n, false)));
        if i + 1 < ns.len() {
            s.push_str(", ");
        }
    }
    s.push_str("]");
    s
}

pub fn to_str_key(pk: &proto::SCPNodeID, _full_key: bool) -> String {
    match pk {
        proto::PublicKey::Ed25519 { ed25519 } => hex_abbrev(&ed25519.into_inner()[..]),
    }
}
