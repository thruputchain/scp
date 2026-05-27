use crate::*;
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Interval {
    pub lo: scp_proto::SCPCount,
    pub hi: scp_proto::SCPCount,
}

pub fn compare_ballots(
    left: &scp_proto::SCPBallot,
    right: &scp_proto::SCPBallot,
) -> Ordering {
    match left.counter.cmp(&right.counter) {
        Ordering::Equal => left.value.cmp(&right.value),
        other => other,
    }
}

pub fn compare_opt_ballots(
    left: &Option<scp_proto::SCPBallot>,
    right: &Option<scp_proto::SCPBallot>,
) -> Ordering {
    match (left, right) {
        (Some(b1), Some(b2)) => compare_ballots(b1, b2),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}

pub fn are_ballots_compatible(
    left: &scp_proto::SCPBallot,
    right: &scp_proto::SCPBallot,
) -> bool {
    left.value == right.value
}

pub fn are_ballots_less_and_incompatible(
    left: &scp_proto::SCPBallot,
    right: &scp_proto::SCPBallot,
) -> bool {
    (compare_ballots(left, right) != Ordering::Greater)
        && !are_ballots_compatible(left, right)
}

pub fn are_ballots_less_and_compatible(
    left: &scp_proto::SCPBallot,
    right: &scp_proto::SCPBallot,
) -> bool {
    (compare_ballots(left, right) != Ordering::Greater)
        && are_ballots_compatible(left, right)
}

pub fn has_v_blocking_subset_strictly_ahead_of<'a, C: Consensus, D: Driver>(
    local: &LocalProto<'a, C, D>,
    envelopes: &HashMap<scp_proto::SCPNodeID, scp_proto::SCPEnvelope>,
    n: scp_proto::SCPCount,
) -> bool {
    LocalProto::<C, D>::is_vblocking_envelopes(
        local.get_qset(),
        envelopes,
        |st: &scp_proto::SCPStatement| -> bool {
            statement_ballot_counter(st) > n
        },
    )
}

pub fn statement_ballot_counter(
    st: &scp_proto::SCPStatement,
) -> scp_proto::SCPCount {
    match &st.pledges {
        scp_proto::SCPStatement_pledges::SCP_ST_PREPARE { prepare } => {
            prepare.ballot.counter
        }
        scp_proto::SCPStatement_pledges::SCP_ST_CONFIRM { confirm } => {
            confirm.ballot.counter
        }
        scp_proto::SCPStatement_pledges::SCP_ST_EXTERNALIZE { .. } => u32::MAX,
        _ => panic!("bug"),
    }
}
