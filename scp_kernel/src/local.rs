use crate::*;
use std::collections::{HashMap, HashSet};
use tracing::instrument;
use xdr_common::VecM;

#[derive(Debug)]
pub struct LocalState {
    node_id: proto::SCPNodeID,
    is_validator: bool,
    qset: proto::SCPQuorumSet,
    qset_hash: proto::SCPHash,
    #[allow(dead_code)]
    single_qset: Option<proto::SCPQuorumSet>,
    #[allow(dead_code)]
    single_qset_hash: Option<proto::SCPHash>,
}

impl LocalState {
    pub fn new(
        node_id: proto::SCPNodeID,
        is_validator: bool,
        qset: proto::SCPQuorumSet,
        get_hash_of: impl Fn(&[u8]) -> proto::SCPHash,
    ) -> LocalState {
        // Normalize the quorum set (equivalent to normalizeQSet(mQSet) in C++)
        let mut normalized_qset = qset;
        normalize_qset(&mut normalized_qset, None);

        // Calculate hash of the normalized quorum set (equivalent to mQSetHash = driver.getHashOf(...))
        let qset_bytes = proto::marshal_xdr(&normalized_qset).unwrap();
        let qset_hash = get_hash_of(&qset_bytes);

        // Build singleton quorum set (equivalent to buildSingletonQSet(mNodeID))
        let single_qset = Self::build_singleton_qset(&node_id);

        // Calculate hash of singleton quorum set
        let single_qset_bytes = proto::marshal_xdr(&single_qset).unwrap();
        let single_qset_hash = get_hash_of(&single_qset_bytes);

        LocalState {
            node_id,
            is_validator,
            qset: normalized_qset,
            qset_hash,
            single_qset: Some(single_qset),
            single_qset_hash: Some(single_qset_hash),
        }
    }

    fn build_singleton_qset(node_id: &proto::SCPNodeID) -> proto::SCPQuorumSet {
        proto::SCPQuorumSet {
            threshold: 1,
            validators: VecM(vec![node_id.clone()]),
            inner_sets: VecM(vec![]),
        }
    }
}

#[derive(Debug)]
pub struct LocalProto<'a, C: Consensus, D: Driver> {
    pub cd: &'a ConsensusDriver<C, D>,
    pub local: &'a LocalState,
}

#[derive(Debug)]
pub struct LocalProtoMut<'a, C: Consensus, D: Driver> {
    pub cd: &'a ConsensusDriver<C, D>,
    pub local: &'a mut LocalState,
}

impl<'a, C: Consensus, D: Driver> LocalProtoMut<'a, C, D> {
    #[instrument]
    pub fn update_quorum_set(&mut self, qset: &proto::SCPQuorumSet) {
        let mut normalized_qset = qset.clone();
        normalize_qset(&mut normalized_qset, None);

        let qset_bytes = proto::marshal_xdr(&normalized_qset).unwrap();
        let qset_hash = self.cd.driver.get_hash_of(&qset_bytes);

        self.local.qset = normalized_qset;
        self.local.qset_hash = qset_hash;
    }
}

impl<'a, C: Consensus, D: Driver> LocalProto<'a, C, D> {
    pub fn get_qset(&self) -> &proto::SCPQuorumSet {
        &self.local.qset
    }

    pub fn get_qset_hash(&self) -> &proto::SCPHash {
        &self.local.qset_hash
    }

    pub fn get_singleton_qset(&self, node_id: &proto::SCPNodeID) -> proto::SCPQuorumSet {
        LocalState::build_singleton_qset(node_id)
    }

    pub fn get_node_id(&self) -> &proto::SCPNodeID {
        &self.local.node_id
    }

    pub fn is_validator(&self) -> bool {
        self.local.is_validator
    }

    pub fn for_all_nodes<F>(qset: &proto::SCPQuorumSet, proc: &mut F) -> bool
    where
        F: FnMut(&proto::SCPNodeID) -> bool,
    {
        for node in qset.validators.iter() {
            if !proc(node) {
                return false;
            }
        }

        for inner_qset in qset.inner_sets.iter() {
            if !Self::for_all_nodes(inner_qset, proc) {
                return false;
            }
        }

        true
    }

    fn is_quorum_slice_internal(qset: &proto::SCPQuorumSet, node_set: &[proto::SCPNodeID]) -> bool {
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
            if Self::is_quorum_slice_internal(inner_qset, node_set) {
                threshold_left = threshold_left.saturating_sub(1);
                if threshold_left == 0 {
                    return true;
                }
            }
        }

        false
    }

    pub fn is_quorum_slice(qset: &proto::SCPQuorumSet, node_set: &[proto::SCPNodeID]) -> bool {
        Self::is_quorum_slice_internal(qset, node_set)
    }

    fn is_vblocking_internal(qset: &proto::SCPQuorumSet, node_set: &[proto::SCPNodeID]) -> bool {
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
            if Self::is_vblocking_internal(inner_qset, node_set) {
                left_till_block -= 1;
                if left_till_block <= 0 {
                    return true;
                }
            }
        }

        false
    }

    pub fn is_vblocking_nodes(qset: &proto::SCPQuorumSet, nodes: &Vec<proto::SCPNodeID>) -> bool {
        Self::is_vblocking_internal(qset, nodes)
    }

    /// Check if filtered envelopes form a v-blocking set
    #[instrument(skip(filter))]
    pub fn is_vblocking_envelopes<F>(
        qset: &proto::SCPQuorumSet,
        envs: &HashMap<proto::SCPNodeID, proto::SCPEnvelope>,
        filter: F,
    ) -> bool
    where
        F: Fn(&proto::SCPStatement) -> bool,
    {
        // Collect node IDs where the envelope passes the filter
        let filtered_nodes: Vec<proto::SCPNodeID> = envs
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
        Self::is_vblocking_internal(qset, &filtered_nodes)
    }

    /// Check if filtered envelopes form a quorum
    pub fn is_quorum<GetStmtQSet, FilterStmts>(
        qset: &proto::SCPQuorumSet,
        envelopes: &HashMap<proto::SCPNodeID, proto::SCPEnvelope>,
        get_stmt_qset: &GetStmtQSet,
        filter: &FilterStmts,
    ) -> bool
    where
        GetStmtQSet: Fn(&proto::SCPStatement) -> Option<proto::SCPQuorumSet>,
        FilterStmts: Fn(&proto::SCPStatement) -> bool,
    {
        // Start with nodes that pass the filter
        let mut p_nodes: Vec<proto::SCPNodeID> = envelopes
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
                    if Self::is_quorum_slice_internal(&node_qset, &p_nodes) {
                        f_nodes.push(node_id.clone());
                    }
                }
            }

            p_nodes = f_nodes;

            // Break if no change (fixed point reached)
            if count == p_nodes.len() {
                break;
            }
        }

        // Check if final set forms a quorum slice for the given qset
        Self::is_quorum_slice_internal(qset, &p_nodes)
    }

    // computes the distance to the set of v-blocking sets given
    // a set of nodes that agree (but can fail)
    // excluded, if set will be skipped altogether
    pub fn find_closest_v_blocking_nodes(
        qset: &proto::SCPQuorumSet,
        nodes: &HashSet<proto::SCPNodeID>,
        excluded: Option<&proto::SCPNodeID>,
    ) -> Vec<proto::SCPNodeID> {
        // Calculate nodes needed to block: total_entries - threshold + 1
        let total_entries = qset.validators.len() + qset.inner_sets.len();
        let mut left_till_block = (total_entries + 1).saturating_sub(qset.threshold as usize);

        let mut res = Vec::new();

        // Check validators first
        for validator in qset.validators.iter() {
            // Skip excluded node
            if let Some(ex) = excluded {
                if validator == ex {
                    continue;
                }
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
        let mut res_internals: Vec<Vec<proto::SCPNodeID>> = Vec::new();

        for inner_qset in qset.inner_sets.iter() {
            let inner_result = Self::find_closest_v_blocking_nodes(inner_qset, nodes, excluded);
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
        &self,
        qset: &proto::SCPQuorumSet,
        envs: &HashMap<proto::SCPNodeID, proto::SCPEnvelope>,
        filter: F,
        excluded: Option<&proto::SCPNodeID>,
    ) -> Vec<proto::SCPNodeID>
    where
        F: Fn(&proto::SCPStatement) -> bool,
    {
        // Collect nodes that pass the filter into a HashSet
        let nodes: HashSet<proto::SCPNodeID> = envs
            .iter()
            .filter_map(|(node_id, envelope)| {
                if filter(&envelope.statement) {
                    Some(node_id.clone())
                } else {
                    None
                }
            })
            .collect();

        Self::find_closest_v_blocking_nodes(qset, &nodes, excluded)
    }

    pub fn to_json(&self, qset: &proto::SCPQuorumSet, full_keys: bool) -> serde_json::Value {
        Self::to_json_with_formatter(qset, &|node_id| {
            self.cd.driver.to_str_key(node_id, full_keys)
        })
    }

    pub fn to_json_with_formatter<F>(qset: &proto::SCPQuorumSet, formatter: &F) -> serde_json::Value
    where
        F: Fn(&proto::SCPNodeID) -> String,
    {
        let mut ret = serde_json::Map::new();
        ret.insert(
            "t".to_string(),
            serde_json::Value::Number(qset.threshold.into()),
        );

        let mut entries = Vec::new();

        // Add validators as strings
        for validator in qset.validators.iter() {
            let str_key = formatter(validator);
            if str_key.is_empty() {
                // In Rust we'll use a default representation instead of throwing
                entries.push(serde_json::Value::String(format!("{:?}", validator)));
            } else {
                entries.push(serde_json::Value::String(str_key));
            }
        }

        // Recursively add inner sets
        for inner_set in qset.inner_sets.iter() {
            entries.push(Self::to_json_with_formatter(inner_set, formatter));
        }

        ret.insert("v".to_string(), serde_json::Value::Array(entries));
        serde_json::Value::Object(ret)
    }

    /// Parse a quorum set from JSON representation
    pub fn from_json_with(
        &self,
        qset_json: &serde_json::Value,
    ) -> Result<proto::SCPQuorumSet, String> {
        let from_str_key = |s: &str| self.cd.driver.from_str_key(s);
        Self::from_json_with_parser(qset_json, &from_str_key)
    }

    /// Parse a quorum set from JSON representation
    pub fn from_json_with_parser<F>(
        qset_json: &serde_json::Value,
        from_str_key: &F,
    ) -> Result<proto::SCPQuorumSet, String>
    where
        F: Fn(&str) -> Result<proto::SCPNodeID, String>,
    {
        if !qset_json.is_object() {
            return Err("JSON field 'qset' must be an object".to_string());
        }

        let obj = qset_json.as_object().unwrap();

        // Check for required threshold field
        let threshold = match obj.get("t") {
            Some(t) if t.is_number() => match t.as_u64() {
                Some(val) if val <= u32::MAX as u64 => val as u32,
                _ => {
                    return Err(
                        "JSON field 't' must be a valid unsigned 32-bit integer".to_string()
                    );
                }
            },
            _ => return Err("JSON field 't' must exist and be an unsigned integer".to_string()),
        };

        // Check for required validators array field
        let entries = match obj.get("v") {
            Some(v) if v.is_array() => v.as_array().unwrap(),
            _ => return Err("JSON field 'v' must exist and be an array".to_string()),
        };

        let mut validators = Vec::new();
        let mut inner_sets = Vec::new();

        for entry in entries {
            if entry.is_string() {
                // Parse validator key string
                let key_str = entry.as_str().unwrap();
                match from_str_key(key_str) {
                    Ok(node_id) => validators.push(node_id),
                    Err(e) => return Err(format!("Invalid validator key in 'v' array: {}", e)),
                }
            } else if entry.is_object() {
                // Recursively parse inner quorum set
                match Self::from_json_with_parser(entry, from_str_key) {
                    Ok(inner_qset) => inner_sets.push(inner_qset),
                    Err(e) => return Err(e),
                }
            } else {
                return Err("Each entry in 'v' must be either a string (validator key) or an object (inner quorum set)".to_string());
            }
        }

        Ok(proto::SCPQuorumSet {
            threshold,
            validators: VecM(validators),
            inner_sets: VecM(inner_sets),
        })
    }

    pub fn to_string(&self, qset: &proto::SCPQuorumSet) -> String {
        let json = self.to_json(qset, false);
        serde_json::to_string(&json).unwrap()
    }
}
