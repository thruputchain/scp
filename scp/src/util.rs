pub fn hex_abbrev(bytes: &[u8]) -> String {
    let k = bytes.len().min(3);
    bytes[..k]
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<String>()
}
