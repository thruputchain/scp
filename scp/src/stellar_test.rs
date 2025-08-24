use crate::testkernel::*;
use crate::testutil::*;
#[cfg(test)]
use crate::*;
use std::collections::{HashMap, HashSet};
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

#[test]
fn v_blocking_distance() {
    let nodes = TestNode::new_batch(8);
    let v0_node_id = nodes[0].node_id();
    let v1_node_id = nodes[1].node_id();
    let v2_node_id = nodes[2].node_id();
    let v3_node_id = nodes[3].node_id();
    let v4_node_id = nodes[4].node_id();
    let v5_node_id = nodes[5].node_id();
    let v6_node_id = nodes[6].node_id();
    let v7_node_id = nodes[7].node_id();

    let mut q_set = SCPQuorumSet {
        threshold: 2,
        validators: VecM(vec![
            v0_node_id.clone(),
            v1_node_id.clone(),
            v2_node_id.clone(),
        ]),
        inner_sets: VecM(vec![]),
    };

    let check = |q_set_check: &SCPQuorumSet, s: &HashSet<SCPNodeID>, expected: usize| {
        let r = LocalProto::<TestH, TestH>::find_closest_v_blocking_nodes(q_set_check, s, None);
        assert_eq!(expected, r.len());
    };

    let mut good = HashSet::new();
    good.insert(v0_node_id.clone());

    // already v-blocking
    check(&q_set, &good, 0);

    good.insert(v1_node_id.clone());
    // either v0 or v1
    check(&q_set, &good, 1);

    good.insert(v2_node_id.clone());
    // any 2 of v0..v2
    check(&q_set, &good, 2);

    // Create subset 1
    let q_sub_set1 = SCPQuorumSet {
        threshold: 1,
        validators: VecM(vec![
            v3_node_id.clone(),
            v4_node_id.clone(),
            v5_node_id.clone(),
        ]),
        inner_sets: VecM(vec![]),
    };
    q_set.inner_sets.0.push(q_sub_set1);

    good.insert(v3_node_id.clone());
    // any 3 of v0..v3
    check(&q_set, &good, 3);

    good.insert(v4_node_id.clone());
    // v0..v2
    check(&q_set, &good, 3);

    q_set.threshold = 1;
    // v0..v4
    check(&q_set, &good, 5);

    good.insert(v5_node_id.clone());
    // v0..v5
    check(&q_set, &good, 6);

    // Create subset 2
    let q_sub_set2 = SCPQuorumSet {
        threshold: 2,
        validators: VecM(vec![v6_node_id.clone(), v7_node_id.clone()]),
        inner_sets: VecM(vec![]),
    };
    q_set.inner_sets.0.push(q_sub_set2);

    // v0..v5
    check(&q_set, &good, 6);

    good.insert(v6_node_id.clone());
    // v0..v5
    check(&q_set, &good, 6);

    good.insert(v7_node_id.clone());
    // v0..v5 and one of 6,7
    check(&q_set, &good, 7);

    q_set.threshold = 4;
    // v6, v7
    check(&q_set, &good, 2);

    q_set.threshold = 3;
    // v0..v2
    check(&q_set, &good, 3);

    q_set.threshold = 2;
    // v0..v2 and one of v6,v7
    check(&q_set, &good, 4);
}

type GenEnvelope = Box<dyn Fn(&proto::SecretKey) -> proto::SCPEnvelope>;

fn make_prepare_gen(
    qset_hash: proto::SCPHash,
    ballot: proto::SCPBallot,
    prepared: Option<proto::SCPBallot>,
    n_c: proto::SCPCount,
    n_h: proto::SCPCount,
    prepared_prime: Option<proto::SCPBallot>,
) -> GenEnvelope {
    Box::new(move |secret_key: &proto::SecretKey| {
        make_prepare(
            secret_key,
            qset_hash,
            0, // slot_index
            ballot.clone(),
            prepared.clone(),
            n_c,
            n_h,
            prepared_prime.clone(),
            derive_public_key(secret_key),
        )
        .unwrap()
    })
}

fn make_confirm_gen(
    qset_hash: proto::SCPHash,
    prepare_counter: proto::SCPCount,
    ballot: proto::SCPBallot,
    n_c: proto::SCPCount,
    n_h: proto::SCPCount,
) -> GenEnvelope {
    Box::new(move |secret_key: &proto::SecretKey| {
        make_confirm(
            secret_key,
            qset_hash,
            0, // slot_index
            prepare_counter,
            ballot.clone(),
            n_c,
            n_h,
            derive_public_key(secret_key),
        )
        .unwrap()
    })
}

fn make_externalize_gen(
    qset_hash: proto::SCPHash,
    commit_ballot: proto::SCPBallot,
    n_h: proto::SCPCount,
) -> GenEnvelope {
    Box::new(move |secret_key: &proto::SecretKey| {
        make_externalize(
            secret_key,
            qset_hash,
            0, // slot_index
            commit_ballot.clone(),
            n_h,
            derive_public_key(secret_key),
        )
        .unwrap()
    })
}
