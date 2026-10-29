use crate::qset;
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use tracing::{debug, instrument};

pub use qset::{QSetError, is_qset_sane};

pub fn for_all_nodes<F>(qset: &thruput_scp::SCPQuorumSet, proc: &mut F) -> bool
where
    F: FnMut(&thruput_scp::SCPNodeID) -> bool,
{
    for node in qset.validators.iter() {
        if !proc(node) {
            return false;
        }
    }

    for inner_qset in qset.inner_sets.iter() {
        if !for_all_nodes(inner_qset, proc) {
            return false;
        }
    }

    true
}

fn is_quorum_slice_internal(
    qset: &thruput_scp::SCPQuorumSet,
    node_set: &[thruput_scp::SCPNodeID],
) -> bool {
    let mut threshold_left = qset.threshold;

    for validator in qset.validators.iter() {
        if node_set.contains(validator) {
            threshold_left = threshold_left.saturating_sub(1);
            if threshold_left == 0 {
                return true;
            }
        }
    }

    for inner_qset in qset.inner_sets.iter() {
        if is_quorum_slice_internal(inner_qset, node_set) {
            threshold_left = threshold_left.saturating_sub(1);
            if threshold_left == 0 {
                return true;
            }
        }
    }

    false
}

pub fn is_quorum_slice(
    qset: &thruput_scp::SCPQuorumSet,
    node_set: &[thruput_scp::SCPNodeID],
) -> bool {
    is_quorum_slice_internal(qset, node_set)
}

fn is_vblocking_internal(
    qset: &thruput_scp::SCPQuorumSet,
    node_set: &[thruput_scp::SCPNodeID],
) -> bool {
    // There is no v-blocking set for 0-threshold
    if qset.threshold == 0 {
        return false;
    }

    // Calculate nodes needed to block: total_entries - threshold + 1
    let total_entries = qset.validators.len() + qset.inner_sets.len();
    let mut left_till_block =
        (total_entries + 1).saturating_sub(qset.threshold as usize) as i32;

    // Check validators
    for validator in qset.validators.iter() {
        if node_set.contains(validator) {
            left_till_block -= 1;
            if left_till_block <= 0 {
                return true;
            }
        }
    }

    // Check inner sets
    for inner_qset in qset.inner_sets.iter() {
        if is_vblocking_internal(inner_qset, node_set) {
            left_till_block -= 1;
            if left_till_block <= 0 {
                return true;
            }
        }
    }

    false
}

pub fn is_vblocking_nodes(
    qset: &thruput_scp::SCPQuorumSet,
    nodes: &[thruput_scp::SCPNodeID],
) -> bool {
    is_vblocking_internal(qset, nodes)
}

/// Check if filtered envelopes form a v-blocking set
#[instrument(skip(qset, envs, filter))]
pub fn is_vblocking_envelopes<F>(
    qset: &thruput_scp::SCPQuorumSet,
    envs: &HashMap<thruput_scp::SCPNodeID, thruput_scp::SCPEnvelope>,
    filter: F,
) -> bool
where
    F: Fn(&thruput_scp::SCPStatement) -> bool,
{
    // Collect node IDs where the envelope passes the filter
    let filtered_nodes: Vec<thruput_scp::SCPNodeID> = envs
        .iter()
        .filter_map(|(node_id, envelope)| {
            if filter(&envelope.statement) {
                Some(node_id.clone())
            } else {
                None
            }
        })
        .collect();

    // Check if the filtered nodes form a v-blocking set
    is_vblocking_internal(qset, &filtered_nodes)
}

/// Check if filtered envelopes form a quorum
pub fn is_quorum<GetStmtQSet, FilterStmts>(
    qset: &thruput_scp::SCPQuorumSet,
    envelopes: &HashMap<thruput_scp::SCPNodeID, thruput_scp::SCPEnvelope>,
    get_stmt_qset: &GetStmtQSet,
    filter: &FilterStmts,
) -> bool
where
    GetStmtQSet:
        Fn(&thruput_scp::SCPStatement) -> Option<thruput_scp::SCPQuorumSet>,
    FilterStmts: Fn(&thruput_scp::SCPStatement) -> bool,
{
    // TODO(frost): Add an internal bound or bounded input type for consensus

    // quorum evaluation; external proof payloads are bounded at the verifier
    // boundary.
    // Start with nodes that pass the filter
    let mut p_nodes: Vec<thruput_scp::SCPNodeID> = envelopes
        .iter()
        .filter_map(|(node_id, envelope)| {
            if filter(&envelope.statement) {
                Some(node_id.clone())
            } else {
                None
            }
        })
        .collect();

    // Iterate until convergence (fixed point)
    loop {
        let count = p_nodes.len();

        // Filter nodes that form quorum slices
        let mut f_nodes = Vec::new();
        for node_id in &p_nodes {
            let envelope = envelopes.get(node_id).unwrap();
            if let Some(node_qset) = get_stmt_qset(&envelope.statement) {
                if is_quorum_slice_internal(&node_qset, &p_nodes) {
                    f_nodes.push(node_id.clone());
                }
            } else {
                debug!(
                    "qset for {}'s envelope is missing",
                    crate::to_str_key(node_id, false)
                );
            }
        }

        p_nodes = f_nodes;

        // Break if no change (fixed point reached)
        if count == p_nodes.len() {
            break;
        }
    }

    // Check if final set forms a quorum slice for the given qset
    is_quorum_slice_internal(qset, &p_nodes)
}

// Computes the distance to the set of v-blocking sets given a set of nodes
// that agree but can fail. `excluded`, if set, will be skipped altogether.
pub fn find_closest_v_blocking_nodes(
    qset: &thruput_scp::SCPQuorumSet,
    nodes: &HashSet<thruput_scp::SCPNodeID>,
    excluded: Option<&thruput_scp::SCPNodeID>,
) -> Vec<thruput_scp::SCPNodeID> {
    // Calculate nodes needed to block: total_entries - threshold + 1
    let total_entries = qset.validators.len() + qset.inner_sets.len();
    let mut left_till_block =
        (total_entries + 1).saturating_sub(qset.threshold as usize);

    let mut res = Vec::new();

    // Check validators first
    for validator in qset.validators.iter() {
        // Skip excluded node
        if let Some(ex) = excluded
            && validator == ex
        {
            continue;
        }

        if !nodes.contains(validator) {
            // This validator is not in the agreeing set, so it's already blocking
            left_till_block = left_till_block.saturating_sub(1);
            if left_till_block == 0 {
                // Already blocked
                return Vec::new();
            }
        } else {
            res.push(validator.clone());
        }
    }

    // Collect results from inner sets, ordered by size
    let mut res_internals: Vec<Vec<thruput_scp::SCPNodeID>> = Vec::new();

    for inner_qset in qset.inner_sets.iter() {
        let inner_result =
            find_closest_v_blocking_nodes(inner_qset, nodes, excluded);
        if inner_result.is_empty() {
            // This inner set is already blocked
            left_till_block = left_till_block.saturating_sub(1);
            if left_till_block == 0 {
                return Vec::new();
            }
        } else {
            res_internals.push(inner_result);
        }
    }

    // Sort internal results by size (smallest first)
    res_internals.sort_by_key(|v| v.len());

    // Use top-level validators to get closer
    if res.len() > left_till_block {
        res.truncate(left_till_block);
    }
    left_till_block = left_till_block.saturating_sub(res.len());

    // Use subsets to get closer, using the smallest ones first
    for inner_res in res_internals.iter() {
        if left_till_block == 0 {
            break;
        }
        res.extend_from_slice(inner_res);
        left_till_block = left_till_block.saturating_sub(1);
    }

    res
}

/// Find the closest v-blocking set from filtered envelopes
pub fn find_closest_v_blocking_envelopes<F>(
    qset: &thruput_scp::SCPQuorumSet,
    envs: &HashMap<thruput_scp::SCPNodeID, thruput_scp::SCPEnvelope>,
    filter: F,
    excluded: Option<&thruput_scp::SCPNodeID>,
) -> Vec<thruput_scp::SCPNodeID>
where
    F: Fn(&thruput_scp::SCPStatement) -> bool,
{
    // Collect nodes that pass the filter into a HashSet
    let nodes: HashSet<thruput_scp::SCPNodeID> = envs
        .iter()
        .filter_map(|(node_id, envelope)| {
            if filter(&envelope.statement) {
                Some(node_id.clone())
            } else {
                None
            }
        })
        .collect();

    find_closest_v_blocking_nodes(qset, &nodes, excluded)
}

/// Error returned when a statement does not carry a companion quorum-set hash.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum CompanionQuorumSetHashError {
    #[error("statement does not carry a companion quorum-set hash")]
    MissingStatementPledge,
}

/// Returns the companion quorum-set hash carried by an SCP statement.
///
/// For EXTERNALIZE statements this is `commit_quorum_set_hash`.
pub fn try_get_companion_quorum_set_hash_from_statement(
    statement: &thruput_scp::SCPStatement,
) -> Result<thruput_scp::SCPHash, CompanionQuorumSetHashError> {
    match &statement.pledges {
        thruput_scp::SCPStatement_pledges::SCP_ST_PREPARE { prepare } => {
            Ok(prepare.quorum_set_hash)
        }
        thruput_scp::SCPStatement_pledges::SCP_ST_CONFIRM { confirm } => {
            Ok(confirm.quorum_set_hash)
        }
        thruput_scp::SCPStatement_pledges::SCP_ST_EXTERNALIZE {
            externalize,
        } => Ok(externalize.commit_quorum_set_hash),
        thruput_scp::SCPStatement_pledges::SCP_ST_NOMINATE { nominate } => {
            Ok(nominate.quorum_set_hash)
        }
        _ => Err(CompanionQuorumSetHashError::MissingStatementPledge),
    }
}

// Legacy infallible wrapper for kernel paths where malformed statement arms are
// treated as internal bugs rather than peer-proof validation failures.
pub fn get_companion_quorum_set_hash_from_statement(
    statement: &thruput_scp::SCPStatement,
) -> thruput_scp::SCPHash {
    try_get_companion_quorum_set_hash_from_statement(statement)
        .expect("bug: statement does not carry a companion quorum-set hash")
}
