//! The Stellar Consensus Protocol (SCP) kernel: federated voting, nomination
//! and the ballot protocol, following stellar-core's implementation, over
//! Thruput's protocol types from [`thruput_scp`].
//!
//! The application supplies two things. A [`Consensus`] validates and
//! combines its values and receives progress notifications through
//! [`ConsensusNotify`]. A [`Driver`] supplies quorum sets by hash, hashes and
//! signs for the node, emits envelopes to peers and runs the timers.
//! [`StellarKernel::new`] takes both, with the node ID, the validator flag
//! and the local quorum set. [`StellarKernel::proto`] returns the
//! [`StellarProto`] through which envelopes from peers arrive
//! ([`DriverCb::received_envelope`]), values are nominated
//! ([`StellarProto::nominate`]) and expired timers are delivered
//! ([`DriverCb::driver_calling_back`]).
//!
//! The kernel does not verify envelope signatures. Envelopes must be
//! authenticated before they reach it; see [`DriverCb`].
//!
//! [`verifier`] checks, outside a running kernel, that a set of
//! `EXTERNALIZE` envelopes proves a slot externalized under a quorum set.
//!
//! Everything in [`scp_util`] is re-exported.

pub use scp_util::*;

mod consensus;
pub use consensus::*;

mod driver;
pub use driver::*;

pub mod stellar;
pub use stellar::*;

pub mod kernel;
pub use kernel::*;

pub mod info;
pub use info::*;

pub mod local;
pub use local::*;

pub mod federated;
pub use federated::*;

pub mod verifier;
pub use verifier::*;

pub mod slot;
pub use slot::*;

pub mod qset;
pub use qset::*;

pub mod ballotutil;
pub use ballotutil::*;

pub mod phase;
pub use phase::*;

pub mod envelope;
pub use envelope::*;

pub mod nomination;
pub use nomination::*;

pub mod ballot;
pub use ballot::*;

pub mod heuristic;
pub use heuristic::*;

pub mod util;
pub use util::*;

// The test harness, for this crate's tests only.
#[cfg(test)]
pub mod unitkernel;
#[cfg(test)]
pub use unitkernel::*;

#[cfg(test)]
pub mod testkernel;
#[cfg(test)]
pub use testkernel::*;

#[cfg(test)]
pub mod testutil;
#[cfg(test)]
pub use testutil::*;

#[cfg(test)]
mod qset_test;

#[cfg(test)]
mod unit_test;

#[cfg(test)]
mod stellar_test;

#[cfg(test)]
mod span_test;
