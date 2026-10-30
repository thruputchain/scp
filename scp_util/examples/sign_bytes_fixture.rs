use scp_util::{derive_public_key, generate_secret_key, sign_bytes};

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

fn from_hex(input: &str) -> Result<Vec<u8>, String> {
    if !input.len().is_multiple_of(2) {
        return Err("hex input must have even length".to_string());
    }

    let mut bytes = Vec::with_capacity(input.len() / 2);
    let chars: Vec<char> = input.chars().collect();
    for i in (0..chars.len()).step_by(2) {
        let pair = format!("{}{}", chars[i], chars[i + 1]);
        let value = u8::from_str_radix(&pair, 16)
            .map_err(|_| format!("invalid hex at byte pair index {}", i / 2))?;
        bytes.push(value);
    }

    Ok(bytes)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: sign_bytes_fixture <seed> <message_hex>");
        std::process::exit(2);
    }

    let seed = args[1].as_bytes();
    let message = match from_hex(&args[2]) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(2);
        }
    };

    let secret_key = generate_secret_key(seed);
    let public_key = derive_public_key(&secret_key);
    let signature = sign_bytes(&secret_key, &message);

    let public_key_bytes = match &public_key {
        thruput_xdr::types::PublicKey::Ed25519 { ed25519 } => &ed25519[..],
        _ => panic!("unsupported key type"),
    };
    let signature_bytes = match &signature {
        thruput_xdr::types::Signature::Ed25519 { ed25519 } => &ed25519[..],
        _ => panic!("unsupported signature type"),
    };

    println!("PUBLIC_KEY_HEX={}", to_hex(public_key_bytes));
    println!("SIGNATURE_HEX={}", to_hex(signature_bytes));
}
