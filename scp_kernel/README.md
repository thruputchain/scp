# SCP state machine

This crate implements a state machine for the SCP protocol.

## Compatibility with Stellar's SCP

The wire format of Stellar's SCP implementation and this one is the same.

However, note that there are some semantic differences:

- __Quorum set normalization function__ The function used here differs from Stellar's in that it guarantees: _If two quorum sets are semantically identical, their normalized analogs will be equal and maximally compressed_. In principle, it is possible to add support for either one normalization function.

- __Hash function__ Stellar defers hashing of multiple opaque values to the application. This implementation defers hashing of a single opaque value to the application, and uses an internal implementation of hashing multiple values based on that.

## Usage

The kernel is driven by the application through two traits:

- `Consensus`: validates and combines the application's values and receives
  progress notifications (`ConsensusNotify`), such as `value_externalized`.
- `Driver`: supplies quorum sets by hash, hashes and signs on behalf of the
  node, emits envelopes to peers and manages the nomination and ballot
  timers.

`StellarKernel::new` takes the local node ID, whether it is a validator, its
quorum set and implementations of both traits. `kernel.proto()` returns the
`StellarProto` the application drives: envelopes from peers go to
`received_envelope`, values to `nominate`, and expired timers come back
through `driver_calling_back` with the `DriverCallback` the driver was given
when the timer was started. `scp_server` in this repository is a complete
driver that exposes the kernel over ONC-RPC.

The `testkernel`, `unitkernel` and `testutil` modules are the harness for the
kernel's own test suite and for downstream tests; they are not part of the
protocol implementation.
