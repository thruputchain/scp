use crate::*;
use derive_more::{Constructor, Deref, DerefMut, From, Into};
use std::cmp::{Ordering, PartialEq, PartialOrd};
use xdr_common::*;

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

/// BallotRegion represents all Ballot(n, x) with low <= n <= high and (all x, or specific x)
#[derive(Clone, Copy, PartialEq, Debug, Constructor)]
pub struct BallotRegion<'a> {
    counter_range: CounterRange,
    value_constraint: ValueConstraint<'a>,
}

#[derive(Clone, Copy, PartialEq, Debug, Constructor)]
pub struct BallotRegionRefinement<'a> {
    left_only: BallotRegion<'a>,
    both: BallotRegion<'a>,
    right_only: BallotRegion<'a>,
}

pub fn refine_ballot_regions<'a>(
    left: &'a BallotRegion,
    right: &'a BallotRegion,
) -> BallotRegionRefinement<'a> {
    panic!("XXX")
}

#[derive(Clone, Copy, PartialEq, Debug, Constructor)]
pub struct CounterRange {
    at_least: u32,
    at_most: u32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ValueConstraint<'a> {
    All,
    One(&'a BytesM),
}
