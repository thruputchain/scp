use crate::*;
use std::collections::{HashMap, HashSet};

pub struct Local {}

impl Local {
    pub fn get_quorum_set(&self) -> &proto::SCPQuorumSet {
        panic!("XXX")
    }

    pub fn get_quorum_set_hash(&self) -> &proto::SCPHash {
        panic!("XXX")
    }

    pub fn get_node_id(&self) -> &proto::SCPNodeID {
        panic!("XXX")
    }

    pub fn is_vblocking_nodes(
        &self,
        qset: &proto::SCPQuorumSet,
        nodes: &Vec<proto::SCPNodeID>,
    ) -> bool {
        panic!("XXX")
    }

    pub fn is_vblocking_envelopes<F>(
        &self,
        qset: &proto::SCPQuorumSet,
        envs: &HashMap<proto::SCPNodeID, proto::SCPEnvelope>,
        filter: F,
    ) -> bool
    where
        F: Fn(&proto::SCPStatement) -> bool,
    {
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
