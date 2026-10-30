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
defined in the Thruput protocol's `scp_control.x` and `scp_handle.x`, and
their Rust bindings are in [`thruput-scp`](https://docs.rs/thruput-scp).

**The application must authenticate peer envelopes.** Neither `scpd` nor the
kernel verifies envelope signatures: an envelope passed to
`scp_control_received_envelope` or `scp_control_set_state_from_envelope` is
processed as a genuine statement by `statement.node_id`, so forwarding an
unverified one lets its sender speak for any node in the quorum set. Verify
the Ed25519ph signature over the XDR of `statement` against `node_id` first
(`scp_util::verify_envelope` does exactly this in Rust) and drop envelopes
that fail.

By default `scpd` binds the control API on `127.0.0.1:11055` and connects to
the handler on `127.0.0.1:22022`. The control API has no authentication of
its own; keep it on loopback or on a network only the application can reach.

A node needs its secret key and a configuration file:

- `--secret-key`: the 64-byte Ed25519 key (32-byte seed followed by the
  32-byte public key), base64 standard encoded. The node ID is the public
  key.
- `--config-path` (default `config.json`): the timeouts and the local quorum
  set.
- `--is-validator`: whether this node votes, or only follows consensus.

A three-validator network where any two validators form a quorum:

```json
{
  "max_timeout_ms": 30000,
  "initial_ballot_timeout_ms": 1000,
  "increment_ballot_timeout_ms": 1000,
  "initial_nomination_timeout_ms": 1000,
  "increment_nomination_timeout_ms": 1000,
  "quorum_set": {
    "threshold": 2,
    "validators": [
      [211, 191, 176, 60, 94, 168, 170, 40, 132, 54, 59, 244, 214, 142, 189, 94, 56, 5, 155, 3, 184, 161, 81, 155, 14, 15, 90, 187, 98, 126, 59, 210],
      [253, 78, 91, 115, 71, 210, 243, 198, 171, 210, 251, 84, 1, 64, 11, 125, 227, 243, 202, 29, 86, 250, 92, 124, 240, 73, 139, 32, 78, 62, 186, 201],
      [207, 23, 227, 10, 22, 56, 61, 179, 62, 195, 188, 24, 27, 105, 125, 140, 163, 197, 168, 194, 125, 124, 138, 212, 240, 227, 52, 81, 243, 84, 159, 152]
    ],
    "inner_sets": []
  }
}
```

Each validator is a 32-byte Ed25519 public key written as a JSON array of 32
integers; `inner_sets` holds nested quorum sets of the same shape, each of
which counts as one member towards its parent's threshold.

Logging is controlled with `RUST_LOG`. Tracing export and heap profiling are
described in [`scp_server/README.md`](scp_server/README.md).

### Build settings for profiling

A plain `cargo install thruput-scp-server` builds with Cargo's default
release profile. The repository's own builds, including the CI check, add
frame pointers and a GNU build ID through `.cargo/config.toml`, and keep
overflow checks and line tables in release through the workspace
`[profile.release]`. Profilers and the heap profiler rely on the first two.
`cargo install` ignores the repository's configuration and the published
crate carries no profile, so pass them explicitly:

```sh
RUSTFLAGS="-C force-frame-pointers=yes -C link-arg=-Wl,--build-id=sha1" \
cargo install thruput-scp-server \
  --config 'profile.release.overflow-checks=true' \
  --config 'profile.release.debug="line-tables-only"'
```

Add `--features heap-profiling` for the jemalloc heap profiler.

## Relation to stellar-core

The kernel follows stellar-core's SCP implementation (federated voting,
nomination, the ballot protocol and its timeouts), and `scp_kernel`'s test
suite is a port of stellar-core's. It is not wire compatible with
stellar-core nodes:

- Envelopes use Thruput's protocol types. Node IDs are `thruput-xdr`'s
  `PublicKey` union and envelope signatures its `Signature` union, each with
  Ed25519 and ML-DSA-65 arms, where stellar-core has its own `PublicKey` and
  an opaque 64-byte signature.
- Signatures are Ed25519ph (the statement's XDR pre-hashed with SHA-512) over
  the `SCPStatement` alone. stellar-core signs the XDR of the network ID, an
  envelope type tag and the statement with plain Ed25519.
- Only Ed25519 keys sign and verify; an envelope with an ML-DSA-65 key or
  signature fails verification.
- Quorum set normalization and the hashing of value lists differ, as
  described in [`scp_kernel/README.md`](scp_kernel/README.md).

## Releases

All crates share the workspace version in `Cargo.toml` and are released
together under a `v<version>` tag by the `Release package` workflow, which
publishes to crates.io with trusted publishing. Changes are recorded in
[`CHANGELOG.md`](CHANGELOG.md).

## License

Apache-2.0
