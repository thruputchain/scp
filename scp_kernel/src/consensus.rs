use crate::*;
use std::collections::HashSet;
use std::fmt::Debug;

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub enum ValidationLevel {
    Invalid = 0,    // definitively invalid
    MaybeValid = 1, // unable to fully decide at this stage; value is possibly valid
    FullyValid = 2, // definitively valid at this stage (validate_tentatively or validate_fully)
}

pub trait Consensus: ConsensusNotify + Clone + Debug {
    fn validate_ballot_value(
        &self,
        slot_index: thruput_scp::SCPSlotIndex,
        value: &thruput_scp::SCPValue,
    ) -> ValidationLevel;

    fn validate_nomination_value(
        &self,
        slot_index: thruput_scp::SCPSlotIndex,
        value: &thruput_scp::SCPValue,
    ) -> ValidationLevel;

    fn extract_valid_value(
        &self,
        slot_index: thruput_scp::SCPSlotIndex,
        value: &thruput_scp::SCPValue,
    ) -> Option<thruput_scp::SCPValue>;

    fn combine_candidates(
        &self,
        slot_index: thruput_scp::SCPSlotIndex,
        candidates: &HashSet<thruput_scp::SCPValue>,
    ) -> Option<thruput_scp::SCPValue>;
}

pub trait ConsensusNotify {
    fn value_externalized(
        &self,
        slot_index: SlotIndex,
        value: &thruput_scp::SCPValue,
    );
    fn started_ballot_proto(
        &self,
        slot_index: SlotIndex,
        ballot: &thruput_scp::SCPBallot,
    );
    fn accepted_ballot_prepared(
        &self,
        slot_index: SlotIndex,
        ballot: &thruput_scp::SCPBallot,
    );
    fn confirmed_ballot_prepared(
        &self,
        slot_index: SlotIndex,
        ballot: &thruput_scp::SCPBallot,
    );
    fn accepted_commit(
        &self,
        slot_index: SlotIndex,
        ballot: &thruput_scp::SCPBallot,
    );
    fn ballot_did_hear_from_quorum(
        &self,
        slot_index: SlotIndex,
        ballot: &thruput_scp::SCPBallot,
    );
    fn nominating_value(
        &self,
        slot_index: SlotIndex,
        value: &thruput_scp::SCPValue,
    );
    fn updated_candidate_value(
        &self,
        slot_index: SlotIndex,
        value: &thruput_scp::SCPValue,
    );
}
