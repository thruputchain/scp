# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com),
and this project adheres to [Semantic Versioning](https://semver.org).

## [2.0.0] - 2026-10-30

### All crates

#### Changed
- Published to crates.io as `thruput-scp-util`, `thruput-scp-kernel` and
  `thruput-scp-server`. The library names (`scp_util`, `scp_kernel`) are
  unchanged.
- Built on `thruput-xdrs`/`thruput-xdr-common` 0.17 and
  `thruput-rpc-driver` 0.17 from crates.io.

#### Removed
- `scp_proto`. The protocol types and RPC bindings now come from
  `thruput-scp`, and the key types from `thruput-xdr`: `SCPPublicKey`,
  `SCPSecretKey` and `SCPSignature` are now `PublicKey`, `SecretKey` and
  `Signature`, which also carry an ML-DSA-65 arm.
- `shfix` is no longer published; it is a test-only dependency.

### scp_server

#### Changed
- `scpd` takes its secret key only through `--secret-key-fd N`, an
  inherited file descriptor read to EOF: a launcher passes a pipe that
  nothing else can read, and a shell can pass a file with `3<key`.
  `--secret-key` is gone, because a command-line argument is readable by
  every process on the host.

### scp_util

#### Added
- `verify_envelope` checks an envelope's signature against its statement's
  node ID.

#### Changed
- `verify_bytes`, `verify_xdr` and `verify_envelope` return an error for a
  key or signature in a scheme other than Ed25519, and for an Ed25519 key
  whose bytes are not a valid curve point, instead of panicking.
- Ed25519 signing and verification pre-hash the message with SHA-512, for
  compatibility with the application side.

### scp_kernel

#### Added
- `verifier` module: verifies that a set of `EXTERNALIZE` envelopes proves a
  slot externalized under a quorum set, with bounds on proof size, and
  snapshots the proof as an `ExternalizedSlotProof`.

#### Changed
- `compare_public_key` and node ID formatting accept every key scheme.
  Ed25519 keys order and print as before.
- No longer depends on `tokio`, `tokio-util` or `async-trait`, which it did
  not use.
- The test harness (`TestKernel`, `UnitTestKernel`, `testutil`) is no longer
  part of the public API; it is compiled only for the crate's own tests.

### scp_server

#### Added
- `scpd` exports its `tracing` spans over OTLP/gRPC when
  `OTEL_EXPORTER_OTLP_ENDPOINT` is set; see `scp_server/README.md` for the
  `OTEL_*` variables and `THRUPUT_NETWORK_HASH`.
- Opt-in jemalloc heap profiles on Linux, behind the `heap-profiling`
  feature, `THRUPUT_HEAP_PROFILING=1` and `SCPD_PPROF_LISTEN`. The profile
  listener accepts loopback addresses only unless `SCPD_PPROF_ALLOW_REMOTE=1`.
- `--reuse` (default on) sets `SO_REUSEADDR`/`SO_REUSEPORT` on the control
  listener.
- `SCP_Control.scp_control_set_state_from_envelope`.

#### Changed
- Control RPC messages are capped at 10 MiB.
- Nominations, timer callbacks and `set_state_from_envelope` for slots
  strictly below the lowest non-externalized slot are ignored.
- Release builds keep overflow checks and line tables.
- Logs print values as `hov:<sha256 prefix>` instead of raw bytes.
- `HandleKernel`'s `Debug` output no longer includes the secret key.
- The `GROUNDHOG_HEAP_PROFILING` and `GROUNDHOG_NETWORK_HASH` variables and
  the `groundhog.block.height` span attribute are renamed to
  `THRUPUT_HEAP_PROFILING`, `THRUPUT_NETWORK_HASH` and
  `thruput.block.height`. The application launching scpd must set the new
  names; the old ones are not read.

## [1.0.1] to [1.2.2] - 2026-03-11 to 2026-04-20

`SCP_Control.scp_control_set_lowest_non_externalized_slot`, `tracing`
instrumentation of `scpd`, and dependency bumps (`rpc_driver`, Dependabot).

## [1.0.0] - 2025-11-17

### shfix

#### Added
- `section!` macro for writing tests, analog to the C++ CATCH2 test library

### proto

#### Added
- Hashing, signing, verifying signatures for XDR structures

### scp_kernel

#### Added
- An essentially exact translation of stellar-core's C++ SCP implementation, with the following exceptions:
  - Quorum set normalization compresses unnormalized quorum sets optimally, and has the property: if `Q1` and `Q2` are semantically equivalent, then `normalized(Q1)` is exactly equal to `normalized(Q2)`.
  - In the driver interface, `get_hash_of` delegates the hashing of a single value to the application. Whereas the Stellar C++ implementation delegates the hashing of a list of values to the application.

### scp_server

#### Added
- `scpd` is a standalone SCP daemon, designed to be used as a sidecar to an application written in another language.
