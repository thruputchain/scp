use crate::*;
use std::time::Duration;

pub enum Timer {
    Nomination = 0,
    Ballot = 1,
}

pub trait Driver {
    /// Retrieves a quorum set from its hash
    ///
    /// All SCP statement (see `SCPNomination` and `SCPStatement`) include
    /// a quorum set hash.
    /// SCP does not define how quorum sets are exchanged between nodes,
    /// hence their retrieval is delegated to the user of SCP.
    /// The return value is not cached by SCP, as quorum sets are transient.
    ///
    /// Nonde is a valid return value which cause the statement to be
    /// considered invalid.
    fn get_qset(&self, h: &proto::SCPHash) -> Option<&proto::SCPQuorumSet>;
    fn to_str_key(&self, pk: &proto::SCPNodeID, full_key: bool) -> String;

    fn emit_envelope(&self, env: proto::SCPEnvelope);
    fn compute_timeout(&self, counter: proto::SCPCount, is_nomination: bool) -> Duration;
    fn start_timer(&self, slot_index: SlotIndex, timer: Timer, timeout: Duration);
    fn stop_timer(&self, slot_index: SlotIndex, timer: Timer);
}

pub trait DriverCb {
    fn received(&mut self, env: &proto::SCPEnvelope);
    fn timer_expired(&mut self, slot_index: SlotIndex, timer: Timer);
}
