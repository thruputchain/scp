# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust workspace implementing the Stellar Consensus Protocol (SCP) with two main components:

- **proto/**: Protocol definitions and code generation from XDR files
- **scp/**: Main SCP implementation (currently minimal)

The project uses XDR (External Data Representation) files to define protocol structures, which are compiled to Rust code using the `x23c` tool.

## Build Commands

```bash
# Build the entire workspace
cargo build

# Build specific crate
cargo build -p proto
cargo build -p scp

# Run the scp binary
cargo run -p scp

# Run tests
cargo test

# Check code without building
cargo check

# Format code
cargo fmt

# Run clippy linter
cargo clippy
```

## Code Generation

The `proto` crate uses a build script that generates Rust code from XDR files:

- XDR source files are in `proto/x/` (types.x, scp.x)
- Generated Rust files are placed in `proto/src/` (types.rs, scp.rs)
- Code generation happens automatically during build via `proto/build.rs`
- Requires `x23c` tool to be available in PATH

To regenerate protocol code after modifying XDR files:
```bash
cargo build -p proto
```

## Architecture

### Protocol Layer (`proto/`)
- Contains XDR definitions for SCP protocol structures
- Uses external dependencies for XDR handling (`xdr_common`, `xdrs`)
- Exports generated types and SCP-specific structures
- Build script automatically converts `.x` files to `.rs` files

### Application Layer (`scp/`)
- Main binary crate for SCP implementation
- Currently minimal (just prints "Hello, world!")
- Depends on the proto crate for protocol definitions

## Dependencies

The project uses private Git dependencies:
- `xdr_common`: XDR utilities from groundhogchain
- `xdrs`: XDR serialization library with specific features enabled

## Development Notes

- XDR files follow the Stellar protocol specification
- Generated code is marked with `#[rustfmt::skip]` to prevent formatting
- Protocol documentation references are in `proto/doc/NOTES.md`
- The workspace uses Rust edition 2024