# SCP state machine

This crate implements a state machine for the SCP protocol.

## Relation to stellar-core

The protocol logic follows stellar-core's SCP implementation and the test
suite is a port of stellar-core's, but envelopes are not interchangeable with
stellar-core's: node IDs and signatures are Thruput's `PublicKey` and
`Signature` unions, and signatures are Ed25519ph over the statement's XDR
alone. Two parts of the logic also differ deliberately:

- __Quorum set normalization__ The function used here guarantees: _if two
  quorum sets are semantically identical, their normalized forms are equal
  and maximally compressed_. In principle, it is possible to add support for
  either normalization function.

- __Hash function__ stellar-core defers hashing of a list of opaque values to
  the application. This implementation defers hashing of a single opaque
  value to the application (`Driver::get_hash_of`) and hashes lists
  internally on top of it.

## Usage

The kernel is driven by the application through two traits:

- `Consensus`: validates and combines the application's values and receives
  progress notifications (`ConsensusNotify`), such as `value_externalized`.
- `Driver`: supplies quorum sets by hash, hashes and signs on behalf of the
  node, emits envelopes to peers and manages the nomination and ballot
  timers.

The kernel does not verify envelope signatures: authenticate every envelope
before passing it in (`scp_util::verify_envelope`), or anyone can speak for
any node in the quorum set.

`StellarKernel::new` takes the local node ID, whether it is a validator, its
quorum set and implementations of both traits. `kernel.proto()` returns the
`StellarProto` the application drives: envelopes from peers go to
`received_envelope`, values to `nominate`, and expired timers come back
through `driver_calling_back` with the `DriverCallback` the driver was given
when the timer was started. `scp_server` in this repository is a complete
driver that exposes the kernel over ONC-RPC.
