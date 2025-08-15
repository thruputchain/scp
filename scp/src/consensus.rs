use crate::*;

#[derive(PartialEq, PartialOrd, Ord, Eq)]
pub enum ValidationLevel {
    Invalid = 0,    // definitively invalid
    MaybeValid = 1, // unable to fully decide at this stage; value is possibly valid
    FullyValid = 2, // definitively valid at this stage (validate_tentatively or validate_fully)
}

pub trait Consensus: ConsensusCb + ConsensusNotify {
    fn validate_value(
        &self,
        slot_index: SCPSlotIndex,
        value: &proto::SCPValue,
        nomination: bool,
    ) -> ValidationLevel;

    // XXX: async
    // fn validate(&self, slot_index: SCPSlotIndex, value: &proto::SCPValue);
    // fn cancel_validate(&self, slot_index: SCPSlotIndex, value: &proto::SCPValue);
}

pub trait ConsensusCb {
    // fn validated(
    //     &mut self,
    //     slot_index: SCPSlotIndex,
    //     value: &proto::SCPValue,
    //     result: ValidationLevel,
    // );
}

pub trait ConsensusNotify {
    fn value_externalized(&self, slot_index: SlotIndex, value: &proto::SCPValue);
    //
    fn started_ballot_proto(&self, slot_index: SlotIndex, ballot: &proto::SCPBallot);
    fn accepted_ballot_prepared(&self, slot_index: SlotIndex, ballot: &proto::SCPBallot);
    fn confirmed_ballot_prepared(&self, slot_index: SlotIndex, ballot: &proto::SCPBallot);
    fn accepted_commit(&self, slot_index: SlotIndex, ballot: &proto::SCPBallot);
    fn ballot_did_hear_from_quorum(&self, slot_index: SlotIndex, ballot: &proto::SCPBallot);
}
