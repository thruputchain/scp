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
        value: scp_proto::SCPValue,
        prev_value: scp_proto::SCPValue,
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
    fn get_qset(&self, h: &scp_proto::SCPHash) -> Option<scp_proto::SCPQuorumSet>;
    fn to_short_string(&self, pk: &scp_proto::SCPNodeID) -> String;
    fn to_str_key(&self, pk: &scp_proto::SCPNodeID, full_key: bool) -> String;
    fn from_str_key(&self, s: &str) -> Result<scp_proto::SCPNodeID, String>; // XXX: get from KeyUtils.cpp
    fn get_value_string(&self, value: &scp_proto::SCPValue) -> String;
    fn get_hash_of(&self, opaque: &[u8]) -> scp_proto::SCPHash;

    fn get_node_weight(
        &self,
        node_id: &scp_proto::SCPNodeID,
        qset: &scp_proto::SCPQuorumSet,
        is_local_node: bool,
    ) -> u64;
    fn compute_hash_node(
        &self,
        slot_index: scp_proto::SCPSlotIndex,
        prev: &scp_proto::SCPValue,
        is_priority: bool,
        round_number: scp_proto::SCPCount,
        node_id: &scp_proto::SCPNodeID,
    ) -> u64;
    fn compute_value_hash(
        &self,
        slot_index: scp_proto::SCPSlotIndex,
        prev: &scp_proto::SCPValue,
        round_number: scp_proto::SCPCount,
        value: &scp_proto::SCPValue,
    ) -> u64;
    fn sign_envelope(&self, env: &mut scp_proto::SCPEnvelope);

    fn emit_envelope(&self, env: scp_proto::SCPEnvelope);
    fn compute_timeout(&self, round_number: scp_proto::SCPCount, is_nomination: bool) -> Duration;
    fn start_timer(
        &self,
        slot_index: SlotIndex,
        timer: Timer,
        timeout: Duration,
        cb: DriverCallback,
    );
    fn stop_timer(&self, slot_index: SlotIndex, timer: Timer);
}

pub trait DriverCb {
    fn received_envelope(&mut self, env: &scp_proto::SCPEnvelope);
    fn driver_calling_back(&mut self, cb: DriverCallback);
    fn set_state_from_envelope(&mut self, slot_index: SlotIndex, envelope: &scp_proto::SCPEnvelope);
}

// values used to switch hash function between priority and neighborhood checks
pub const HASH_N: u32 = 1;
pub const HASH_P: u32 = 2;
pub const HASH_K: u32 = 3;
