use crate::*;
use std::collections::{HashMap, HashSet};
use tracing::{debug, instrument};
use xdr_common::VecM;

#[derive(Debug)]
pub struct LocalState {
    node_id: thruput_scp::SCPNodeID,
    is_validator: bool,
    qset: thruput_scp::SCPQuorumSet,
    qset_hash: thruput_scp::SCPHash,
    #[allow(dead_code)]
    single_qset: Option<thruput_scp::SCPQuorumSet>,
    #[allow(dead_code)]
    single_qset_hash: Option<thruput_scp::SCPHash>,
}

impl LocalState {
    pub fn new(
        node_id: thruput_scp::SCPNodeID,
        is_validator: bool,
        qset: thruput_scp::SCPQuorumSet,
        get_hash_of: impl Fn(&[u8]) -> thruput_scp::SCPHash,
    ) -> LocalState {
        debug!(
            "local, new, qset: {}",
            serde_json::to_string_pretty(&qset).unwrap()
        );

        // Normalize the quorum set (equivalent to normalizeQSet(mQSet) in C++)
        let mut normalized_qset = qset;
        normalize_qset(&mut normalized_qset, None);

        debug!(
            "local, new, normalized qset: {}",
            serde_json::to_string_pretty(&normalized_qset).unwrap()
        );

        // Calculate hash of the normalized quorum set (equivalent to mQSetHash = driver.getHashOf(...))
        let qset_bytes = scp_util::marshal_xdr(&normalized_qset).unwrap();
        let qset_hash = get_hash_of(&qset_bytes);

        // Build singleton quorum set (equivalent to buildSingletonQSet(mNodeID))
        let single_qset = Self::build_singleton_qset(&node_id);

        // Calculate hash of singleton quorum set
        let single_qset_bytes = scp_util::marshal_xdr(&single_qset).unwrap();
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

    fn build_singleton_qset(
        node_id: &thruput_scp::SCPNodeID,
    ) -> thruput_scp::SCPQuorumSet {
        thruput_scp::SCPQuorumSet {
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
    #[instrument(skip_all)]
    pub fn update_quorum_set(&mut self, qset: &thruput_scp::SCPQuorumSet) {
        debug!(
            "local, update_quorum_set, qset: {}",
            serde_json::to_string_pretty(qset).unwrap()
        );

        let mut normalized_qset = qset.clone();
        normalize_qset(&mut normalized_qset, None);

        debug!(
            "local, update_quorum_set, normalized qset: {}",
            serde_json::to_string_pretty(&normalized_qset).unwrap()
        );

        let qset_bytes = scp_util::marshal_xdr(&normalized_qset).unwrap();
        let qset_hash = self.cd.driver.get_hash_of(&qset_bytes);

        self.local.qset = normalized_qset;
        self.local.qset_hash = qset_hash;
    }
}

impl<'a, C: Consensus, D: Driver> LocalProto<'a, C, D> {
    pub fn get_qset(&self) -> &thruput_scp::SCPQuorumSet {
        &self.local.qset
    }

    pub fn get_qset_hash(&self) -> &thruput_scp::SCPHash {
        &self.local.qset_hash
    }

    pub fn get_singleton_qset(
        &self,
        node_id: &thruput_scp::SCPNodeID,
    ) -> thruput_scp::SCPQuorumSet {
        LocalState::build_singleton_qset(node_id)
    }

    pub fn get_node_id(&self) -> &thruput_scp::SCPNodeID {
        &self.local.node_id
    }

    pub fn is_validator(&self) -> bool {
        self.local.is_validator
    }

    pub fn for_all_nodes<F>(
        qset: &thruput_scp::SCPQuorumSet,
        proc: &mut F,
    ) -> bool
    where
        F: FnMut(&thruput_scp::SCPNodeID) -> bool,
    {
        federated::for_all_nodes(qset, proc)
    }

    pub fn is_quorum_slice(
        qset: &thruput_scp::SCPQuorumSet,
        node_set: &[thruput_scp::SCPNodeID],
    ) -> bool {
        federated::is_quorum_slice(qset, node_set)
    }

    pub fn is_vblocking_nodes(
        qset: &thruput_scp::SCPQuorumSet,
        nodes: &[thruput_scp::SCPNodeID],
    ) -> bool {
        federated::is_vblocking_nodes(qset, nodes)
    }

    /// Check if filtered envelopes form a v-blocking set
    pub fn is_vblocking_envelopes<F>(
        qset: &thruput_scp::SCPQuorumSet,
        envs: &HashMap<thruput_scp::SCPNodeID, thruput_scp::SCPEnvelope>,
        filter: F,
    ) -> bool
    where
        F: Fn(&thruput_scp::SCPStatement) -> bool,
    {
        federated::is_vblocking_envelopes(qset, envs, filter)
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
        federated::is_quorum(qset, envelopes, get_stmt_qset, filter)
    }

    // computes the distance to the set of v-blocking sets given
    // a set of nodes that agree (but can fail)
    // excluded, if set will be skipped altogether
    pub fn find_closest_v_blocking_nodes(
        qset: &thruput_scp::SCPQuorumSet,
        nodes: &HashSet<thruput_scp::SCPNodeID>,
        excluded: Option<&thruput_scp::SCPNodeID>,
    ) -> Vec<thruput_scp::SCPNodeID> {
        federated::find_closest_v_blocking_nodes(qset, nodes, excluded)
    }

    /// Find the closest v-blocking set from filtered envelopes
    pub fn find_closest_v_blocking_envelopes<F>(
        &self,
        qset: &thruput_scp::SCPQuorumSet,
        envs: &HashMap<thruput_scp::SCPNodeID, thruput_scp::SCPEnvelope>,
        filter: F,
        excluded: Option<&thruput_scp::SCPNodeID>,
    ) -> Vec<thruput_scp::SCPNodeID>
    where
        F: Fn(&thruput_scp::SCPStatement) -> bool,
    {
        federated::find_closest_v_blocking_envelopes(
            qset, envs, filter, excluded,
        )
    }

    pub fn to_json(
        &self,
        qset: &thruput_scp::SCPQuorumSet,
        full_keys: bool,
    ) -> serde_json::Value {
        Self::to_json_with_formatter(qset, &|node_id| {
            self.cd.driver.to_str_key(node_id, full_keys)
        })
    }

    pub fn to_json_with_formatter<F>(
        qset: &thruput_scp::SCPQuorumSet,
        formatter: &F,
    ) -> serde_json::Value
    where
        F: Fn(&thruput_scp::SCPNodeID) -> String,
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
                entries.push(serde_json::Value::String(format!(
                    "{:?}",
                    validator
                )));
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
    ) -> Result<thruput_scp::SCPQuorumSet, String> {
        let from_str_key = |s: &str| self.cd.driver.from_str_key(s);
        Self::from_json_with_parser(qset_json, &from_str_key)
    }

    /// Parse a quorum set from JSON representation
    pub fn from_json_with_parser<F>(
        qset_json: &serde_json::Value,
        from_str_key: &F,
    ) -> Result<thruput_scp::SCPQuorumSet, String>
    where
        F: Fn(&str) -> Result<thruput_scp::SCPNodeID, String>,
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
            _ => {
                return Err(
                    "JSON field 't' must exist and be an unsigned integer"
                        .to_string(),
                );
            }
        };

        // Check for required validators array field
        let entries = match obj.get("v") {
            Some(v) if v.is_array() => v.as_array().unwrap(),
            _ => {
                return Err(
                    "JSON field 'v' must exist and be an array".to_string()
                );
            }
        };

        let mut validators = Vec::new();
        let mut inner_sets = Vec::new();

        for entry in entries {
            if entry.is_string() {
                // Parse validator key string
                let key_str = entry.as_str().unwrap();
                match from_str_key(key_str) {
                    Ok(node_id) => validators.push(node_id),
                    Err(e) => {
                        return Err(format!(
                            "Invalid validator key in 'v' array: {}",
                            e
                        ));
                    }
                }
            } else if entry.is_object() {
                // Recursively parse inner quorum set
                inner_sets
                    .push(Self::from_json_with_parser(entry, from_str_key)?);
            } else {
                return Err("Each entry in 'v' must be either a string (validator key) or an object (inner quorum set)".to_string());
            }
        }

        Ok(thruput_scp::SCPQuorumSet {
            threshold,
            validators: VecM(validators),
            inner_sets: VecM(inner_sets),
        })
    }

    pub fn to_string(&self, qset: &thruput_scp::SCPQuorumSet) -> String {
        let json = self.to_json(qset, false);
        serde_json::to_string(&json).unwrap()
    }
}
