use crate::*;

#[derive(PartialEq, PartialOrd, Ord, Eq)]
pub enum QuorumInfoNodeState {
    NoInfo = 0,
    Agree = 1,
    Missing = 2,
    Delayed = 3,
    Disagree = 4,
}
