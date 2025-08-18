use crate::*;
use std::fmt::Debug;
use std::time::Duration;

pub enum Timer {
    Nomination = 0,
    Ballot = 1,
}

pub trait Driver: Debug {
    fn get_qset(&self, h: &proto::SCPHash) -> Option<&proto::SCPQuorumSet>;
    fn to_short_string(&self, pk: &proto::SCPNodeID) -> String;
    fn to_str_key(&self, pk: &proto::SCPNodeID, full_key: bool) -> String;
    fn from_str_key(&self, s: &str) -> Result<proto::SCPNodeID, String>; // XXX: get from KeyUtils.cpp
    fn get_value_string(&self, value: &proto::SCPValue) -> String;
    fn get_hash_of(&self, opaque: &[u8]) -> proto::SCPHash;

    fn get_node_weight(
        &self,
        node_id: &proto::SCPNodeID,
        qset: &proto::SCPQuorumSet,
        is_local_node: bool,
    ) -> u64;
    fn compute_hash_node(
        &self,
        slot_index: proto::SCPSlotIndex,
        prev: &proto::SCPValue,
        is_priority: bool,
        round_number: i32,
        node_id: &proto::SCPNodeID,
    ) -> u64;
    fn compute_value_hash(
        &self,
        slot_index: proto::SCPSlotIndex,
        prev: &proto::SCPValue,
        round_number: i32,
        value: &proto::SCPValue,
    ) -> u64;

    fn emit_envelope(&self, env: proto::SCPEnvelope);
    fn compute_timeout(&self, counter: proto::SCPCount, is_nomination: bool) -> Duration;
    fn start_timer(&self, slot_index: SlotIndex, timer: Timer, timeout: Duration);
    fn stop_timer(&self, slot_index: SlotIndex, timer: Timer);
}

pub trait DriverCb {
    fn received(&mut self, env: &proto::SCPEnvelope);
    fn timer_expired(&mut self, slot_index: SlotIndex, timer: Timer);
}
