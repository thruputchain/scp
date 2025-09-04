#[cfg(test)]
mod unit_tests {
    use crate::testkernel::*;
    use crate::testutil::*;
    use crate::*;
    use shfix::shfix;
    use std::collections::HashSet;
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
}
