#[cfg(test)]
mod unit_tests {
    use crate::testkernel::*;
    use crate::testutil::*;
    use crate::*;
    use shfix::shfix;
    use std::collections::HashMap;
    use std::u32;
    use xdr_common::VecM;

    fn is_near(r: u64, target: f64) -> bool {
        let v = r as f64 / u64::MAX as f64;
        (v - target).abs() < 0.01
    }

    #[test]
    fn nomination_weight() {
        let nodes = TestNode::new_batch(6);
        let mut q_set = SCPQuorumSet {
            threshold: 3,
            validators: VecM(vec![
                nodes[0].node_id().clone(),
                nodes[1].node_id().clone(),
                nodes[2].node_id().clone(),
                nodes[3].node_id().clone(),
            ]),
            inner_sets: VecM(vec![]),
        };
        normalize_qset(&mut q_set, None);

        let mut uk = UnitTestKernel::new(nodes[0].node_id().clone(), q_set.clone(), true);

        let result = uk
            .proto()
            .cd
            .driver
            .get_node_weight(&nodes[2].node_id(), &q_set, false);
        assert!(is_near(result, 0.75));

        let result = uk
            .proto()
            .cd
            .driver
            .get_node_weight(&nodes[4].node_id(), &q_set, false);
        assert_eq!(result, 0);

        let inner_qset = SCPQuorumSet {
            threshold: 1,
            validators: VecM(vec![nodes[4].node_id().clone(), nodes[5].node_id().clone()]),
            inner_sets: VecM(vec![]),
        };
        q_set.inner_sets.0.push(inner_qset);
        normalize_qset(&mut q_set, None);

        let result = uk
            .proto()
            .cd
            .driver
            .get_node_weight(&nodes[4].node_id(), &q_set, false);
        assert!(is_near(result, 0.6 * 0.5));
    }

    fn make_qset(
        node_ids: &[proto::SCPNodeID],
        threshold: u32,
        total: usize,
        offset: usize,
    ) -> proto::SCPQuorumSet {
        let mut validators = Vec::new();
        for i in 0..total {
            validators.push(node_ids[i + offset].clone());
        }

        proto::SCPQuorumSet {
            threshold,
            validators: VecM(validators),
            inner_sets: VecM(vec![]),
        }
    }

    #[shfix]
    #[test]
    fn nomination_weight_stats() {
        let nodes = TestNode::new_batch(7);

        let node_ids: Vec<proto::SCPNodeID> = nodes.iter().map(|n| n.node_id()).collect();

        let total_slots = 1000;
        let max_round_per_slot = 5; // 5 -> 15 seconds
        let total_rounds = total_slots * max_round_per_slot;

        let run_tests = |qset: &proto::SCPQuorumSet| -> HashMap<proto::SCPNodeID, i32> {
            let mut wins = HashMap::<proto::SCPNodeID, i32>::new();

            let mut uk = UnitTestKernel::new(node_ids[0].clone(), qset.clone(), true);

            for s in 0..total_slots {
                let mut value = proto::SCPValue::default();
                value.0.push(s as u8); // anything will do as a value

                let mut proto = uk.proto();
                let mut slot = proto.slot_proto_or_create(s);
                let mut nom_proto = slot.nomination_proto();
                nom_proto.dbg_set_previous_value(&value);

                for i in 0..max_round_per_slot {
                    nom_proto.dbg_set_round_number(i.try_into().unwrap());
                    nom_proto.update_round_leaders();

                    let leaders = nom_proto.dbg_get_round_leaders();
                    assert!(!leaders.is_empty());

                    for w in leaders {
                        *wins.entry(w.clone()).or_insert(0) += 1;
                    }
                }
            }
            wins
        };

        // 274
    }
}
