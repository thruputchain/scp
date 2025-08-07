use derive_more::{Constructor, Deref, DerefMut, From, Into};
use std::cmp::{Ordering, PartialEq, PartialOrd};

/// Ballot augments proto::SCPBallot with partial order
#[derive(From, Into, Deref, DerefMut, Constructor)]
#[repr(transparent)]
pub struct Ballot(proto::SCPBallot);

impl PartialEq for Ballot {
    fn eq(&self, other: &Self) -> bool {
        self.0.counter == other.0.counter && self.0.value == other.0.value
    }
}

impl PartialOrd for Ballot {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.0.value != other.0.value {
            None
        } else {
            self.0.counter.partial_cmp(&other.0.counter)
        }
    }
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
