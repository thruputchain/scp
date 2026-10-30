# SCP state machine

This crate implements a state machine for the SCP protocol.

## Compatibility with Stellar's SCP

The wire format of Stellar's SCP implementation and this one is the same.

However, note that there are some semantic differences:

- __Quorum set normalization function__ The function used here differs from Stellar's in that it guarantees: _If two quorum sets are semantically identical, their normalized analogs will be equal and maximally compressed_. In principle, it is possible to add support for either one normalization function.

- __Hash function__ Stellar defers hashing of multiple opaque values to the application. This implementation defers hashing of a single opaque value to the application, and uses an internal implementation of hashing multiple values based on that.
