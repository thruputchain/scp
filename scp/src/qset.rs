use std::collections::HashSet;
use thiserror::Error;
use xdr_common::VecM;

// level = 0 when there is no nesting.
pub const MAXIMUM_QUORUM_NESTING_LEVEL: u32 = 4;

#[derive(Debug, Error)]
pub enum QSetError {
    #[error("{0}")]
    SanityError(String),
}

struct QuorumSetSanityChecker {
    extra_checks: bool,
    known_nodes: HashSet<proto::SCPNodeID>,
    is_sane: bool,
    count: usize,
}

impl QuorumSetSanityChecker {
    fn new(qset: &proto::SCPQuorumSet, extra_checks: bool) -> Result<Self, QSetError> {
        let mut checker = QuorumSetSanityChecker {
            extra_checks,
            known_nodes: HashSet::new(),
            is_sane: true,
            count: 0,
        };

        checker.is_sane = checker.check_sanity(qset, 0)?;

        if checker.is_sane && (checker.count < 1 || checker.count > 1000) {
            return Err(QSetError::SanityError(
                "total number of nodes in a quorum must be within 1 and 1000".to_string(),
            ));
        }

        Ok(checker)
    }

    fn is_sane(&self) -> bool {
        self.is_sane
    }

    fn check_sanity(&mut self, qset: &proto::SCPQuorumSet, depth: u32) -> Result<bool, QSetError> {
        if depth > MAXIMUM_QUORUM_NESTING_LEVEL {
            return Err(QSetError::SanityError(
                "maximum quorum nesting level exceeded".to_string(),
            ));
        }

        if qset.threshold < 1 {
            return Err(QSetError::SanityError(
                "threshold must be greater than 0".to_string(),
            ));
        }

        let validators = &qset.validators;
        let inner_sets = &qset.inner_sets;

        let tot_entries = validators.len() + inner_sets.len();
        let v_blocking_size = tot_entries - qset.threshold as usize + 1;
        self.count += validators.len();

        if qset.threshold as usize > tot_entries {
            return Err(QSetError::SanityError(
                "threshold exceeds total number of entries".to_string(),
            ));
        }

        // threshold is within the proper range
        if self.extra_checks && (qset.threshold as usize) < v_blocking_size {
            return Err(QSetError::SanityError(
                "threshold is lower than the v-blocking size (< 51%)".to_string(),
            ));
        }

        for node in validators.iter() {
            if !self.known_nodes.insert(node.clone()) {
                return Err(QSetError::SanityError(
                    "duplicate node found in quorum configuration".to_string(),
                ));
            }
        }

        for inner_set in inner_sets.iter() {
            self.check_sanity(inner_set, depth + 1)?;
        }

        Ok(true)
    }
}

pub fn is_qset_sane(qset: &proto::SCPQuorumSet, extra_checks: bool) -> Result<(), QSetError> {
    let checker = QuorumSetSanityChecker::new(qset, extra_checks)?;
    if checker.is_sane() {
        Ok(())
    } else {
        Err(QSetError::SanityError("quorum set is not sane".to_string()))
    }
}

fn normalize_qset_simplify(
    qset: &mut proto::SCPQuorumSet,
    id_to_remove: Option<&proto::SCPNodeID>,
) {
    // remove the specified node ID if provided
    if let Some(id_to_remove) = id_to_remove {
        let validators = &mut qset.validators;
        let original_len = validators.len();
        validators.0.retain(|node| node != id_to_remove);
        let removed_count = original_len - validators.len();
        qset.threshold = qset.threshold.saturating_sub(removed_count as u32);
    }

    // process inner sets
    let mut updated_inner_sets = VecM::<proto::SCPQuorumSet>(Vec::new());
    for mut inner_set in qset.inner_sets.0.iter_mut() {
        normalize_qset_simplify(&mut inner_set, id_to_remove);

        // check if this inner set can be merged into validator list
        // (threshold=1, 1 validator, 0 inner sets)
        if inner_set.threshold == 1
            && inner_set.validators.len() == 1
            && inner_set.inner_sets.len() == 0
        {
            qset.validators
                .0
                .push(inner_set.validators.iter().next().unwrap().clone());
        } else {
            updated_inner_sets.0.push(inner_set.clone());
        }
    }
    qset.inner_sets = updated_inner_sets;

    // simplify quorum set if needed
    // (threshold=1, no validators, 1 inner set)
    if qset.threshold == 1 && qset.validators.len() == 0 && qset.inner_sets.len() == 1 {
        let inner_set = qset.inner_sets.iter().next().unwrap().clone();
        *qset = inner_set;
    }
}

pub fn normalize_qset(qset: &mut proto::SCPQuorumSet, id_to_remove: Option<&proto::SCPNodeID>) {
    normalize_qset_simplify(qset, id_to_remove);
    // normalizeQuorumSetReorder(qSet);
    panic!("XXX")
}
