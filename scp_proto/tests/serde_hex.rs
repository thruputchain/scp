use scp_proto::SCPEd25519Signature as Signature;

#[test]
fn signature_serde_preserves_hex_strings() {
    let signature = Signature::from([0xab; 64]);
    assert_eq!(
        serde_json::to_value(signature).unwrap(),
        serde_json::json!("ab".repeat(64))
    );
    let decoded: Signature =
        serde_json::from_value(serde_json::json!("aB".repeat(64))).unwrap();
    assert_eq!(decoded, Signature::from([0xab; 64]));
}

#[test]
fn signature_serde_rejects_unicode_without_panicking() {
    // The decoded JSON string has the correct 128-byte length, but the
    // first two-byte slice would split the three-byte euro character.
    let invalid = format!("€{}", "0".repeat(125));
    assert_eq!(invalid.len(), 128);
    let json = serde_json::to_string(&invalid).unwrap();
    assert!(serde_json::from_str::<Signature>(&json).is_err());
}
