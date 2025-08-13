use crate::*;

pub enum ValidationLevel {
    Invalid,    // definitively invalid
    MaybeValid, // unable to fully decide at this stage; value is possibly valid
    FullyValid, // definitively valid at this stage (validate_tentatively or validate_fully)
}

pub trait Consensus: ConsensusCb + ConsensusNotify {
    fn validate(&self, slot_index: SCPSlotIndex, value: &proto::SCPValue);
    fn cancel_validate(&self, slot_index: SCPSlotIndex, value: &proto::SCPValue);
}

pub trait ConsensusCb {
    fn validated(
        &mut self,
        slot_index: SCPSlotIndex,
        value: &proto::SCPValue,
        result: ValidationLevel,
    );
}

pub trait ConsensusNotify {
    fn started_ballot_proto(&self, slot_index: SlotIndex, ballot: &proto::SCPBallot);
}
