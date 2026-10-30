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

## Repository layout

| Path | Contents |
|---|---|
| `scp_kernel/` | The consensus kernel. Applications implement its `Consensus` and `Driver` traits and drive a `StellarKernel`. |
| `scp_util/` | XDR marshalling, SHA-256 hashing, Ed25519 signing and key helpers shared by the kernel and `scpd`. |
| `scp_server/` | `scpd`. See [`scp_server/README.md`](scp_server/README.md) for its tracing and heap profiling options. |
| `shfix/` | A test-only proc macro that unfolds CATCH2-style `section!` blocks into separate test functions. Not published. |
| `protocol/` | The XDR definitions of the SCP types and of the `SCP_Control` and `SCP_Handle` RPC programs, for reference. The bindings are generated from the Thruput protocol's copy. |

## Building and testing

The minimum supported Rust version is 1.96.

```sh
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --check
```

`cargo test -p thruput-scp-kernel` runs the kernel's unit tests and a port of
the stellar-core SCP test suite. `cargo test -p thruput-scp-server` also runs
a three-node system test over loopback RPC.

The dependency policy (advisories, licenses, sources) is in `deny.toml` and is
checked with [cargo-deny](https://github.com/EmbarkStudios/cargo-deny):

```sh
cargo deny --all-features check
```

## scpd

```sh
cargo install thruput-scp-server
scpd --help
```

`scpd` serves the control API (`SCP_Control`) and calls back into the
application's handler API (`SCP_Handle`), both over ONC-RPC. The application
nominates values, forwards envelopes received from peers and updates quorum
sets through `SCP_Control`; `scpd` asks the application to validate and
combine values, hands it envelopes to broadcast and reports progress
(prepared, committed, externalized) through `SCP_Handle`. Both interfaces are
defined in `protocol/scp_control.x` and `protocol/scp_handle.x`.

By default `scpd` binds the control API on `127.0.0.1:11055` and connects to
the handler on `127.0.0.1:22022`. The control API has no authentication; keep
it on loopback or on a network only the application can reach.

A node needs its secret key and a configuration file:

- `--secret-key`: the 64-byte Ed25519 key (32-byte seed followed by the
  32-byte public key), base64 standard encoded. The node ID is the public
  key.
- `--config-path` (default `config.json`): the timeouts and the local quorum
  set.
- `--is-validator`: whether this node votes, or only follows consensus.

```json
{
  "max_timeout_ms": 30000,
  "initial_ballot_timeout_ms": 1000,
  "increment_ballot_timeout_ms": 1000,
  "initial_nomination_timeout_ms": 1000,
  "increment_nomination_timeout_ms": 1000,
  "quorum_set": {
    "threshold": 2,
    "validators": [[0, 1, 2, "...32 bytes of an Ed25519 public key..."]],
    "inner_sets": []
  }
}
```

Each validator is a 32-byte Ed25519 public key written as a JSON array of 32
integers; `inner_sets` holds nested quorum sets of the same shape.

Logging is controlled with `RUST_LOG`. Tracing export and heap profiling are
described in [`scp_server/README.md`](scp_server/README.md).

## Compatibility with stellar-core

The wire format matches stellar-core's SCP implementation. Two behaviours
differ; see [`scp_kernel/README.md`](scp_kernel/README.md).

## Releases

All crates share the workspace version in `Cargo.toml` and are released
together under a `v<version>` tag by the `Release package` workflow, which
publishes to crates.io with trusted publishing. Changes are recorded in
[`CHANGELOG.md`](CHANGELOG.md).

## License

Apache-2.0
