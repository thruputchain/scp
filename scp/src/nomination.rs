use crate::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct NominationState {
    round_number: i32,
    votes: HashSet<proto::SCPValue>,      // X
    accepted: HashSet<proto::SCPValue>,   // Y
    candidates: HashSet<proto::SCPValue>, // Z
    latest_nominations: HashMap<proto::SCPNodeID, proto::SCPEnvelope>, // N
    last_envelope: Option<proto::SCPEnvelope>, // last envelope emitted by this node
    round_leaders: HashSet<proto::SCPNodeID>, // nodes from quorum set that have the highest priority this round
    nomination_started: bool,                 // true if 'nominate' was called
    latest_composite_candidate: Option<proto::SCPValue>, // the latest (if any) candidate value
    previous_value: Option<proto::SCPValue>,  // the value from the previous slot
    timer_exp_count: i32, // the number of times the timer has expired; used for the quorum endpoint
}

impl NominationState {
    pub fn new() -> Self {
        NominationState {
            round_number: 0,
            votes: HashSet::new(),
            accepted: HashSet::new(),
            candidates: HashSet::new(),
            latest_nominations: HashMap::new(),
            last_envelope: None,
            round_leaders: HashSet::new(),
            nomination_started: false,
            latest_composite_candidate: None,
            previous_value: None,
            timer_exp_count: 0,
        }
    }
}

#[derive(Debug)]
pub struct NominationProto<'a, 'b, C: Consensus, D: Driver>(pub &'a mut SlotProto<'b, C, D>);

impl<'a, 'b, C: Consensus, D: Driver> NominationProto<'a, 'b, C, D> {
    fn slot_mut(&mut self) -> &mut SlotProto<'b, C, D> {
        self.0
    }

    fn slot(&self) -> &SlotProto<'b, C, D> {
        self.0
    }

    fn nomination(&self) -> &NominationState {
        self.slot().nomination()
    }

    fn nomination_mut(&mut self) -> &mut NominationState {
        self.slot_mut().nomination_mut()
    }

    fn driver(&self) -> &D {
        self.slot().driver()
    }

    fn consensus(&self) -> &C {
        self.slot().consensus()
    }
}
