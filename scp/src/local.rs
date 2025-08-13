use crate::*;

pub struct Local {}

impl Local {
    pub fn get_quorum_hash(&self) -> &proto::SCPHash {
        panic!("XXX")
    }

    pub fn get_node_id(&self) -> &proto::SCPNodeID {
        panic!("XXX")
    }
}
