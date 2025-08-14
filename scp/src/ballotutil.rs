use std::cmp::Ordering;

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
