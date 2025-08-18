use crate::*;
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Interval {
    pub lo: proto::SCPCount,
    pub hi: proto::SCPCount,
}

pub fn compare_ballots(left: &proto::SCPBallot, right: &proto::SCPBallot) -> Ordering {
    match left.counter.cmp(&right.counter) {
        Ordering::Equal => left.value.cmp(&right.value),
        other => other,
    }
}

pub fn compare_opt_ballots(
    left: &Option<proto::SCPBallot>,
    right: &Option<proto::SCPBallot>,
) -> Ordering {
    match (left, right) {
        (Some(b1), Some(b2)) => compare_ballots(b1, b2),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}

pub fn are_ballots_compatible(left: &proto::SCPBallot, right: &proto::SCPBallot) -> bool {
    left.counter == right.counter
}

pub fn are_ballots_less_and_incompatible(
    left: &proto::SCPBallot,
    right: &proto::SCPBallot,
) -> bool {
    (compare_ballots(left, right) != Ordering::Greater) && !are_ballots_compatible(left, right)
}

pub fn are_ballots_less_and_compatible(left: &proto::SCPBallot, right: &proto::SCPBallot) -> bool {
    (compare_ballots(left, right) != Ordering::Greater) && are_ballots_compatible(left, right)
}

pub fn has_v_blocking_subset_strictly_ahead_of<'a, 'b, C: Consensus, D: Driver>(
    local: &LocalProto<'a, 'b, C, D>,
    envelopes: &HashMap<proto::SCPNodeID, proto::SCPEnvelope>,
    n: proto::SCPCount,
) -> bool {
    LocalProto::<C, D>::is_vblocking_envelopes(
        local.get_qset(),
        envelopes,
        |st: &proto::SCPStatement| -> bool { statement_ballot_counter(st) > n },
    )
}

pub fn statement_ballot_counter(st: &proto::SCPStatement) -> proto::SCPCount {
    match &st.pledges {
        proto::SCPStatement_pledges::SCP_ST_PREPARE { prepare } => prepare.ballot.counter,
        proto::SCPStatement_pledges::SCP_ST_CONFIRM { confirm } => confirm.ballot.counter,
        proto::SCPStatement_pledges::SCP_ST_EXTERNALIZE { .. } => u32::MAX,
        _ => panic!("bug"),
    }
}
