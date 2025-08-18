use crate::*;
use std::collections::{HashMap, HashSet};
use xdr_common::VecM;

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

    pub fn is_newer_node_statement(
        &self,
        node_id: &proto::SCPNodeID,
        st: &proto::SCPStatementNominate,
    ) -> bool {
        match self.nomination().latest_nominations.get(node_id) {
            None => true,
            Some(old_envelope) => {
                if let proto::SCPStatement_pledges::SCP_ST_NOMINATE { nominate: old_st } =
                    &old_envelope.statement.pledges
                {
                    Self::is_newer_statement(old_st, st)
                } else {
                    false
                }
            }
        }
    }

    fn is_newer_statement(
        oldst: &proto::SCPStatementNominate,
        st: &proto::SCPStatementNominate,
    ) -> bool {
        let mut grows_votes = false;

        if Self::is_subset(&oldst.votes, &st.votes, &mut grows_votes) {
            let mut grows_accepted = false;
            if Self::is_subset(&oldst.accepted, &st.accepted, &mut grows_accepted) {
                grows_votes || grows_accepted
            } else {
                false
            }
        } else {
            false
        }
    }

    fn is_subset(
        p: &VecM<proto::SCPValue>,
        v: &VecM<proto::SCPValue>,
        not_equal: &mut bool,
    ) -> bool {
        if p.len() <= v.len() {
            let mut p_sorted: Vec<_> = p.iter().cloned().collect();
            let mut v_sorted: Vec<_> = v.iter().cloned().collect();
            p_sorted.sort();
            v_sorted.sort();

            let is_subset = p_sorted
                .iter()
                .all(|item| v_sorted.binary_search(item).is_ok());
            if is_subset {
                *not_equal = p.len() != v.len();
            } else {
                *not_equal = true;
            }
            is_subset
        } else {
            *not_equal = true;
            false
        }
    }

    pub fn is_sane(&self, st: &proto::SCPStatement) -> bool {
        if let proto::SCPStatement_pledges::SCP_ST_NOMINATE { nominate: nom } = &st.pledges {
            // Check that nomination has at least one vote or accepted value
            let has_content = !nom.votes.is_empty() || !nom.accepted.is_empty();

            if !has_content {
                return false;
            }

            // Check that votes are sorted
            let votes_sorted = Self::is_sorted(&nom.votes);

            // Check that accepted values are sorted
            let accepted_sorted = Self::is_sorted(&nom.accepted);

            votes_sorted && accepted_sorted
        } else {
            panic!("bug")
        }
    }

    fn is_sorted(values: &VecM<proto::SCPValue>) -> bool {
        if values.len() <= 1 {
            return true;
        }

        let values_vec: Vec<_> = values.iter().collect();
        values_vec.windows(2).all(|window| window[0] <= window[1])
    }
}
