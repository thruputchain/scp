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
        slot_index: scp_proto::SCPSlotIndex,
        value: &scp_proto::SCPValue,
    ) -> ValidationLevel;

    fn validate_nomination_value(
        &self,
        slot_index: scp_proto::SCPSlotIndex,
        value: &scp_proto::SCPValue,
    ) -> ValidationLevel;

    fn extract_valid_value(
        &self,
        slot_index: scp_proto::SCPSlotIndex,
        value: &scp_proto::SCPValue,
    ) -> Option<scp_proto::SCPValue>;

    fn combine_candidates(
        &self,
        slot_index: scp_proto::SCPSlotIndex,
        candidates: &HashSet<scp_proto::SCPValue>,
    ) -> Option<scp_proto::SCPValue>;
}

pub trait ConsensusNotify {
    fn value_externalized(
        &self,
        slot_index: SlotIndex,
        value: &scp_proto::SCPValue,
    );
    fn started_ballot_proto(
        &self,
        slot_index: SlotIndex,
        ballot: &scp_proto::SCPBallot,
    );
    fn accepted_ballot_prepared(
        &self,
        slot_index: SlotIndex,
        ballot: &scp_proto::SCPBallot,
    );
    fn confirmed_ballot_prepared(
        &self,
        slot_index: SlotIndex,
        ballot: &scp_proto::SCPBallot,
    );
    fn accepted_commit(
        &self,
        slot_index: SlotIndex,
        ballot: &scp_proto::SCPBallot,
    );
    fn ballot_did_hear_from_quorum(
        &self,
        slot_index: SlotIndex,
        ballot: &scp_proto::SCPBallot,
    );
    fn nominating_value(
        &self,
        slot_index: SlotIndex,
        value: &scp_proto::SCPValue,
    );
    fn updated_candidate_value(
        &self,
        slot_index: SlotIndex,
        value: &scp_proto::SCPValue,
    );
}
