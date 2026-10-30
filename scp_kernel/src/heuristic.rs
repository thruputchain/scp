fn compute_weight(m: u64, total: u64, threshold: u64) -> u64 {
    assert!(threshold <= total);
    // Since threshold <= total, calculating res=m*threshold/total will always
    // produce res <= m, and we do not need to handle the possibility of overflow.
    // We implement big division with rounding up manually
    let (r, overflow) = unsigned_div_up(m, threshold, total);
    assert!(!overflow);
    r
}

fn unsigned_div_up(a: u64, b: u64, c: u64) -> (u64, bool) {
    assert!(c > 0);
    let a: u128 = a.into();
    let b: u128 = b.into();
    let c: u128 = c.into();
    let x = (a * b).div_ceil(c);
    (x as u64, false)
}

/// Returns the weight of the node within the qset normalized between 0-u64::MAX.
/// If `node_id` is the local node, then set `is_local_node` to `true`.
///
/// If a validator is repeated multiple times its weight is only the weight of
/// the first occurrence.
pub fn get_node_weight(
    node_id: &thruput_scp::SCPNodeID,
    qset: &thruput_scp::SCPQuorumSet,
    is_local_node: bool,
) -> u64 {
    if is_local_node {
        // local node is in all quorum sets
        return u64::MAX;
    }

    let n = qset.threshold as u64;
    let d = qset.inner_sets.len() as u64 + qset.validators.len() as u64;

    // Check if the node is directly in the validators list
    for qset_node in qset.validators.iter() {
        if qset_node == node_id {
            return compute_weight(u64::MAX, d, n);
        }
    }

    // Recursively check inner sets
    for q in qset.inner_sets.iter() {
        let leaf_w = get_node_weight(node_id, q, is_local_node);
        if leaf_w != 0 {
            return compute_weight(leaf_w, d, n);
        }
    }

    0
}
