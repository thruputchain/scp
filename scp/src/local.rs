use crate::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct Local {
    node_id: proto::SCPNodeID,
    is_validator: bool,
    qset: proto::SCPQuorumSet,
    qset_hash: proto::SCPHash,
    single_qset: Option<proto::SCPQuorumSet>,
    single_qset_hash: Option<proto::SCPHash>,
}

impl Local {
    pub fn new() -> Local {
        panic!("XXX")
    }
}

#[derive(Debug)]
pub struct LocalProto<'a, 'b, C: Consensus, D: Driver> {
    pub cd: &'a ConsensusDriver<'b, C, D>,
    pub local: &'a Local,
}

#[derive(Debug)]
pub struct LocalProtoMut<'a, 'b, C: Consensus, D: Driver> {
    pub cd: &'a ConsensusDriver<'b, C, D>,
    pub local: &'a mut Local,
}

impl<'a, 'b, C: Consensus, D: Driver> LocalProtoMut<'a, 'b, C, D> {}

impl<'a, 'b, C: Consensus, D: Driver> LocalProto<'a, 'b, C, D> {
    pub fn get_quorum_set(&self) -> &proto::SCPQuorumSet {
        panic!("XXX")
    }

    pub fn get_quorum_set_hash(&self) -> &proto::SCPHash {
        panic!("XXX")
    }

    pub fn get_node_id(&self) -> &proto::SCPNodeID {
        panic!("XXX")
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
