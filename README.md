# SCP

A Rust implementation of the Stellar Consensus Protocol (SCP), and `scpd`, a
standalone SCP daemon that runs as a sidecar to an application written in
another language.

| Package | Library | Purpose |
|---|---|---|
| `thruput-scp-kernel` | `scp_kernel` | The SCP consensus kernel: nomination, ballot protocol, quorum sets |
| `thruput-scp-util` | `scp_util` | Hashing, signing and verifying XDR-encoded SCP structures |
| `thruput-scp-server` | | `scpd`, which serves the kernel over ONC-RPC |

On crates.io the crates are published with a `thruput-` prefix. Their library
names don't change, so code still uses `scp_kernel::` and `scp_util::`.

The SCP protocol types and RPC bindings come from
[`thruput-scp`](https://crates.io/crates/thruput-scp), generated from the
Thruput protocol in
[thruput-protocol-rs](https://github.com/thruputchain/thruput-protocol-rs).
Node IDs and signatures use `thruput-xdr`'s `PublicKey` and `Signature`; SCP
signs and verifies Ed25519 only, and treats any other scheme as an invalid
signature.

## scpd

```sh
cargo install thruput-scp-server
scpd --help
```

`scpd` serves the control API (`SCP_Control`) and calls back into the
application's handler API (`SCP_Handle`), both over ONC-RPC.

## License

Apache-2.0
