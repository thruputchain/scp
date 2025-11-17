# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com),
and this project adheres to [Semantic Versioning](https://semver.org).

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
