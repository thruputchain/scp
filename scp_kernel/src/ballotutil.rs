use crate::*;
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Interval {
    pub lo: thruput_scp::SCPCount,
    pub hi: thruput_scp::SCPCount,
}

pub fn compare_ballots(
    left: &thruput_scp::SCPBallot,
    right: &thruput_scp::SCPBallot,
) -> Ordering {
    match left.counter.cmp(&right.counter) {
        Ordering::Equal => left.value.cmp(&right.value),
        other => other,
    }
}

pub fn compare_opt_ballots(
    left: &Option<thruput_scp::SCPBallot>,
    right: &Option<thruput_scp::SCPBallot>,
) -> Ordering {
    match (left, right) {
        (Some(b1), Some(b2)) => compare_ballots(b1, b2),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}

pub fn are_ballots_compatible(
    left: &thruput_scp::SCPBallot,
    right: &thruput_scp::SCPBallot,
) -> bool {
    left.value == right.value
}

pub fn are_ballots_less_and_incompatible(
    left: &thruput_scp::SCPBallot,
    right: &thruput_scp::SCPBallot,
) -> bool {
    (compare_ballots(left, right) != Ordering::Greater)
        && !are_ballots_compatible(left, right)
}

pub fn are_ballots_less_and_compatible(
    left: &thruput_scp::SCPBallot,
    right: &thruput_scp::SCPBallot,
) -> bool {
    (compare_ballots(left, right) != Ordering::Greater)
        && are_ballots_compatible(left, right)
}

pub fn has_v_blocking_subset_strictly_ahead_of<'a, C: Consensus, D: Driver>(
    local: &LocalProto<'a, C, D>,
    envelopes: &HashMap<thruput_scp::SCPNodeID, thruput_scp::SCPEnvelope>,
    n: thruput_scp::SCPCount,
) -> bool {
    LocalProto::<C, D>::is_vblocking_envelopes(
        local.get_qset(),
        envelopes,
        |st: &thruput_scp::SCPStatement| -> bool {
            statement_ballot_counter(st) > n
        },
    )
}

pub fn statement_ballot_counter(
    st: &thruput_scp::SCPStatement,
) -> thruput_scp::SCPCount {
    match &st.pledges {
        thruput_scp::SCPStatement_pledges::SCP_ST_PREPARE { prepare } => {
            prepare.ballot.counter
        }
        thruput_scp::SCPStatement_pledges::SCP_ST_CONFIRM { confirm } => {
            confirm.ballot.counter
        }
        thruput_scp::SCPStatement_pledges::SCP_ST_EXTERNALIZE { .. } => {
            u32::MAX
        }
        _ => panic!("bug"),
    }
}
