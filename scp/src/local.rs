use crate::*;
use std::collections::HashMap;

pub struct Local {}

impl Local {
    pub fn get_quorum_hash(&self) -> &proto::SCPHash {
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

    pub fn is_vblocking_envs<F>(
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
}
