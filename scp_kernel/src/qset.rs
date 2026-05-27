use std::cmp::Ordering;
use std::collections::HashSet;
use thiserror::Error;

// level = 0 when there is no nesting.
pub const MAXIMUM_QUORUM_NESTING_LEVEL: u32 = 4;

#[derive(Debug, Error)]
pub enum QSetError {
    #[error("{0}")]
    SanityError(String),
}

struct QuorumSetSanityChecker {
    extra_checks: bool,
    known_nodes: HashSet<scp_proto::SCPNodeID>,
    is_sane: bool,
    count: usize,
}

impl QuorumSetSanityChecker {
    fn new(
        qset: &scp_proto::SCPQuorumSet,
        extra_checks: bool,
    ) -> Result<Self, QSetError> {
        let mut checker = QuorumSetSanityChecker {
            extra_checks,
            known_nodes: HashSet::new(),
            is_sane: true,
            count: 0,
        };

        checker.is_sane = checker.check_sanity(qset, 0)?;

        if checker.is_sane && (checker.count < 1 || checker.count > 1000) {
            return Err(QSetError::SanityError(
                "total number of nodes in a quorum must be within 1 and 1000"
                    .to_string(),
            ));
        }

        Ok(checker)
    }

    fn is_sane(&self) -> bool {
        self.is_sane
    }

    fn check_sanity(
        &mut self,
        qset: &scp_proto::SCPQuorumSet,
        depth: u32,
    ) -> Result<bool, QSetError> {
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
        let v_blocking_size = tot_entries
            .saturating_add(1)
            .saturating_sub(qset.threshold as usize);
        self.count += validators.len();

        if qset.threshold as usize > tot_entries {
            return Err(QSetError::SanityError(
                "threshold exceeds total number of entries".to_string(),
            ));
        }

        // threshold is within the proper range
        if self.extra_checks && (qset.threshold as usize) < v_blocking_size {
            return Err(QSetError::SanityError(
                "threshold is lower than the v-blocking size (< 51%)"
                    .to_string(),
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

pub fn is_qset_sane(
    qset: &scp_proto::SCPQuorumSet,
    extra_checks: bool,
) -> Result<(), QSetError> {
    let checker = QuorumSetSanityChecker::new(qset, extra_checks)?;
    if checker.is_sane() {
        Ok(())
    } else {
        Err(QSetError::SanityError("quorum set is not sane".to_string()))
    }
}

fn normalize_qset_simplify(
    qset: &mut scp_proto::SCPQuorumSet,
    id_to_remove: Option<&scp_proto::SCPNodeID>,
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
    let mut unprocessed = qset.inner_sets.0.clone();
    qset.inner_sets.0.clear();

    while let Some(mut inner_set) = unprocessed.pop() {
        normalize_qset_simplify(&mut inner_set, id_to_remove);
        if qset.threshold == 1 && inner_set.threshold == 1 {
            for inner_inner_set in inner_set.inner_sets.0 {
                unprocessed.push(inner_inner_set);
            }
            for inner_validator in inner_set.validators.0 {
                if !qset.validators.0.contains(&inner_validator) {
                    qset.validators.0.push(inner_validator);
                }
            }
        } else {
            qset.inner_sets.0.push(inner_set);
        }
    }

    // simplify quorum set if needed
    // (threshold=1, no validators, 1 inner set)
    if qset.threshold == 1
        && qset.validators.is_empty()
        && qset.inner_sets.len() == 1
    {
        let inner_set = qset.inner_sets.0.pop().unwrap().clone();
        *qset = inner_set;
    }
}

pub fn compare_public_key(
    l: &scp_proto::SCPPublicKey,
    r: &scp_proto::SCPPublicKey,
) -> Ordering {
    match (l, r) {
        (
            scp_proto::SCPPublicKey::Ed25519 { ed25519: l_key },
            scp_proto::SCPPublicKey::Ed25519 { ed25519: r_key },
        ) => l_key.as_ref().cmp(r_key.as_ref()),
        _ => panic!("unreachable"),
    }
}

fn qset_compare(
    l: &scp_proto::SCPQuorumSet,
    r: &scp_proto::SCPQuorumSet,
) -> Ordering {
    // Compare by validators first - lexicographically (element by element)
    for (l_validator, r_validator) in
        l.validators.0.iter().zip(r.validators.0.iter())
    {
        let validator_cmp = compare_public_key(l_validator, r_validator);
        if validator_cmp != Ordering::Equal {
            return validator_cmp;
        }
    }

    // If we've compared all common elements and they're equal, shorter vector is less than
    let validators_len_cmp = l.validators.0.len().cmp(&r.validators.0.len());
    if validators_len_cmp != Ordering::Equal {
        return validators_len_cmp;
    }

    // Then compare by inner sets (lexicographically, element by element)
    for (l_inner, r_inner) in l.inner_sets.0.iter().zip(r.inner_sets.0.iter()) {
        let inner_cmp = qset_compare(l_inner, r_inner);
        if inner_cmp != Ordering::Equal {
            return inner_cmp;
        }
    }

    // If we've compared all common elements and they're equal, shorter vector is less than
    let inner_sets_len_cmp = l.inner_sets.0.len().cmp(&r.inner_sets.0.len());
    if inner_sets_len_cmp != Ordering::Equal {
        return inner_sets_len_cmp;
    }

    // Finally compare by threshold
    l.threshold.cmp(&r.threshold)
}

fn normalize_quorum_set_reorder(qset: &mut scp_proto::SCPQuorumSet) {
    // Sort validators using custom comparison
    qset.validators.0.sort_by(compare_public_key);

    // Recursively reorder inner sets
    for inner_set in qset.inner_sets.0.iter_mut() {
        normalize_quorum_set_reorder(inner_set);
    }

    // Sort inner sets using custom comparison
    qset.inner_sets.0.sort_by(qset_compare);
}

pub fn normalize_qset(
    qset: &mut scp_proto::SCPQuorumSet,
    id_to_remove: Option<&scp_proto::SCPNodeID>,
) {
    normalize_qset_simplify(qset, id_to_remove);
    normalize_quorum_set_reorder(qset);
}
