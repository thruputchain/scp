use crate::testkernel::*;
use crate::testutil::*;
#[cfg(test)]
use crate::*;
use proto::*;
use sha2::{Digest, Sha256};
use xdr_common::VecM;

#[test]
fn test_vblocking_and_quorum() {
    let values = TestValues::default();
    let nodes = TestNode::new_batch(4);

    let qset = SCPQuorumSet {
        threshold: 3,
        validators: VecM(vec![
            nodes[0].node_id(),
            nodes[1].node_id(),
            nodes[2].node_id(),
            nodes[3].node_id(),
        ]),
        inner_sets: VecM(vec![]),
    };

    let mut node_set = Vec::new();
    node_set.push(nodes[0].node_id());

    assert_eq!(
        LocalProto::<TestH, TestH>::is_quorum_slice(&qset, &node_set),
        false,
    );
    assert_eq!(
        LocalProto::<TestH, TestH>::is_vblocking_nodes(&qset, &node_set),
        false,
    );

    node_set.push(nodes[2].node_id());

    assert_eq!(
        LocalProto::<TestH, TestH>::is_quorum_slice(&qset, &node_set),
        false,
    );
    assert_eq!(
        LocalProto::<TestH, TestH>::is_vblocking_nodes(&qset, &node_set),
        true,
    );

    node_set.push(nodes[3].node_id());

    assert_eq!(
        LocalProto::<TestH, TestH>::is_quorum_slice(&qset, &node_set),
        true,
    );
    assert_eq!(
        LocalProto::<TestH, TestH>::is_vblocking_nodes(&qset, &node_set),
        true,
    );

    node_set.push(nodes[1].node_id());

    assert_eq!(
        LocalProto::<TestH, TestH>::is_quorum_slice(&qset, &node_set),
        true,
    );
    assert_eq!(
        LocalProto::<TestH, TestH>::is_vblocking_nodes(&qset, &node_set),
        true,
    );
}
