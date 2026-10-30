use crate::*;
use std::fmt::Debug;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Hash)]
pub enum Timer {
    Nomination = 0,
    Ballot = 1,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum DriverCallback {
    BallotTimerExpired {
        slot_index: SlotIndex,
    },
    NominationTimerExpired {
        slot_index: SlotIndex,
        value: thruput_scp::SCPValue,
        prev_value: thruput_scp::SCPValue,
    },
}

impl std::fmt::Display for DriverCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriverCallback::BallotTimerExpired { slot_index } => {
                write!(f, "Ballot{{ i: {} }}", slot_index)
            }
            DriverCallback::NominationTimerExpired {
                slot_index,
                value,
                prev_value,
            } => {
                write!(
                    f,
                    "Nomination{{ i: {} | X: {} | X': {} }}",
                    slot_index,
                    hah(value),
                    hah(prev_value)
                )
            }
        }
        //
    }
}
#[allow(clippy::wrong_self_convention)]
pub trait Driver: Clone + Debug {
    fn get_qset(
        &self,
        h: &thruput_scp::SCPHash,
    ) -> Option<thruput_scp::SCPQuorumSet>;
    fn to_short_string(&self, pk: &thruput_scp::SCPNodeID) -> String;
    fn to_str_key(&self, pk: &thruput_scp::SCPNodeID, full_key: bool)
    -> String;
    fn from_str_key(&self, s: &str) -> Result<thruput_scp::SCPNodeID, String>; // XXX: get from KeyUtils.cpp
    fn get_value_string(&self, value: &thruput_scp::SCPValue) -> String;
    fn get_hash_of(&self, opaque: &[u8]) -> thruput_scp::SCPHash;

    fn get_node_weight(
        &self,
        node_id: &thruput_scp::SCPNodeID,
        qset: &thruput_scp::SCPQuorumSet,
        is_local_node: bool,
    ) -> u64;
    fn compute_hash_node(
        &self,
        slot_index: thruput_scp::SCPSlotIndex,
        prev: &thruput_scp::SCPValue,
        is_priority: bool,
        round_number: thruput_scp::SCPCount,
        node_id: &thruput_scp::SCPNodeID,
    ) -> u64;
    fn compute_value_hash(
        &self,
        slot_index: thruput_scp::SCPSlotIndex,
        prev: &thruput_scp::SCPValue,
        round_number: thruput_scp::SCPCount,
        value: &thruput_scp::SCPValue,
    ) -> u64;
    fn sign_envelope(&self, env: &mut thruput_scp::SCPEnvelope);

    fn emit_envelope(&self, env: thruput_scp::SCPEnvelope);
    fn compute_timeout(
        &self,
        round_number: thruput_scp::SCPCount,
        is_nomination: bool,
    ) -> Duration;
    fn start_timer(
        &self,
        slot_index: SlotIndex,
        timer: Timer,
        timeout: Duration,
        cb: DriverCallback,
    );
    fn stop_timer(&self, slot_index: SlotIndex, timer: Timer);
}

/// The entry points through which the application drives the protocol.
///
/// The kernel does not verify envelope signatures. Every envelope passed to
/// [`received_envelope`](Self::received_envelope) or
/// [`set_state_from_envelope`](Self::set_state_from_envelope) must already be
/// authenticated by the caller, normally with [`scp_util::verify_envelope`],
/// which checks the signature against the statement's `node_id`:
///
/// ```no_run
/// # fn forward(
/// #     kernel: &mut impl scp_kernel::DriverCb,
/// #     env: &thruput_scp::SCPEnvelope,
/// # ) {
/// if scp_util::verify_envelope(env).is_ok() {
///     kernel.received_envelope(env);
/// }
/// # }
/// ```
///
/// An unverified envelope lets whoever sent it speak for any node in the
/// quorum set.
pub trait DriverCb {
    /// Processes an envelope received from a peer. The envelope must be
    /// authenticated; see the trait documentation.
    fn received_envelope(&mut self, env: &thruput_scp::SCPEnvelope);
    /// Delivers an expired nomination or ballot timer, with the callback the
    /// driver was given in [`Driver::start_timer`].
    fn driver_calling_back(&mut self, cb: DriverCallback);
    /// Restores a slot's state from one of this node's own earlier envelopes,
    /// for example after a restart. The envelope must be authenticated like
    /// any other.
    fn set_state_from_envelope(
        &mut self,
        slot_index: SlotIndex,
        envelope: &thruput_scp::SCPEnvelope,
    );
}

// values used to switch hash function between priority and neighborhood checks
pub const HASH_N: u32 = 1;
pub const HASH_P: u32 = 2;
pub const HASH_K: u32 = 3;
