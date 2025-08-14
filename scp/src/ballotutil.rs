use crate::*;
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Interval {
    pub lo: proto::SCPCount,
    pub hi: proto::SCPCount,
}

pub fn compare_ballots(left: &proto::SCPBallot, right: &proto::SCPBallot) -> Ordering {
    panic!("XXX")
}

pub fn compare_opt_ballots(
    left: &Option<proto::SCPBallot>,
    right: &Option<proto::SCPBallot>,
) -> Ordering {
    panic!("XXX")
}

pub fn are_ballots_compatible(left: &proto::SCPBallot, right: &proto::SCPBallot) -> bool {
    panic!("XXX")
}

pub fn are_ballots_less_and_incompatible(
    left: &proto::SCPBallot,
    right: &proto::SCPBallot,
) -> bool {
    panic!("XXX")
}

pub fn are_ballots_less_and_compatible(left: &proto::SCPBallot, right: &proto::SCPBallot) -> bool {
    panic!("XXX")
}

pub fn has_v_blocking_subset_strictly_ahead_of(
    local: &Local,
    envelopes: &HashMap<proto::SCPNodeID, proto::SCPEnvelope>,
    n: proto::SCPCount,
) -> bool {
    local.is_vblocking_envelopes(
        local.get_quorum_set(),
        envelopes,
        |st: &proto::SCPStatement| -> bool { statement_ballot_counter(st) > n },
    )
}
