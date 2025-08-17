use crate::*;
use std::collections::{HashMap, HashSet};
use tracing::{debug, error, instrument, trace};
use xdr_common::VecM;

#[derive(Debug)]
pub struct LocalState {
    node_id: proto::SCPNodeID,
    is_validator: bool,
    qset: proto::SCPQuorumSet,
    qset_hash: proto::SCPHash,
    single_qset: Option<proto::SCPQuorumSet>,
    single_qset_hash: Option<proto::SCPHash>,
}

impl LocalState {
    /// Constructor equivalent to C++ LocalNode::LocalNode
    /// Parameters:
    /// - node_id: The node ID for this local node
    /// - is_validator: Whether this node is a validator
    /// - qset: The quorum set for this node
    /// Note: SCPDriver parameter is ignored as requested
    pub fn new(
        node_id: proto::SCPNodeID,
        is_validator: bool,
        qset: proto::SCPQuorumSet,
        get_hash_of: impl Fn(&[u8]) -> proto::SCPHash,
    ) -> LocalState {
        // Normalize the quorum set (equivalent to normalizeQSet(mQSet) in C++)
        let mut normalized_qset = qset;
        crate::qset::normalize_qset(&mut normalized_qset, None);

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
pub struct LocalProto<'a, 'b, C: Consensus, D: Driver> {
    pub cd: &'a ConsensusDriver<'b, C, D>,
    pub local: &'a LocalState,
}

#[derive(Debug)]
pub struct LocalProtoMut<'a, 'b, C: Consensus, D: Driver> {
    pub cd: &'a ConsensusDriver<'b, C, D>,
    pub local: &'a mut LocalState,
}

impl<'a, 'b, C: Consensus, D: Driver> LocalProtoMut<'a, 'b, C, D> {
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

impl<'a, 'b, C: Consensus, D: Driver> LocalProto<'a, 'b, C, D> {
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

    pub fn for_all_nodes<F>(qset: &proto::SCPQuorumSet, mut proc: F) -> bool
    where
        F: FnMut(&proto::SCPNodeID) -> bool,
    {
        for node in qset.validators.iter() {
            if !proc(node) {
                return false;
            }
        }

        for inner_qset in qset.inner_sets.iter() {
            if !Self::for_all_nodes(inner_qset, &mut proc) {
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

    fn is_quorum_slice(qset: &proto::SCPQuorumSet, node_set: &[proto::SCPNodeID]) -> bool {
        Self::is_quorum_slice_internal(qset, node_set)
    }

    pub fn is_vblocking_nodes(qset: &proto::SCPQuorumSet, nodes: &Vec<proto::SCPNodeID>) -> bool {
        panic!("XXX")
    }

    pub fn is_vblocking_envelopes<F>(
        qset: &proto::SCPQuorumSet,
        envs: &HashMap<proto::SCPNodeID, proto::SCPEnvelope>,
        filter: F,
    ) -> bool
    where
        F: Fn(&proto::SCPStatement) -> bool,
    {
        panic!("XXX")
    }

    pub fn is_quorum(
        qset: &proto::SCPQuorumSet,
        envelopes: &HashMap<proto::SCPNodeID, proto::SCPEnvelope>,
        qfun: impl Fn(&proto::SCPStatement) -> Option<proto::SCPQuorumSet>,
        filter: impl Fn(&proto::SCPStatement) -> bool,
    ) -> bool {
        panic!("XXX")
    }

    // computes the distance to the set of v-blocking sets given
    // a set of nodes that agree (but can fail)
    // excluded, if set will be skipped altogether
    pub fn find_closest_v_blocking_nodes(
        &self,
        qset: &proto::SCPQuorumSet,
        nodes: &HashSet<proto::SCPNodeID>,
        excluded: Option<&proto::SCPNodeID>,
    ) -> Vec<&proto::SCPNodeID> {
        panic!("XXX")
    }

    pub fn find_closest_v_blocking_envelopes(
        &self,
        qset: &proto::SCPQuorumSet,
        envs: &HashMap<proto::SCPNodeID, proto::SCPEnvelope>,
        filter: impl Fn(&proto::SCPStatement) -> bool,
        excluded: Option<&proto::SCPNodeID>,
    ) -> Vec<&proto::SCPNodeID> {
        panic!("XXX")
    }

    pub fn to_json(&self, qset: &proto::SCPQuorumSet, full_keys: bool) -> serde_json::Value {
        panic!("XXX")
    }
}
