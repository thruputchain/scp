use sha2::{Digest, Sha256};

pub fn hex_abbrev(bytes: &[u8]) -> String {
    let k = bytes.len().min(3);
    bytes[..k]
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<String>()
}

pub fn hash_bytes(bytes: &[u8]) -> proto::SCPHash {
    Sha256::digest(bytes).into()
}
