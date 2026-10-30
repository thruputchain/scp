# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com),
and this project adheres to [Semantic Versioning](https://semver.org).

## [2.0.0] - Unreleased

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

#### Added
- `scpd --secret-key-file PATH` reads the base64 secret key from a file, and
  `scpd --secret-key-fd N` from an inherited file descriptor such as a pipe
  from the launcher, so it need not appear in the command line, which every
  process on the host can read. `--secret-key` still works; the three are
  mutually exclusive.

### scp_util

#### Changed
- `verify_bytes`, `verify_xdr` and `verify_envelope` return an error for a
  key or signature in a scheme other than Ed25519, instead of panicking.

### scp_kernel

#### Changed
- `compare_public_key` and node ID formatting accept every key scheme.
  Ed25519 keys order and print as before.

## [1.0.0] - 2025-11-17

### shfix

#### Added
- `section!` macro for writing tests, analog to the C++ CATCH2 test library

### proto

#### Added
- Hashing, signing, verifying signatures for XDR structures

### scp_kernel

#### Added
- An essentially exact translation of Stellar's C++ implementation of C++, with the following exceptions:
  - Quorum set normalization compresses unnormalized quorum sets optimally, and has the property: if `Q1` and `Q2` are semantically equivalent, then `normalized(Q1)` is exactly equal to `normalized(Q2)`.
  - In the driver interface, `get_hash_of` delegates the hashing of a single value to the application. Whereas the Stellar C++ implementation delegates the hashing of a list of values to the application.

### scp_server

#### Added
- `scpd` is a standalone SCP daemon, designed to be used as a sidecar to an application written in another language.
