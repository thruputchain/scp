#[cfg(test)]
mod stellar_tests {
    use crate::testkernel::*;
    use crate::testutil::*;
    use crate::*;
    use shfix::shfix;
    use std::collections::HashSet;
    use std::u32;
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
            )
            .unwrap()
        })
    }

    #[derive(Clone)]
    struct NetworkConfig {
        NOMINATION_TIMEOUT_INITIAL_MILLISECONDS: u32,
        NOMINATION_TIMEOUT_INCREMENT_MILLISECONDS: u32,
        BALLOT_TIMEOUT_INITIAL_MILLISECONDS: u32,
        BALLOT_TIMEOUT_INCREMENT_MILLISECONDS: u32,
    }

    impl NetworkConfig {
        fn apply_to(&self, tk: &mut TestKernel) {
            tk.driver().initial_nomination_timeout_ms =
                self.NOMINATION_TIMEOUT_INITIAL_MILLISECONDS;
            tk.driver().initial_ballot_timeout_ms = self.BALLOT_TIMEOUT_INITIAL_MILLISECONDS;
            tk.driver().increment_nomination_timeout_ms =
                self.NOMINATION_TIMEOUT_INCREMENT_MILLISECONDS;
            tk.driver().increment_ballot_timeout_ms = self.BALLOT_TIMEOUT_INCREMENT_MILLISECONDS;
        }
    }

    fn get_network_cfgs() -> Vec<NetworkConfig> {
        const MIN_CFG: NetworkConfig = NetworkConfig {
            NOMINATION_TIMEOUT_INITIAL_MILLISECONDS: 750,
            NOMINATION_TIMEOUT_INCREMENT_MILLISECONDS: 750,
            BALLOT_TIMEOUT_INITIAL_MILLISECONDS: 750,
            BALLOT_TIMEOUT_INCREMENT_MILLISECONDS: 750,
        };

        const MAX_CFG: NetworkConfig = NetworkConfig {
            NOMINATION_TIMEOUT_INITIAL_MILLISECONDS: 2500,
            NOMINATION_TIMEOUT_INCREMENT_MILLISECONDS: 2000,
            BALLOT_TIMEOUT_INITIAL_MILLISECONDS: 2500,
            BALLOT_TIMEOUT_INCREMENT_MILLISECONDS: 2000,
        };

        const INIT_CFG: NetworkConfig = NetworkConfig {
            NOMINATION_TIMEOUT_INITIAL_MILLISECONDS: 1000,
            NOMINATION_TIMEOUT_INCREMENT_MILLISECONDS: 1000,
            BALLOT_TIMEOUT_INITIAL_MILLISECONDS: 1000,
            BALLOT_TIMEOUT_INCREMENT_MILLISECONDS: 1000,
        };
        vec![MIN_CFG.clone(), MAX_CFG.clone(), INIT_CFG.clone()]
    }

    #[shfix]
    #[test]
    fn ballot_protocol_core5() {
        let test = |net_cfg: &NetworkConfig| {
            let values = TestValues::default();
            let nodes = TestNode::new_batch(5);
            let mut q_set = SCPQuorumSet {
                threshold: 4,
                validators: VecM(vec![
                    nodes[0].node_id().clone(),
                    nodes[1].node_id().clone(),
                    nodes[2].node_id().clone(),
                    nodes[3].node_id().clone(),
                    nodes[4].node_id().clone(),
                ]),
                inner_sets: VecM(vec![]),
            };
            normalize_qset(&mut q_set, None); // NOTE: LocalProto internally normalizes qsets! This ensures parity.
            let qset_hash = proto::hash_xdr(&q_set).unwrap();
            let mut tk = TestKernel::new(nodes[0].node_id().clone(), q_set.clone(), true);
            let v0_secret_key = nodes[0].secret_key.clone();
            let v1_secret_key = nodes[1].secret_key.clone();
            let v2_secret_key = nodes[2].secret_key.clone();
            let v3_secret_key = nodes[3].secret_key.clone();
            let v4_secret_key = nodes[4].secret_key.clone();
            let v0_node_id = nodes[0].node_id();
            let v1_node_id = nodes[1].node_id();
            let v2_node_id = nodes[2].node_id();
            let v3_node_id = nodes[3].node_id();
            let v4_node_id = nodes[4].node_id();
            net_cfg.apply_to(&mut tk);
            //
            tk.driver().store_quorum_set(&q_set);
            let qset_hash0 = tk.proto().local_proto().get_qset_hash().clone();

            // Verify value ordering
            assert!(values.x_value < values.y_value);
            assert!(values.y_value < values.z_value);
            assert!(values.z_value < values.zz_value);

            let mut recv_v_blocking_checks =
                |tk: &mut TestKernel, gen_: &GenEnvelope, with_checks: bool| {
                    let e1 = gen_(&v1_secret_key);
                    let e2 = gen_(&v2_secret_key);

                    tk.driver().bump_timer_offset();

                    // Nothing should happen with first message
                    let i = tk.driver().envelopes.len();
                    tk.proto().received_envelope(&e1);
                    if with_checks {
                        assert_eq!(tk.driver().envelopes.len(), i);
                    }
                    let i = i + 1;
                    tk.proto().received_envelope(&e2);
                    if with_checks {
                        assert_eq!(tk.driver().envelopes.len(), i);
                    }
                };

            let mut recv_v_blocking =
                |tk: &mut TestKernel, gen_: &GenEnvelope| recv_v_blocking_checks(tk, gen_, true);

            let mut recv_quorum_checks_ex =
                |tk: &mut TestKernel,
                 gen_: &GenEnvelope,
                 with_checks: bool,
                 delayed_quorum: bool,
                 check_upcoming: bool| {
                    let e1 = gen_(&v1_secret_key);
                    let e2 = gen_(&v2_secret_key);
                    let e3 = gen_(&v3_secret_key);
                    let e4 = gen_(&v4_secret_key);

                    tk.driver().bump_timer_offset();

                    tk.proto().received_envelope(&e1);
                    tk.proto().received_envelope(&e2);
                    let i = tk.driver().envelopes.len() + 1;
                    tk.proto().received_envelope(&e3);
                    if with_checks && !delayed_quorum {
                        assert_eq!(tk.driver().envelopes.len(), i);
                    }
                    if check_upcoming && !delayed_quorum {
                        tk.driver().has_ballot_timer_upcoming();
                    }
                    // Nothing happens with an extra vote (unless we're in delayed_quorum)
                    tk.proto().received_envelope(&e4);
                    if with_checks && delayed_quorum {
                        assert_eq!(tk.driver().envelopes.len(), i);
                    }
                    if check_upcoming && delayed_quorum {
                        tk.driver().has_ballot_timer_upcoming();
                    }
                };

            let mut recv_quorum_checks =
                |tk: &mut TestKernel,
                 gen_: &GenEnvelope,
                 with_checks: bool,
                 delayed_quorum: bool| {
                    recv_quorum_checks_ex(tk, gen_, with_checks, delayed_quorum, false)
                };

            let mut recv_quorum_ex =
                |tk: &mut TestKernel, gen_: &GenEnvelope, check_upcoming: bool| {
                    recv_quorum_checks_ex(tk, gen_, true, false, check_upcoming)
                };

            let mut recv_quorum =
                |tk: &mut TestKernel, gen_: &GenEnvelope| recv_quorum_ex(tk, gen_, false);

            let mut nodes_all_pledge_to_commit = |tk: &mut TestKernel| {
                let b = SCPBallot {
                    counter: 1,
                    value: values.x_value.clone(),
                };

                let prepare1 = make_prepare(
                    &v1_secret_key,
                    qset_hash.into_inner(),
                    0,
                    b.clone(),
                    None,
                    0,
                    0,
                    None,
                )
                .unwrap();
                let prepare2 = make_prepare(
                    &v2_secret_key,
                    qset_hash.into_inner(),
                    0,
                    b.clone(),
                    None,
                    0,
                    0,
                    None,
                )
                .unwrap();
                let prepare3 = make_prepare(
                    &v3_secret_key,
                    qset_hash.into_inner(),
                    0,
                    b.clone(),
                    None,
                    0,
                    0,
                    None,
                )
                .unwrap();
                let prepare4 = make_prepare(
                    &v4_secret_key,
                    qset_hash.into_inner(),
                    0,
                    b.clone(),
                    None,
                    0,
                    0,
                    None,
                )
                .unwrap();

                assert!(tk.bump_state(0, &values.x_value));
                assert_eq!(tk.driver().envelopes.len(), 1);

                verify_prepare(
                    &tk.driver().envelopes[0],
                    &v0_secret_key,
                    qset_hash0,
                    0,
                    b.clone(),
                    None,
                    0,
                    0,
                    None,
                );

                tk.proto().received_envelope(&prepare1);
                assert_eq!(tk.driver().envelopes.len(), 1);
                assert_eq!(tk.driver().heard_from_quorums.get(&0), None);

                tk.proto().received_envelope(&prepare2);
                assert_eq!(tk.driver().envelopes.len(), 1);
                assert_eq!(tk.driver().heard_from_quorums.get(&0), None);

                tk.proto().received_envelope(&prepare3);
                assert_eq!(tk.driver().envelopes.len(), 2);
                assert_eq!(tk.driver().heard_from_quorums.get(&0).unwrap().len(), 1);
                assert_eq!(tk.driver().heard_from_quorums.get(&0).unwrap()[0], b);

                // We have a quorum including us
                verify_prepare(
                    &tk.driver().envelopes[1],
                    &v0_secret_key,
                    qset_hash0,
                    0,
                    b.clone(),
                    Some(b.clone()),
                    0,
                    0,
                    None,
                );

                tk.proto().received_envelope(&prepare4);
                assert_eq!(tk.driver().envelopes.len(), 2);

                let prepared1 = make_prepare(
                    &v1_secret_key,
                    qset_hash.into_inner(),
                    0,
                    b.clone(),
                    Some(b.clone()),
                    0,
                    0,
                    None,
                )
                .unwrap();
                let prepared2 = make_prepare(
                    &v2_secret_key,
                    qset_hash.into_inner(),
                    0,
                    b.clone(),
                    Some(b.clone()),
                    0,
                    0,
                    None,
                )
                .unwrap();
                let prepared3 = make_prepare(
                    &v3_secret_key,
                    qset_hash.into_inner(),
                    0,
                    b.clone(),
                    Some(b.clone()),
                    0,
                    0,
                    None,
                )
                .unwrap();
                let prepared4 = make_prepare(
                    &v4_secret_key,
                    qset_hash.into_inner(),
                    0,
                    b.clone(),
                    Some(b.clone()),
                    0,
                    0,
                    None,
                )
                .unwrap();

                tk.proto().received_envelope(&prepared4);
                tk.proto().received_envelope(&prepared3);
                assert_eq!(tk.driver().envelopes.len(), 2);

                tk.proto().received_envelope(&prepared2);
                assert_eq!(tk.driver().envelopes.len(), 3);

                // confirms prepared
                verify_prepare(
                    &tk.driver().envelopes[2],
                    &v0_secret_key,
                    qset_hash0,
                    0,
                    b.clone(),
                    Some(b.clone()),
                    b.counter,
                    b.counter,
                    None,
                );

                // extra statement doesn't do anything
                tk.proto().received_envelope(&prepared1);
                assert_eq!(tk.driver().envelopes.len(), 3);
            };

            section!(bump_state_x, "bump_state x", {
                assert!(tk.bump_state(0, &values.x_value.clone()));
                assert_eq!(tk.driver().envelopes.len(), 1);

                let expected_ballot = SCPBallot {
                    counter: 1,
                    value: values.x_value.clone(),
                };

                verify_prepare(
                    &tk.driver().envelopes[0],
                    &v0_secret_key,
                    qset_hash0,
                    0,
                    expected_ballot,
                    None,
                    0,
                    0,
                    None,
                );
            });

            section!(start_1x, "start <1,x>", {
                // No timer should be set initially
                assert!(!tk.driver().has_ballot_timer());

                let a_value = &values.x_value;
                let b_value = &values.z_value;
                let mid_value = &values.y_value;
                let big_value = &values.zz_value;

                let a1 = SCPBallot {
                    counter: 1,
                    value: a_value.clone(),
                };
                let b1 = SCPBallot {
                    counter: 1,
                    value: b_value.clone(),
                };
                let mid1 = SCPBallot {
                    counter: 1,
                    value: mid_value.clone(),
                };
                let big1 = SCPBallot {
                    counter: 1,
                    value: big_value.clone(),
                };

                let mut a2 = a1.clone();
                a2.counter += 1;

                let mut a3 = a2.clone();
                a3.counter += 1;

                let mut a4 = a3.clone();
                a4.counter += 1;

                let mut a5 = a4.clone();
                a5.counter += 1;

                let a_inf = SCPBallot {
                    counter: u32::MAX,
                    value: a_value.clone(),
                };
                let b_inf = SCPBallot {
                    counter: u32::MAX,
                    value: b_value.clone(),
                };

                let mut b2 = b1.clone();
                b2.counter += 1;

                let mut b3 = b2.clone();
                b3.counter += 1;

                let mut mid2 = mid1.clone();
                mid2.counter += 1;

                let mut big2 = big1.clone();
                big2.counter += 1;

                assert!(tk.bump_state(0, a_value));
                assert_eq!(tk.driver().envelopes.len(), 1);
                assert!(!tk.driver().has_ballot_timer());

                section!(prep_a1, "prepared A1", {
                    recv_quorum_ex(
                        &mut tk,
                        &make_prepare_gen(qset_hash.into_inner(), a1.clone(), None, 0, 0, None),
                        true,
                    );

                    assert_eq!(tk.driver().envelopes.len(), 2);
                    verify_prepare(
                        &tk.driver().envelopes[1],
                        &v0_secret_key,
                        qset_hash0,
                        0,
                        a1.clone(),
                        Some(a1.clone()),
                        0,
                        0,
                        None,
                    );

                    section!(prep_a2, "bump prepared A2", {
                        // bump to (2,a)

                        tk.driver().bump_timer_offset();
                        assert!(tk.bump_state(0, a_value));
                        assert_eq!(tk.driver().envelopes.len(), 3);
                        verify_prepare(
                            &tk.driver().envelopes[2],
                            &v0_secret_key,
                            qset_hash0,
                            0,
                            a2.clone(),
                            Some(a1.clone()),
                            0,
                            0,
                            None,
                        );
                        assert!(!tk.driver().has_ballot_timer());

                        recv_quorum_ex(
                            &mut tk,
                            &make_prepare_gen(qset_hash.into_inner(), a2.clone(), None, 0, 0, None),
                            true,
                        );
                        assert_eq!(tk.driver().envelopes.len(), 4);
                        verify_prepare(
                            &tk.driver().envelopes[3],
                            &v0_secret_key,
                            qset_hash0,
                            0,
                            a2.clone(),
                            Some(a2.clone()),
                            0,
                            0,
                            None,
                        );

                        section!(confirm_prep_a2, "Confirm prepared A2", {
                            recv_quorum(
                                &mut tk,
                                &make_prepare_gen(
                                    qset_hash.into_inner(),
                                    a2.clone(),
                                    Some(a2.clone()),
                                    0,
                                    0,
                                    None,
                                ),
                            );
                            assert_eq!(tk.driver().envelopes.len(), 5);
                            verify_prepare(
                                &tk.driver().envelopes[4],
                                &v0_secret_key,
                                qset_hash0,
                                0,
                                a2.clone(),
                                Some(a2.clone()),
                                2,
                                2,
                                None,
                            );
                            assert!(!tk.driver().has_ballot_timer_upcoming());

                            section!(accept_commit, "Accept commit", {
                                section!(quorum_a2, "Quorum A2", {
                                    recv_quorum(
                                        &mut tk,
                                        &make_prepare_gen(
                                            qset_hash.into_inner(),
                                            a2.clone(),
                                            Some(a2.clone()),
                                            2,
                                            2,
                                            None,
                                        ),
                                    );
                                    assert_eq!(tk.driver().envelopes.len(), 6);
                                    verify_confirm(
                                        &tk.driver().envelopes[5],
                                        &v0_secret_key,
                                        qset_hash0,
                                        0,
                                        2,
                                        a2.clone(),
                                        2,
                                        2,
                                    );
                                    assert!(!tk.driver().has_ballot_timer_upcoming());

                                    section!(quorum_prep_a3, "Quorum prepared A3", {
                                        recv_v_blocking(
                                            &mut tk,
                                            &make_prepare_gen(
                                                qset_hash.into_inner(),
                                                a3.clone(),
                                                Some(a2.clone()),
                                                2,
                                                2,
                                                None,
                                            ),
                                        );
                                        assert_eq!(tk.driver().envelopes.len(), 7);
                                        verify_confirm(
                                            &tk.driver().envelopes[6],
                                            &v0_secret_key,
                                            qset_hash0,
                                            0,
                                            2,
                                            a3.clone(),
                                            2,
                                            2,
                                        );
                                        assert!(!tk.driver().has_ballot_timer());

                                        recv_quorum_ex(
                                            &mut tk,
                                            &make_prepare_gen(
                                                qset_hash.into_inner(),
                                                a3.clone(),
                                                Some(a2.clone()),
                                                2,
                                                2,
                                                None,
                                            ),
                                            true,
                                        );
                                        assert_eq!(tk.driver().envelopes.len(), 8);
                                        verify_confirm(
                                            &tk.driver().envelopes[7],
                                            &v0_secret_key,
                                            qset_hash0,
                                            0,
                                            3,
                                            a3.clone(),
                                            2,
                                            2,
                                        );

                                        section!(accept_more_commit_a3, "Accept more commit A3", {
                                            recv_quorum(
                                                &mut tk,
                                                &make_prepare_gen(
                                                    qset_hash.into_inner(),
                                                    a3.clone(),
                                                    Some(a3.clone()),
                                                    2,
                                                    3,
                                                    None,
                                                ),
                                            );
                                            assert_eq!(tk.driver().envelopes.len(), 9);
                                            verify_confirm(
                                                &tk.driver().envelopes[8],
                                                &v0_secret_key,
                                                qset_hash0,
                                                0,
                                                3,
                                                a3.clone(),
                                                2,
                                                3,
                                            );
                                            assert!(!tk.driver().has_ballot_timer_upcoming());

                                            assert_eq!(tk.driver().externalized_values.len(), 0);

                                            section!(
                                                quorum_externalize_a3,
                                                "Quorum externalize A3",
                                                {
                                                    recv_quorum(
                                                        &mut tk,
                                                        &make_confirm_gen(
                                                            qset_hash.into_inner(),
                                                            3,
                                                            a3.clone(),
                                                            2,
                                                            3,
                                                        ),
                                                    );
                                                    assert_eq!(tk.driver().envelopes.len(), 10);
                                                    verify_externalize(
                                                        &tk.driver().envelopes[9],
                                                        &v0_secret_key,
                                                        qset_hash0,
                                                        0,
                                                        a2.clone(),
                                                        3,
                                                    );
                                                    assert!(!tk.driver().has_ballot_timer());

                                                    assert_eq!(
                                                        tk.driver().externalized_values.len(),
                                                        1
                                                    );
                                                    assert_eq!(
                                                        tk.driver()
                                                            .externalized_values
                                                            .get(&0)
                                                            .unwrap(),
                                                        a_value
                                                    );
                                                }
                                            );
                                        });

                                        section!(
                                            vblocking_accept_more_a3,
                                            "v-blocking accept more A3",
                                            {
                                                section!(confirm_a3, "Confirm A3", {
                                                    recv_v_blocking(
                                                        &mut tk,
                                                        &make_confirm_gen(
                                                            qset_hash.into_inner(),
                                                            3,
                                                            a3.clone(),
                                                            2,
                                                            3,
                                                        ),
                                                    );
                                                    assert_eq!(tk.driver().envelopes.len(), 9);
                                                    verify_confirm(
                                                        &tk.driver().envelopes[8],
                                                        &v0_secret_key,
                                                        qset_hash0,
                                                        0,
                                                        3,
                                                        a3.clone(),
                                                        2,
                                                        3,
                                                    );
                                                    assert!(
                                                        !tk.driver().has_ballot_timer_upcoming()
                                                    );
                                                });

                                                section!(externalize_a3, "Externalize A3", {
                                                    recv_v_blocking(
                                                        &mut tk,
                                                        &make_externalize_gen(
                                                            qset_hash.into_inner(),
                                                            a2.clone(),
                                                            3,
                                                        ),
                                                    );
                                                    assert_eq!(tk.driver().envelopes.len(), 9);
                                                    verify_confirm(
                                                        &tk.driver().envelopes[8],
                                                        &v0_secret_key,
                                                        qset_hash0,
                                                        0,
                                                        u32::MAX,
                                                        a_inf.clone(),
                                                        2,
                                                        u32::MAX,
                                                    );
                                                    assert!(!tk.driver().has_ballot_timer());
                                                });

                                                section!(
                                                    other_nodes_moved_c_a4_h_a5,
                                                    "other nodes moved to c=A4 h=A5",
                                                    {
                                                        section!(confirm_a4_5, "Confirm A4..5", {
                                                            recv_v_blocking(
                                                                &mut tk,
                                                                &make_confirm_gen(
                                                                    qset_hash.into_inner(),
                                                                    3,
                                                                    a5.clone(),
                                                                    4,
                                                                    5,
                                                                ),
                                                            );
                                                            assert_eq!(
                                                                tk.driver().envelopes.len(),
                                                                9
                                                            );
                                                            verify_confirm(
                                                                &tk.driver().envelopes[8],
                                                                &v0_secret_key,
                                                                qset_hash0,
                                                                0,
                                                                3,
                                                                a5.clone(),
                                                                4,
                                                                5,
                                                            );
                                                            assert!(
                                                                !tk.driver().has_ballot_timer()
                                                            );
                                                        });

                                                        section!(
                                                            externalize_a4_5,
                                                            "Externalize A4..5",
                                                            {
                                                                recv_v_blocking(
                                                                    &mut tk,
                                                                    &make_externalize_gen(
                                                                        qset_hash.into_inner(),
                                                                        a4.clone(),
                                                                        5,
                                                                    ),
                                                                );
                                                                assert_eq!(
                                                                    tk.driver().envelopes.len(),
                                                                    9
                                                                );
                                                                verify_confirm(
                                                                    &tk.driver().envelopes[8],
                                                                    &v0_secret_key,
                                                                    qset_hash0,
                                                                    0,
                                                                    u32::MAX,
                                                                    a_inf.clone(),
                                                                    4,
                                                                    u32::MAX,
                                                                );
                                                                assert!(
                                                                    !tk.driver().has_ballot_timer()
                                                                );
                                                            }
                                                        );
                                                    }
                                                );
                                            }
                                        );
                                    });

                                    section!(vblocking_prepared_a3, "v-blocking prepared A3", {
                                        recv_v_blocking(
                                            &mut tk,
                                            &make_prepare_gen(
                                                qset_hash.into_inner(),
                                                a3.clone(),
                                                Some(a3.clone()),
                                                2,
                                                2,
                                                None,
                                            ),
                                        );
                                        assert_eq!(tk.driver().envelopes.len(), 7);
                                        verify_confirm(
                                            &tk.driver().envelopes[6],
                                            &v0_secret_key,
                                            qset_hash0,
                                            0,
                                            3,
                                            a3.clone(),
                                            2,
                                            2,
                                        );
                                        assert!(!tk.driver().has_ballot_timer());
                                    });

                                    section!(
                                        vblocking_prepared_a3_b3,
                                        "v-blocking prepared A3+B3",
                                        {
                                            recv_v_blocking(
                                                &mut tk,
                                                &make_prepare_gen(
                                                    qset_hash.into_inner(),
                                                    a3.clone(),
                                                    Some(b3.clone()),
                                                    2,
                                                    2,
                                                    Some(a3.clone()),
                                                ),
                                            );
                                            assert_eq!(tk.driver().envelopes.len(), 7);
                                            verify_confirm(
                                                &tk.driver().envelopes[6],
                                                &v0_secret_key,
                                                qset_hash0,
                                                0,
                                                3,
                                                a3.clone(),
                                                2,
                                                2,
                                            );
                                            assert!(!tk.driver().has_ballot_timer());
                                        }
                                    );

                                    section!(vblocking_confirm_a3, "v-blocking confirm A3", {
                                        recv_v_blocking(
                                            &mut tk,
                                            &make_confirm_gen(
                                                qset_hash.into_inner(),
                                                3,
                                                a3.clone(),
                                                2,
                                                2,
                                            ),
                                        );
                                        assert_eq!(tk.driver().envelopes.len(), 7);
                                        verify_confirm(
                                            &tk.driver().envelopes[6],
                                            &v0_secret_key,
                                            qset_hash0,
                                            0,
                                            3,
                                            a3.clone(),
                                            2,
                                            2,
                                        );
                                        assert!(!tk.driver().has_ballot_timer());
                                    });

                                    section!(
                                        hang_no_switch_b_confirm,
                                        "Hang - does not switch to B in CONFIRM",
                                        {
                                            section!(network_externalize, "Network EXTERNALIZE", {
                                                // externalize messages have a counter at infinite
                                                recv_v_blocking(
                                                    &mut tk,
                                                    &make_externalize_gen(
                                                        qset_hash.into_inner(),
                                                        b2.clone(),
                                                        3,
                                                    ),
                                                );
                                                assert_eq!(tk.driver().envelopes.len(), 7);
                                                verify_confirm(
                                                    &tk.driver().envelopes[6],
                                                    &v0_secret_key,
                                                    qset_hash0,
                                                    0,
                                                    2,
                                                    a_inf.clone(),
                                                    2,
                                                    2,
                                                );
                                                assert!(!tk.driver().has_ballot_timer());

                                                // stuck
                                                recv_quorum_checks(
                                                    &mut tk,
                                                    &make_externalize_gen(
                                                        qset_hash.into_inner(),
                                                        b2.clone(),
                                                        3,
                                                    ),
                                                    false,
                                                    false,
                                                );
                                                assert_eq!(tk.driver().envelopes.len(), 7);
                                                assert_eq!(
                                                    tk.driver().externalized_values.len(),
                                                    0
                                                );
                                                // timer scheduled as there is a quorum with (2, *)
                                                assert!(tk.driver().has_ballot_timer_upcoming());
                                            });

                                            section!(
                                                network_confirms_other_ballot,
                                                "Network CONFIRMS other ballot",
                                                {
                                                    section!(at_same_counter, "at same counter", {
                                                        // nothing should happen here, in particular, node should not
                                                        // attempt to switch 'p'
                                                        recv_quorum_checks(
                                                            &mut tk,
                                                            &make_confirm_gen(
                                                                qset_hash.into_inner(),
                                                                3,
                                                                b2.clone(),
                                                                2,
                                                                3,
                                                            ),
                                                            false,
                                                            false,
                                                        );
                                                        assert_eq!(tk.driver().envelopes.len(), 6);
                                                        assert_eq!(
                                                            tk.driver().externalized_values.len(),
                                                            0
                                                        );
                                                        assert!(
                                                            !tk.driver()
                                                                .has_ballot_timer_upcoming()
                                                        );
                                                    });

                                                    section!(
                                                        at_different_counter,
                                                        "at a different counter",
                                                        {
                                                            recv_v_blocking(
                                                                &mut tk,
                                                                &make_confirm_gen(
                                                                    qset_hash.into_inner(),
                                                                    3,
                                                                    b3.clone(),
                                                                    3,
                                                                    3,
                                                                ),
                                                            );
                                                            assert_eq!(
                                                                tk.driver().envelopes.len(),
                                                                7
                                                            );
                                                            verify_confirm(
                                                                &tk.driver().envelopes[6],
                                                                &v0_secret_key,
                                                                qset_hash0,
                                                                0,
                                                                2,
                                                                a3.clone(),
                                                                2,
                                                                2,
                                                            );
                                                            assert!(
                                                                !tk.driver().has_ballot_timer()
                                                            );

                                                            recv_quorum_checks(
                                                                &mut tk,
                                                                &make_confirm_gen(
                                                                    qset_hash.into_inner(),
                                                                    3,
                                                                    b3.clone(),
                                                                    3,
                                                                    3,
                                                                ),
                                                                false,
                                                                false,
                                                            );
                                                            assert_eq!(
                                                                tk.driver().envelopes.len(),
                                                                7
                                                            );
                                                            assert_eq!(
                                                                tk.driver()
                                                                    .externalized_values
                                                                    .len(),
                                                                0
                                                            );
                                                            // timer scheduled as there is a quorum with (3, *)
                                                            assert!(
                                                                tk.driver()
                                                                    .has_ballot_timer_upcoming()
                                                            );
                                                        }
                                                    );
                                                }
                                            );
                                        }
                                    );
                                });

                                section!(vblocking, "v-blocking", {
                                    section!(confirm, "CONFIRM", {
                                        section!(confirm_a2, "CONFIRM A2", {
                                            recv_v_blocking(
                                                &mut tk,
                                                &make_confirm_gen(
                                                    qset_hash.into_inner(),
                                                    2,
                                                    a2.clone(),
                                                    2,
                                                    2,
                                                ),
                                            );
                                            assert_eq!(tk.driver().envelopes.len(), 6);
                                            verify_confirm(
                                                &tk.driver().envelopes[5],
                                                &v0_secret_key,
                                                qset_hash0,
                                                0,
                                                2,
                                                a2.clone(),
                                                2,
                                                2,
                                            );
                                            assert!(!tk.driver().has_ballot_timer_upcoming());
                                        });

                                        section!(confirm_a3_4, "CONFIRM A3..4", {
                                            recv_v_blocking(
                                                &mut tk,
                                                &make_confirm_gen(
                                                    qset_hash.into_inner(),
                                                    4,
                                                    a4.clone(),
                                                    3,
                                                    4,
                                                ),
                                            );
                                            assert_eq!(tk.driver().envelopes.len(), 6);
                                            verify_confirm(
                                                &tk.driver().envelopes[5],
                                                &v0_secret_key,
                                                qset_hash0,
                                                0,
                                                4,
                                                a4.clone(),
                                                3,
                                                4,
                                            );
                                            assert!(!tk.driver().has_ballot_timer());
                                        });

                                        section!(confirm_b2, "CONFIRM B2", {
                                            recv_v_blocking(
                                                &mut tk,
                                                &make_confirm_gen(
                                                    qset_hash.into_inner(),
                                                    2,
                                                    b2.clone(),
                                                    2,
                                                    2,
                                                ),
                                            );
                                            assert_eq!(tk.driver().envelopes.len(), 6);
                                            verify_confirm(
                                                &tk.driver().envelopes[5],
                                                &v0_secret_key,
                                                qset_hash0,
                                                0,
                                                2,
                                                b2.clone(),
                                                2,
                                                2,
                                            );
                                            assert!(!tk.driver().has_ballot_timer_upcoming());
                                        });
                                    });

                                    section!(externalize, "EXTERNALIZE", {
                                        section!(externalize_a2, "EXTERNALIZE A2", {
                                            recv_v_blocking(
                                                &mut tk,
                                                &make_externalize_gen(
                                                    qset_hash.into_inner(),
                                                    a2.clone(),
                                                    2,
                                                ),
                                            );
                                            assert_eq!(tk.driver().envelopes.len(), 6);
                                            verify_confirm(
                                                &tk.driver().envelopes[5],
                                                &v0_secret_key,
                                                qset_hash0,
                                                0,
                                                u32::MAX,
                                                a_inf.clone(),
                                                2,
                                                u32::MAX,
                                            );
                                            assert!(!tk.driver().has_ballot_timer());
                                        });

                                        section!(externalize_b2, "EXTERNALIZE B2", {
                                            recv_v_blocking(
                                                &mut tk,
                                                &make_externalize_gen(
                                                    qset_hash.into_inner(),
                                                    b2.clone(),
                                                    2,
                                                ),
                                            );
                                            assert_eq!(tk.driver().envelopes.len(), 6);
                                            verify_confirm(
                                                &tk.driver().envelopes[5],
                                                &v0_secret_key,
                                                qset_hash0,
                                                0,
                                                u32::MAX,
                                                b_inf.clone(),
                                                2,
                                                u32::MAX,
                                            );
                                            assert!(!tk.driver().has_ballot_timer());
                                        });
                                    });
                                });
                            });

                            section!(get_conflicting_prepared_b, "get conflicting prepared B", {
                                section!(same_counter, "same counter", {
                                    recv_v_blocking(
                                        &mut tk,
                                        &make_prepare_gen(
                                            qset_hash.into_inner(),
                                            b2.clone(),
                                            Some(b2.clone()),
                                            0,
                                            0,
                                            None,
                                        ),
                                    );
                                    assert_eq!(tk.driver().envelopes.len(), 6);
                                    verify_prepare(
                                        &tk.driver().envelopes[5],
                                        &v0_secret_key,
                                        qset_hash0,
                                        0,
                                        a2.clone(),
                                        Some(b2.clone()),
                                        0,
                                        2,
                                        Some(a2.clone()),
                                    );
                                    assert!(!tk.driver().has_ballot_timer_upcoming());

                                    recv_quorum(
                                        &mut tk,
                                        &make_prepare_gen(
                                            qset_hash.into_inner(),
                                            b2.clone(),
                                            Some(b2.clone()),
                                            2,
                                            2,
                                            None,
                                        ),
                                    );
                                    assert_eq!(tk.driver().envelopes.len(), 7);
                                    verify_confirm(
                                        &tk.driver().envelopes[6],
                                        &v0_secret_key,
                                        qset_hash0,
                                        0,
                                        2,
                                        b2.clone(),
                                        2,
                                        2,
                                    );
                                    assert!(!tk.driver().has_ballot_timer_upcoming());
                                });

                                section!(higher_counter, "higher counter", {
                                    recv_v_blocking(
                                        &mut tk,
                                        &make_prepare_gen(
                                            qset_hash.into_inner(),
                                            b3.clone(),
                                            Some(b2.clone()),
                                            2,
                                            2,
                                            None,
                                        ),
                                    );
                                    assert_eq!(tk.driver().envelopes.len(), 6);
                                    verify_prepare(
                                        &tk.driver().envelopes[5],
                                        &v0_secret_key,
                                        qset_hash0,
                                        0,
                                        a3.clone(),
                                        Some(b2.clone()),
                                        0,
                                        2,
                                        Some(a2.clone()),
                                    );
                                    assert!(!tk.driver().has_ballot_timer());

                                    recv_quorum_checks_ex(
                                        &mut tk,
                                        &make_prepare_gen(
                                            qset_hash.into_inner(),
                                            b3.clone(),
                                            Some(b2.clone()),
                                            2,
                                            2,
                                            None,
                                        ),
                                        true,
                                        true,
                                        true,
                                    );
                                    assert_eq!(tk.driver().envelopes.len(), 7);
                                    verify_confirm(
                                        &tk.driver().envelopes[6],
                                        &v0_secret_key,
                                        qset_hash0,
                                        0,
                                        3,
                                        b3.clone(),
                                        2,
                                        2,
                                    );
                                });

                                section!(higher_counter_mixed, "higher counter mixed", {
                                    recv_v_blocking(
                                        &mut tk,
                                        &make_prepare_gen(
                                            qset_hash.into_inner(),
                                            a3.clone(),
                                            Some(b3.clone()),
                                            0,
                                            2,
                                            Some(a2.clone()),
                                        ),
                                    );
                                    assert_eq!(tk.driver().envelopes.len(), 6);
                                    // h still A2
                                    // v-blocking
                                    //     prepared B3 -> p = B3, p'=A2 (1)
                                    //     counter 3, b = A3 (9) (same value than h)
                                    // c = 0 (1)
                                    verify_prepare(
                                        &tk.driver().envelopes[5],
                                        &v0_secret_key,
                                        qset_hash0,
                                        0,
                                        a3.clone(),
                                        Some(b3.clone()),
                                        0,
                                        2,
                                        Some(a2.clone()),
                                    );
                                    recv_quorum_ex(
                                        &mut tk,
                                        &make_prepare_gen(
                                            qset_hash.into_inner(),
                                            a3.clone(),
                                            Some(b3.clone()),
                                            0,
                                            2,
                                            Some(a2.clone()),
                                        ),
                                        true,
                                    );
                                    // p=B3, p'=A3 (1)
                                    // computed_h = B3
                                    // b = computed_h = B3 (8)
                                    // h = computed_h = B3 (2)
                                    // c = h = B3 (3)
                                    assert_eq!(tk.driver().envelopes.len(), 7);
                                    verify_prepare(
                                        &tk.driver().envelopes[6],
                                        &v0_secret_key,
                                        qset_hash0,
                                        0,
                                        b3.clone(),
                                        Some(b3.clone()),
                                        3,
                                        3,
                                        Some(a3.clone()),
                                    );
                                });
                            });
                        });

                        section!(confirm_prepared_mixed, "Confirm prepared mixed", {
                            // a few nodes prepared B2
                            recv_v_blocking(
                                &mut tk,
                                &make_prepare_gen(
                                    qset_hash.into_inner(),
                                    b2.clone(),
                                    Some(b2.clone()),
                                    0,
                                    0,
                                    Some(a2.clone()),
                                ),
                            );
                            assert_eq!(tk.driver().envelopes.len(), 5);
                            verify_prepare(
                                &tk.driver().envelopes[4],
                                &v0_secret_key,
                                qset_hash0,
                                0,
                                a2.clone(),
                                Some(b2.clone()),
                                0,
                                0,
                                Some(a2.clone()),
                            );
                            assert!(!tk.driver().has_ballot_timer_upcoming());

                            section!(mixed_a2, "mixed A2", {
                                // causes h=A2
                                // but c = 0, as p >!~ h
                                tk.driver().bump_timer_offset();
                                tk.proto().received_envelope(
                                    &make_prepare(
                                        &v3_secret_key,
                                        qset_hash.into_inner(),
                                        0,
                                        a2.clone(),
                                        Some(a2.clone()),
                                        0,
                                        0,
                                        None,
                                    )
                                    .unwrap(),
                                );

                                assert_eq!(tk.driver().envelopes.len(), 6);
                                verify_prepare(
                                    &tk.driver().envelopes[5],
                                    &v0_secret_key,
                                    qset_hash0,
                                    0,
                                    a2.clone(),
                                    Some(b2.clone()),
                                    0,
                                    2,
                                    Some(a2.clone()),
                                );
                                assert!(!tk.driver().has_ballot_timer_upcoming());

                                tk.driver().bump_timer_offset();
                                tk.proto().received_envelope(
                                    &make_prepare(
                                        &v4_secret_key,
                                        qset_hash.into_inner(),
                                        0,
                                        a2.clone(),
                                        Some(a2.clone()),
                                        0,
                                        0,
                                        None,
                                    )
                                    .unwrap(),
                                );

                                assert_eq!(tk.driver().envelopes.len(), 6);
                                assert!(!tk.driver().has_ballot_timer_upcoming());
                            });

                            section!(mixed_b2, "mixed B2", {
                                // causes h=B2, c=B2
                                tk.driver().bump_timer_offset();
                                tk.proto().received_envelope(
                                    &make_prepare(
                                        &v3_secret_key,
                                        qset_hash.into_inner(),
                                        0,
                                        b2.clone(),
                                        Some(b2.clone()),
                                        0,
                                        0,
                                        None,
                                    )
                                    .unwrap(),
                                );

                                assert_eq!(tk.driver().envelopes.len(), 6);
                                verify_prepare(
                                    &tk.driver().envelopes[5],
                                    &v0_secret_key,
                                    qset_hash0,
                                    0,
                                    b2.clone(),
                                    Some(b2.clone()),
                                    2,
                                    2,
                                    Some(a2.clone()),
                                );
                                assert!(!tk.driver().has_ballot_timer_upcoming());

                                tk.driver().bump_timer_offset();
                                tk.proto().received_envelope(
                                    &make_prepare(
                                        &v4_secret_key,
                                        qset_hash.into_inner(),
                                        0,
                                        b2.clone(),
                                        Some(b2.clone()),
                                        0,
                                        0,
                                        None,
                                    )
                                    .unwrap(),
                                );

                                assert_eq!(tk.driver().envelopes.len(), 6);
                                assert!(!tk.driver().has_ballot_timer_upcoming());
                            });
                        });
                    });

                    section!(switch_prepared_b1_from_a1, "switch prepared B1 from A1", {
                        // (p,p') = (B1, A1) [ from (A1, null) ]
                        recv_v_blocking(
                            &mut tk,
                            &make_prepare_gen(
                                qset_hash.into_inner(),
                                b1.clone(),
                                Some(b1.clone()),
                                0,
                                0,
                                None,
                            ),
                        );
                        assert_eq!(tk.driver().envelopes.len(), 3);
                        verify_prepare(
                            &tk.driver().envelopes[2],
                            &v0_secret_key,
                            qset_hash0,
                            0,
                            a1.clone(),
                            Some(b1.clone()),
                            0,
                            0,
                            Some(a1.clone()),
                        );
                        assert!(!tk.driver().has_ballot_timer_upcoming());

                        // v-blocking with n=2 -> bump n
                        recv_v_blocking(
                            &mut tk,
                            &make_prepare_gen(qset_hash.into_inner(), b2.clone(), None, 0, 0, None),
                        );
                        assert_eq!(tk.driver().envelopes.len(), 4);
                        verify_prepare(
                            &tk.driver().envelopes[3],
                            &v0_secret_key,
                            qset_hash0,
                            0,
                            a2.clone(),
                            Some(b1.clone()),
                            0,
                            0,
                            Some(a1.clone()),
                        );

                        // move to (p,p') = (B2, A1) [update p from B1 -> B2]
                        recv_v_blocking(
                            &mut tk,
                            &make_prepare_gen(
                                qset_hash.into_inner(),
                                b2.clone(),
                                Some(b2.clone()),
                                0,
                                0,
                                None,
                            ),
                        );
                        assert_eq!(tk.driver().envelopes.len(), 5);
                        verify_prepare(
                            &tk.driver().envelopes[4],
                            &v0_secret_key,
                            qset_hash0,
                            0,
                            a2.clone(),
                            Some(b2.clone()),
                            0,
                            0,
                            Some(a1.clone()),
                        );
                        assert!(!tk.driver().has_ballot_timer()); // no quorum (other nodes on (A,1))

                        section!(
                            vblocking_switches_to_previous_value_of_p,
                            "v-blocking switches to previous value of p",
                            {
                                // v-blocking with n=3 -> bump n
                                recv_v_blocking(
                                    &mut tk,
                                    &make_prepare_gen(
                                        qset_hash.into_inner(),
                                        b3.clone(),
                                        None,
                                        0,
                                        0,
                                        None,
                                    ),
                                );
                                assert_eq!(tk.driver().envelopes.len(), 6);
                                verify_prepare(
                                    &tk.driver().envelopes[5],
                                    &v0_secret_key,
                                    qset_hash0,
                                    0,
                                    a3.clone(),
                                    Some(b2.clone()),
                                    0,
                                    0,
                                    Some(a1.clone()),
                                );
                                assert!(!tk.driver().has_ballot_timer()); // no quorum (other nodes on (A,1))

                                // vBlocking set says "B1" is prepared - but we already have p=B2
                                recv_v_blocking_checks(
                                    &mut tk,
                                    &make_prepare_gen(
                                        qset_hash.into_inner(),
                                        b3.clone(),
                                        Some(b1.clone()),
                                        0,
                                        0,
                                        None,
                                    ),
                                    false,
                                );
                                assert_eq!(tk.driver().envelopes.len(), 6);
                                assert!(!tk.driver().has_ballot_timer());
                            }
                        );

                        section!(switch_p_prime_to_mid2, "switch p' to Mid2", {
                            // (p,p') = (B2, Mid2)
                            recv_v_blocking(
                                &mut tk,
                                &make_prepare_gen(
                                    qset_hash.into_inner(),
                                    b2.clone(),
                                    Some(b2.clone()),
                                    0,
                                    0,
                                    Some(mid2.clone()),
                                ),
                            );
                            assert_eq!(tk.driver().envelopes.len(), 6);
                            verify_prepare(
                                &tk.driver().envelopes[5],
                                &v0_secret_key,
                                qset_hash0,
                                0,
                                a2.clone(),
                                Some(b2.clone()),
                                0,
                                0,
                                Some(mid2.clone()),
                            );
                            assert!(!tk.driver().has_ballot_timer());
                        });

                        section!(switch_again_big2, "switch again Big2", {
                            // both p and p' get updated
                            // (p,p') = (Big2, B2)
                            recv_v_blocking(
                                &mut tk,
                                &make_prepare_gen(
                                    qset_hash.into_inner(),
                                    b2.clone(),
                                    Some(big2.clone()),
                                    0,
                                    0,
                                    Some(b2.clone()),
                                ),
                            );
                            assert_eq!(tk.driver().envelopes.len(), 6);
                            verify_prepare(
                                &tk.driver().envelopes[5],
                                &v0_secret_key,
                                qset_hash0,
                                0,
                                a2.clone(),
                                Some(big2.clone()),
                                0,
                                0,
                                Some(b2.clone()),
                            );
                            assert!(!tk.driver().has_ballot_timer());
                        });
                    });

                    section!(switch_prepare_b1, "switch prepare B1", {
                        recv_quorum_checks(
                            &mut tk,
                            &make_prepare_gen(qset_hash.into_inner(), b1.clone(), None, 0, 0, None),
                            true,
                            true,
                        );
                        assert_eq!(tk.driver().envelopes.len(), 3);
                        verify_prepare(
                            &tk.driver().envelopes[2],
                            &v0_secret_key,
                            qset_hash0,
                            0,
                            a1.clone(),
                            Some(b1.clone()),
                            0,
                            0,
                            Some(a1.clone()),
                        );
                        assert!(!tk.driver().has_ballot_timer_upcoming());
                    });

                    section!(
                        prepare_higher_counter_vblocking,
                        "prepare higher counter (v-blocking)",
                        {
                            recv_v_blocking(
                                &mut tk,
                                &make_prepare_gen(
                                    qset_hash.into_inner(),
                                    b2.clone(),
                                    None,
                                    0,
                                    0,
                                    None,
                                ),
                            );
                            assert_eq!(tk.driver().envelopes.len(), 3);
                            verify_prepare(
                                &tk.driver().envelopes[2],
                                &v0_secret_key,
                                qset_hash0,
                                0,
                                a2.clone(),
                                Some(a1.clone()),
                                0,
                                0,
                                None,
                            );
                            assert!(!tk.driver().has_ballot_timer());

                            // more timeout from vBlocking set
                            recv_v_blocking(
                                &mut tk,
                                &make_prepare_gen(
                                    qset_hash.into_inner(),
                                    b3.clone(),
                                    None,
                                    0,
                                    0,
                                    None,
                                ),
                            );
                            assert_eq!(tk.driver().envelopes.len(), 4);
                            verify_prepare(
                                &tk.driver().envelopes[3],
                                &v0_secret_key,
                                qset_hash0,
                                0,
                                a3.clone(),
                                Some(a1.clone()),
                                0,
                                0,
                                None,
                            );
                            assert!(!tk.driver().has_ballot_timer());
                        }
                    );
                });

                section!(prepared_b_vblocking, "prepared B (v-blocking)", {
                    recv_v_blocking(
                        &mut tk,
                        &make_prepare_gen(
                            qset_hash.into_inner(),
                            b1.clone(),
                            Some(b1.clone()),
                            0,
                            0,
                            None,
                        ),
                    );
                    assert_eq!(tk.driver().envelopes.len(), 2);
                    verify_prepare(
                        &tk.driver().envelopes[1],
                        &v0_secret_key,
                        qset_hash0,
                        0,
                        a1.clone(),
                        Some(b1.clone()),
                        0,
                        0,
                        None,
                    );
                    assert!(!tk.driver().has_ballot_timer());
                });

                section!(prepare_b_quorum, "prepare B (quorum)", {
                    recv_quorum_checks_ex(
                        &mut tk,
                        &make_prepare_gen(qset_hash.into_inner(), b1.clone(), None, 0, 0, None),
                        true,
                        true,
                        true,
                    );
                    assert_eq!(tk.driver().envelopes.len(), 2);
                    verify_prepare(
                        &tk.driver().envelopes[1],
                        &v0_secret_key,
                        qset_hash0,
                        0,
                        a1.clone(),
                        Some(b1.clone()),
                        0,
                        0,
                        None,
                    );
                });

                section!(confirm_vblocking, "confirm (v-blocking)", {
                    section!(via_confirm, "via CONFIRM", {
                        tk.driver().bump_timer_offset();
                        tk.proto().received_envelope(
                            &make_confirm(
                                &v1_secret_key,
                                qset_hash.into_inner(),
                                0,
                                3,
                                a3.clone(),
                                3,
                                3,
                            )
                            .unwrap(),
                        );
                        tk.proto().received_envelope(
                            &make_confirm(
                                &v2_secret_key,
                                qset_hash.into_inner(),
                                0,
                                4,
                                a4.clone(),
                                2,
                                4,
                            )
                            .unwrap(),
                        );
                        assert_eq!(tk.driver().envelopes.len(), 2);
                        verify_confirm(
                            &tk.driver().envelopes[1],
                            &v0_secret_key,
                            qset_hash0,
                            0,
                            3,
                            a3.clone(),
                            3,
                            3,
                        );
                        assert!(!tk.driver().has_ballot_timer());
                    });

                    section!(via_externalize, "via EXTERNALIZE", {
                        tk.proto().received_envelope(
                            &make_externalize(
                                &v1_secret_key,
                                qset_hash.into_inner(),
                                0,
                                a2.clone(),
                                4,
                            )
                            .unwrap(),
                        );
                        tk.proto().received_envelope(
                            &make_externalize(
                                &v2_secret_key,
                                qset_hash.into_inner(),
                                0,
                                a3.clone(),
                                5,
                            )
                            .unwrap(),
                        );
                        assert_eq!(tk.driver().envelopes.len(), 2);
                        verify_confirm(
                            &tk.driver().envelopes[1],
                            &v0_secret_key,
                            qset_hash0,
                            0,
                            u32::MAX,
                            a_inf.clone(),
                            3,
                            u32::MAX,
                        );
                        assert!(!tk.driver().has_ballot_timer());
                    });
                });
            });

            // this is the same test suite than "start <1,x>" with the exception
            // that some transitions are not possible as x < z - so instead we
            // verify that nothing happens
            section!(start_1z, "start <1,z>", {
                // no timer is set
                assert!(!tk.driver().has_ballot_timer());

                let a_value = &values.z_value; // Different: using z instead of x
                let b_value = &values.x_value; // Different: using x instead of z
                let mid_value = &values.y_value;
                let big_value = &values.zz_value;

                let a1 = SCPBallot {
                    counter: 1,
                    value: a_value.clone(),
                };
                let b1 = SCPBallot {
                    counter: 1,
                    value: b_value.clone(),
                };
                let mid1 = SCPBallot {
                    counter: 1,
                    value: mid_value.clone(),
                };
                let big1 = SCPBallot {
                    counter: 1,
                    value: big_value.clone(),
                };

                let mut a2 = a1.clone();
                a2.counter += 1;

                let mut a3 = a2.clone();
                a3.counter += 1;

                let mut a4 = a3.clone();
                a4.counter += 1;

                let mut a5 = a4.clone();
                a5.counter += 1;

                let a_inf = SCPBallot {
                    counter: u32::MAX,
                    value: a_value.clone(),
                };
                let b_inf = SCPBallot {
                    counter: u32::MAX,
                    value: b_value.clone(),
                };

                let mut b2 = b1.clone();
                b2.counter += 1;

                let mut b3 = b2.clone();
                b3.counter += 1;

                let mut b4 = b3.clone();
                b4.counter += 1;

                assert!(tk.bump_state(0, a_value));
                assert_eq!(tk.driver().envelopes.len(), 1);
                assert!(!tk.driver().has_ballot_timer());

                section!(prepared_a1, "prepared A1", {
                    recv_quorum_ex(
                        &mut tk,
                        &make_prepare_gen(qset_hash.into_inner(), a1.clone(), None, 0, 0, None),
                        true,
                    );

                    assert_eq!(tk.driver().envelopes.len(), 2);
                    verify_prepare(
                        &tk.driver().envelopes[1],
                        &v0_secret_key,
                        qset_hash0,
                        0,
                        a1.clone(),
                        Some(a1.clone()),
                        0,
                        0,
                        None,
                    );

                    section!(bump_prep_a2, "bump prepared A2", {
                        // bump to (2,a)
                        tk.driver().bump_timer_offset();
                        assert!(tk.bump_state(0, a_value));
                        assert_eq!(tk.driver().envelopes.len(), 3);
                        verify_prepare(
                            &tk.driver().envelopes[2],
                            &v0_secret_key,
                            qset_hash0,
                            0,
                            a2.clone(),
                            Some(a1.clone()),
                            0,
                            0,
                            None,
                        );
                        assert!(!tk.driver().has_ballot_timer());

                        recv_quorum_ex(
                            &mut tk,
                            &make_prepare_gen(qset_hash.into_inner(), a2.clone(), None, 0, 0, None),
                            true,
                        );
                        assert_eq!(tk.driver().envelopes.len(), 4);
                        verify_prepare(
                            &tk.driver().envelopes[3],
                            &v0_secret_key,
                            qset_hash0,
                            0,
                            a2.clone(),
                            Some(a2.clone()),
                            0,
                            0,
                            None,
                        );

                        section!(confirm_prep_a2, "Confirm prepared A2", {
                            recv_quorum(
                                &mut tk,
                                &make_prepare_gen(
                                    qset_hash.into_inner(),
                                    a2.clone(),
                                    Some(a2.clone()),
                                    0,
                                    0,
                                    None,
                                ),
                            );
                            assert_eq!(tk.driver().envelopes.len(), 5);
                            verify_prepare(
                                &tk.driver().envelopes[4],
                                &v0_secret_key,
                                qset_hash0,
                                0,
                                a2.clone(),
                                Some(a2.clone()),
                                2,
                                2,
                                None,
                            );
                            assert!(!tk.driver().has_ballot_timer_upcoming());

                            section!(accept_commit, "Accept commit", {
                                section!(quorum_a2, "Quorum A2", {
                                    recv_quorum(
                                        &mut tk,
                                        &make_prepare_gen(
                                            qset_hash.into_inner(),
                                            a2.clone(),
                                            Some(a2.clone()),
                                            2,
                                            2,
                                            None,
                                        ),
                                    );
                                    assert_eq!(tk.driver().envelopes.len(), 6);
                                    verify_confirm(
                                        &tk.driver().envelopes[5],
                                        &v0_secret_key,
                                        qset_hash0,
                                        0,
                                        2,
                                        a2.clone(),
                                        2,
                                        2,
                                    );
                                    assert!(!tk.driver().has_ballot_timer_upcoming());

                                    section!(quorum_prep_a3, "Quorum prepared A3", {
                                        recv_v_blocking(
                                            &mut tk,
                                            &make_prepare_gen(
                                                qset_hash.into_inner(),
                                                a3.clone(),
                                                Some(a2.clone()),
                                                2,
                                                2,
                                                None,
                                            ),
                                        );
                                        assert_eq!(tk.driver().envelopes.len(), 7);
                                        verify_confirm(
                                            &tk.driver().envelopes[6],
                                            &v0_secret_key,
                                            qset_hash0,
                                            0,
                                            2,
                                            a3.clone(),
                                            2,
                                            2,
                                        );
                                        assert!(!tk.driver().has_ballot_timer());

                                        recv_quorum_ex(
                                            &mut tk,
                                            &make_prepare_gen(
                                                qset_hash.into_inner(),
                                                a3.clone(),
                                                Some(a2.clone()),
                                                2,
                                                2,
                                                None,
                                            ),
                                            true,
                                        );
                                        assert_eq!(tk.driver().envelopes.len(), 8);
                                        verify_confirm(
                                            &tk.driver().envelopes[7],
                                            &v0_secret_key,
                                            qset_hash0,
                                            0,
                                            3,
                                            a3.clone(),
                                            2,
                                            2,
                                        );

                                        section!(accept_more_commit_a3, "Accept more commit A3", {
                                            recv_quorum(
                                                &mut tk,
                                                &make_prepare_gen(
                                                    qset_hash.into_inner(),
                                                    a3.clone(),
                                                    Some(a3.clone()),
                                                    2,
                                                    3,
                                                    None,
                                                ),
                                            );
                                            assert_eq!(tk.driver().envelopes.len(), 9);
                                            verify_confirm(
                                                &tk.driver().envelopes[8],
                                                &v0_secret_key,
                                                qset_hash0,
                                                0,
                                                3,
                                                a3.clone(),
                                                2,
                                                3,
                                            );
                                            assert!(!tk.driver().has_ballot_timer_upcoming());

                                            assert_eq!(tk.driver().externalized_values.len(), 0);

                                            section!(
                                                quorum_externalize_a3,
                                                "Quorum externalize A3",
                                                {
                                                    recv_quorum(
                                                        &mut tk,
                                                        &make_confirm_gen(
                                                            qset_hash.into_inner(),
                                                            3,
                                                            a3.clone(),
                                                            2,
                                                            3,
                                                        ),
                                                    );
                                                    assert_eq!(tk.driver().envelopes.len(), 10);
                                                    verify_externalize(
                                                        &tk.driver().envelopes[9],
                                                        &v0_secret_key,
                                                        qset_hash0,
                                                        0,
                                                        a2.clone(),
                                                        3,
                                                    );
                                                    assert!(!tk.driver().has_ballot_timer());

                                                    assert_eq!(
                                                        tk.driver().externalized_values.len(),
                                                        1
                                                    );
                                                    assert_eq!(
                                                        tk.driver()
                                                            .externalized_values
                                                            .get(&0)
                                                            .unwrap(),
                                                        a_value
                                                    );
                                                }
                                            );
                                        });

                                        section!(
                                            vblocking_accept_more_a3,
                                            "v-blocking accept more A3",
                                            {
                                                section!(confirm_a3, "Confirm A3", {
                                                    recv_v_blocking(
                                                        &mut tk,
                                                        &make_confirm_gen(
                                                            qset_hash.into_inner(),
                                                            3,
                                                            a3.clone(),
                                                            2,
                                                            3,
                                                        ),
                                                    );
                                                    assert_eq!(tk.driver().envelopes.len(), 9);
                                                    verify_confirm(
                                                        &tk.driver().envelopes[8],
                                                        &v0_secret_key,
                                                        qset_hash0,
                                                        0,
                                                        3,
                                                        a3.clone(),
                                                        2,
                                                        3,
                                                    );
                                                    assert!(
                                                        !tk.driver().has_ballot_timer_upcoming()
                                                    );
                                                });

                                                section!(externalize_a3, "Externalize A3", {
                                                    recv_v_blocking(
                                                        &mut tk,
                                                        &make_externalize_gen(
                                                            qset_hash.into_inner(),
                                                            a2.clone(),
                                                            3,
                                                        ),
                                                    );
                                                    assert_eq!(tk.driver().envelopes.len(), 9);
                                                    verify_confirm(
                                                        &tk.driver().envelopes[8],
                                                        &v0_secret_key,
                                                        qset_hash0,
                                                        0,
                                                        u32::MAX,
                                                        a_inf.clone(),
                                                        2,
                                                        u32::MAX,
                                                    );
                                                    assert!(!tk.driver().has_ballot_timer());
                                                });

                                                section!(
                                                    other_nodes_moved_c_a4_h_a5_2,
                                                    "other nodes moved to c=A4 h=A5",
                                                    {
                                                        section!(
                                                            confirm_a4_5_2,
                                                            "Confirm A4..5",
                                                            {
                                                                recv_v_blocking(
                                                                    &mut tk,
                                                                    &make_confirm_gen(
                                                                        qset_hash.into_inner(),
                                                                        3,
                                                                        a5.clone(),
                                                                        4,
                                                                        5,
                                                                    ),
                                                                );
                                                                assert_eq!(
                                                                    tk.driver().envelopes.len(),
                                                                    9
                                                                );
                                                                verify_confirm(
                                                                    &tk.driver().envelopes[8],
                                                                    &v0_secret_key,
                                                                    qset_hash0,
                                                                    0,
                                                                    3,
                                                                    a5.clone(),
                                                                    4,
                                                                    5,
                                                                );
                                                                assert!(
                                                                    !tk.driver().has_ballot_timer()
                                                                );
                                                            }
                                                        );
                                                        section!(
                                                            externalize_a4_5,
                                                            "Externalize A4..5",
                                                            {
                                                                recv_v_blocking(
                                                                    &mut tk,
                                                                    &make_externalize_gen(
                                                                        qset_hash.into_inner(),
                                                                        a4.clone(),
                                                                        5,
                                                                    ),
                                                                );
                                                                assert_eq!(
                                                                    tk.driver().envelopes.len(),
                                                                    9
                                                                );
                                                                verify_confirm(
                                                                    &tk.driver().envelopes[8],
                                                                    &v0_secret_key,
                                                                    qset_hash0,
                                                                    0,
                                                                    u32::MAX,
                                                                    a_inf.clone(),
                                                                    4,
                                                                    u32::MAX,
                                                                );
                                                                assert!(
                                                                    !tk.driver().has_ballot_timer()
                                                                );
                                                            }
                                                        );
                                                    }
                                                );
                                            }
                                        );
                                    });

                                    section!(v_blocking_prepared_a3, "v-blocking prepared A3", {
                                        recv_v_blocking(
                                            &mut tk,
                                            &make_prepare_gen(
                                                qset_hash.into_inner(),
                                                a3.clone(),
                                                Some(a3.clone()),
                                                2,
                                                2,
                                                None,
                                            ),
                                        );
                                        assert_eq!(tk.driver().envelopes.len(), 7);
                                        verify_confirm(
                                            &tk.driver().envelopes[6],
                                            &v0_secret_key,
                                            qset_hash0,
                                            0,
                                            3,
                                            a3.clone(),
                                            2,
                                            2,
                                        );
                                        assert!(!tk.driver().has_ballot_timer());
                                    });
                                    section!(
                                        v_blocking_prepared_a3_b3,
                                        "v-blocking prepared A3+B3",
                                        {
                                            recv_v_blocking(
                                                &mut tk,
                                                &make_prepare_gen(
                                                    qset_hash.into_inner(),
                                                    a3.clone(),
                                                    Some(a3.clone()),
                                                    2,
                                                    2,
                                                    Some(b3.clone()),
                                                ),
                                            );
                                            assert_eq!(tk.driver().envelopes.len(), 7);
                                            verify_confirm(
                                                &tk.driver().envelopes[6],
                                                &v0_secret_key,
                                                qset_hash0,
                                                0,
                                                3,
                                                a3.clone(),
                                                2,
                                                2,
                                            );
                                            assert!(!tk.driver().has_ballot_timer());
                                        }
                                    );
                                    section!(v_blocking_confirm_a3, "v-blocking confirm A3", {
                                        recv_v_blocking(
                                            &mut tk,
                                            &make_confirm_gen(
                                                qset_hash.into_inner(),
                                                3,
                                                a3.clone(),
                                                2,
                                                2,
                                            ),
                                        );
                                        assert_eq!(tk.driver().envelopes.len(), 7);
                                        verify_confirm(
                                            &tk.driver().envelopes[6],
                                            &v0_secret_key,
                                            qset_hash0,
                                            0,
                                            3,
                                            a3.clone(),
                                            2,
                                            2,
                                        );
                                        assert!(!tk.driver().has_ballot_timer());
                                    });

                                    section!(
                                        hang_does_not_switch_to_b_in_confirm,
                                        "Hang - does not switch to B in CONFIRM",
                                        {
                                            section!(network_externalize, "Network EXTERNALIZE", {
                                                // externalize messages have a counter
                                                // at infinite
                                                recv_v_blocking(
                                                    &mut tk,
                                                    &make_externalize_gen(
                                                        qset_hash.into_inner(),
                                                        b2.clone(),
                                                        3,
                                                    ),
                                                );
                                                assert_eq!(tk.driver().envelopes.len(), 7);
                                                verify_confirm(
                                                    &tk.driver().envelopes[6],
                                                    &v0_secret_key,
                                                    qset_hash0,
                                                    0,
                                                    2,
                                                    a_inf.clone(),
                                                    2,
                                                    2,
                                                );
                                                assert!(!tk.driver().has_ballot_timer());

                                                // stuck
                                                recv_quorum_checks(
                                                    &mut tk,
                                                    &make_externalize_gen(
                                                        qset_hash.into_inner(),
                                                        b2.clone(),
                                                        3,
                                                    ),
                                                    false,
                                                    false,
                                                );
                                                assert_eq!(tk.driver().envelopes.len(), 7);
                                                assert_eq!(
                                                    tk.driver().externalized_values.len(),
                                                    0
                                                );
                                                // timer scheduled as there is a quorum
                                                // with (inf, *)
                                                assert!(tk.driver().has_ballot_timer_upcoming());
                                            });
                                            section!(
                                                network_confirms_other_ballot,
                                                "Network CONFIRMS other ballot",
                                                {
                                                    section!(at_same_counter, "at same counter", {
                                                        // nothing should happen here, in
                                                        // particular, node should not
                                                        // attempt to switch 'p'
                                                        recv_quorum_checks(
                                                            &mut tk,
                                                            &make_confirm_gen(
                                                                qset_hash.into_inner(),
                                                                3,
                                                                b2.clone(),
                                                                2,
                                                                3,
                                                            ),
                                                            false,
                                                            false,
                                                        );
                                                        assert_eq!(tk.driver().envelopes.len(), 6);
                                                        assert_eq!(
                                                            tk.driver().externalized_values.len(),
                                                            0
                                                        );
                                                        assert!(
                                                            !tk.driver()
                                                                .has_ballot_timer_upcoming()
                                                        );
                                                    });
                                                    section!(
                                                        at_different_counter,
                                                        "at a different counter",
                                                        {
                                                            recv_v_blocking(
                                                                &mut tk,
                                                                &make_confirm_gen(
                                                                    qset_hash.into_inner(),
                                                                    3,
                                                                    b3.clone(),
                                                                    3,
                                                                    3,
                                                                ),
                                                            );
                                                            assert_eq!(
                                                                tk.driver().envelopes.len(),
                                                                7
                                                            );
                                                            verify_confirm(
                                                                &tk.driver().envelopes[6],
                                                                &v0_secret_key,
                                                                qset_hash0,
                                                                0,
                                                                2,
                                                                a3.clone(),
                                                                2,
                                                                2,
                                                            );
                                                            assert!(
                                                                !tk.driver().has_ballot_timer()
                                                            );

                                                            recv_quorum_checks(
                                                                &mut tk,
                                                                &make_confirm_gen(
                                                                    qset_hash.into_inner(),
                                                                    3,
                                                                    b3.clone(),
                                                                    3,
                                                                    3,
                                                                ),
                                                                false,
                                                                false,
                                                            );
                                                            assert_eq!(
                                                                tk.driver().envelopes.len(),
                                                                7
                                                            );
                                                            assert_eq!(
                                                                tk.driver()
                                                                    .externalized_values
                                                                    .len(),
                                                                0
                                                            );
                                                            // timer scheduled as there is a
                                                            // quorum with (3, *)
                                                            assert!(
                                                                tk.driver()
                                                                    .has_ballot_timer_upcoming()
                                                            );
                                                        }
                                                    );
                                                }
                                            );
                                        }
                                    );
                                });
                                section!(v_blocking, "v-blocking", {
                                    section!(confirm_section, "CONFIRM", {
                                        section!(confirm_a2, "CONFIRM A2", {
                                            recv_v_blocking(
                                                &mut tk,
                                                &make_confirm_gen(
                                                    qset_hash.into_inner(),
                                                    2,
                                                    a2.clone(),
                                                    2,
                                                    2,
                                                ),
                                            );
                                            assert_eq!(tk.driver().envelopes.len(), 6);
                                            verify_confirm(
                                                &tk.driver().envelopes[5],
                                                &v0_secret_key,
                                                qset_hash0,
                                                0,
                                                2,
                                                a2.clone(),
                                                2,
                                                2,
                                            );
                                            assert!(!tk.driver().has_ballot_timer_upcoming());
                                        });
                                        section!(confirm_a3_4, "CONFIRM A3..4", {
                                            recv_v_blocking(
                                                &mut tk,
                                                &make_confirm_gen(
                                                    qset_hash.into_inner(),
                                                    4,
                                                    a4.clone(),
                                                    3,
                                                    4,
                                                ),
                                            );
                                            assert_eq!(tk.driver().envelopes.len(), 6);
                                            verify_confirm(
                                                &tk.driver().envelopes[5],
                                                &v0_secret_key,
                                                qset_hash0,
                                                0,
                                                4,
                                                a4.clone(),
                                                3,
                                                4,
                                            );
                                            assert!(!tk.driver().has_ballot_timer());
                                        });
                                        section!(confirm_b2, "CONFIRM B2", {
                                            recv_v_blocking(
                                                &mut tk,
                                                &make_confirm_gen(
                                                    qset_hash.into_inner(),
                                                    2,
                                                    b2.clone(),
                                                    2,
                                                    2,
                                                ),
                                            );
                                            assert_eq!(tk.driver().envelopes.len(), 6);
                                            verify_confirm(
                                                &tk.driver().envelopes[5],
                                                &v0_secret_key,
                                                qset_hash0,
                                                0,
                                                2,
                                                b2.clone(),
                                                2,
                                                2,
                                            );
                                            assert!(!tk.driver().has_ballot_timer_upcoming());
                                        });
                                    });
                                    section!(externalize_section, "EXTERNALIZE", {
                                        section!(externalize_a2, "EXTERNALIZE A2", {
                                            recv_v_blocking(
                                                &mut tk,
                                                &make_externalize_gen(
                                                    qset_hash.into_inner(),
                                                    a2.clone(),
                                                    2,
                                                ),
                                            );
                                            assert_eq!(tk.driver().envelopes.len(), 6);
                                            verify_confirm(
                                                &tk.driver().envelopes[5],
                                                &v0_secret_key,
                                                qset_hash0,
                                                0,
                                                u32::MAX,
                                                a_inf.clone(),
                                                2,
                                                u32::MAX,
                                            );
                                            assert!(!tk.driver().has_ballot_timer());
                                        });
                                        section!(externalize_b2, "EXTERNALIZE B2", {
                                            // can switch to B2 with externalize
                                            // (higher counter)
                                            recv_v_blocking(
                                                &mut tk,
                                                &make_externalize_gen(
                                                    qset_hash.into_inner(),
                                                    b2.clone(),
                                                    2,
                                                ),
                                            );
                                            assert_eq!(tk.driver().envelopes.len(), 6);
                                            verify_confirm(
                                                &tk.driver().envelopes[5],
                                                &v0_secret_key,
                                                qset_hash0,
                                                0,
                                                u32::MAX,
                                                b_inf.clone(),
                                                2,
                                                u32::MAX,
                                            );
                                            assert!(!tk.driver().has_ballot_timer());
                                        });
                                    });
                                });
                            });
                            section!(get_conflicting_prepared_b, "get conflicting prepared B", {
                                section!(same_counter, "same counter", {
                                    // messages are ignored as B2 < A2
                                    recv_quorum_checks(
                                        &mut tk,
                                        &make_prepare_gen(
                                            qset_hash.into_inner(),
                                            b2.clone(),
                                            Some(b2.clone()),
                                            0,
                                            0,
                                            None,
                                        ),
                                        false,
                                        false,
                                    );
                                    assert_eq!(tk.driver().envelopes.len(), 5);
                                    assert!(!tk.driver().has_ballot_timer_upcoming());
                                });
                                section!(higher_counter, "higher counter", {
                                    recv_v_blocking(
                                        &mut tk,
                                        &make_prepare_gen(
                                            qset_hash.into_inner(),
                                            b3.clone(),
                                            Some(b2.clone()),
                                            2,
                                            2,
                                            None,
                                        ),
                                    );
                                    assert_eq!(tk.driver().envelopes.len(), 6);
                                    // A2 > B2 -> p = A2, p'=B2
                                    verify_prepare(
                                        &tk.driver().envelopes[5],
                                        &v0_secret_key,
                                        qset_hash0,
                                        0,
                                        a3.clone(),
                                        Some(a2.clone()),
                                        2,
                                        2,
                                        Some(b2.clone()),
                                    );
                                    assert!(!tk.driver().has_ballot_timer());

                                    // node is trying to commit A2=<2,y> but rest
                                    // of its quorum is trying to commit B2
                                    // we end up with a delayed quorum
                                    recv_quorum_checks_ex(
                                        &mut tk,
                                        &make_prepare_gen(
                                            qset_hash.into_inner(),
                                            b3.clone(),
                                            Some(b2.clone()),
                                            2,
                                            2,
                                            None,
                                        ),
                                        true,
                                        true,
                                        true,
                                    );
                                    assert_eq!(tk.driver().envelopes.len(), 7);
                                    verify_confirm(
                                        &tk.driver().envelopes[6],
                                        &v0_secret_key,
                                        qset_hash0,
                                        0,
                                        3,
                                        b3.clone(),
                                        2,
                                        2,
                                    );
                                });
                                section!(higher_counter_mixed, "higher counter mixed", {
                                    recv_v_blocking(
                                        &mut tk,
                                        &make_prepare_gen(
                                            qset_hash.into_inner(),
                                            a3.clone(),
                                            Some(b3.clone()),
                                            0,
                                            2,
                                            Some(a2.clone()),
                                        ),
                                    );
                                    assert_eq!(tk.driver().envelopes.len(), 6);
                                    // h still A2
                                    // v-blocking
                                    //     prepared B3 -> p = B3, p'=A2 (1)
                                    //     counter 3, b = A3 (9) (same value than h)
                                    // c = 0 (1)
                                    verify_prepare(
                                        &tk.driver().envelopes[5],
                                        &v0_secret_key,
                                        qset_hash0,
                                        0,
                                        a3.clone(),
                                        Some(b3.clone()),
                                        0,
                                        2,
                                        Some(a2.clone()),
                                    );
                                    recv_quorum_ex(
                                        &mut tk,
                                        &make_prepare_gen(
                                            qset_hash.into_inner(),
                                            a3.clone(),
                                            Some(b3.clone()),
                                            0,
                                            2,
                                            Some(a2.clone()),
                                        ),
                                        true,
                                    );
                                    // p=A3, p'=B3 (1)
                                    // computed_h = B3 (2) z = B - cannot update b
                                    assert_eq!(tk.driver().envelopes.len(), 7);
                                    verify_prepare(
                                        &tk.driver().envelopes[6],
                                        &v0_secret_key,
                                        qset_hash0,
                                        0,
                                        a3.clone(),
                                        Some(a3.clone()),
                                        0,
                                        2,
                                        Some(b3.clone()),
                                    );
                                    // timeout, bump to B4
                                    assert!(tk.driver().has_ballot_timer_upcoming());
                                    let cb = tk.driver().get_ballot_timer().unwrap();
                                    tk.proto().driver_calling_back(cb.callback);
                                    // computed_h = B3
                                    // h = B3 (2)
                                    // c = 0
                                    assert_eq!(tk.driver().envelopes.len(), 8);
                                    verify_prepare(
                                        &tk.driver().envelopes[7],
                                        &v0_secret_key,
                                        qset_hash0,
                                        0,
                                        b4.clone(),
                                        Some(a3.clone()),
                                        0,
                                        3,
                                        Some(b3.clone()),
                                    );
                                });
                            });
                        });

                        section!(confirm_prepared_mixed, "Confirm prepared mixed", {
                            // a few nodes prepared B2
                            recv_v_blocking(
                                &mut tk,
                                &make_prepare_gen(
                                    qset_hash.into_inner(),
                                    a2.clone(),
                                    Some(a2.clone()),
                                    0,
                                    0,
                                    Some(b2.clone()),
                                ),
                            );
                            assert_eq!(tk.driver().envelopes.len(), 5);
                            verify_prepare(
                                &tk.driver().envelopes[4],
                                &v0_secret_key,
                                qset_hash0,
                                0,
                                a2.clone(),
                                Some(a2.clone()),
                                0,
                                0,
                                Some(b2.clone()),
                            );
                            assert!(!tk.driver().has_ballot_timer_upcoming());

                            section!(mixed_a2, "mixed A2", {
                                // causes h=A2, c=A2
                                tk.driver().bump_timer_offset();
                                tk.proto().received_envelope(
                                    &make_prepare(
                                        &v3_secret_key,
                                        qset_hash.into_inner(),
                                        0,
                                        a2.clone(),
                                        Some(a2.clone()),
                                        0,
                                        0,
                                        None,
                                    )
                                    .unwrap(),
                                );

                                assert_eq!(tk.driver().envelopes.len(), 6);
                                verify_prepare(
                                    &tk.driver().envelopes[5],
                                    &v0_secret_key,
                                    qset_hash0,
                                    0,
                                    a2.clone(),
                                    Some(a2.clone()),
                                    2,
                                    2,
                                    Some(b2.clone()),
                                );
                                assert!(!tk.driver().has_ballot_timer_upcoming());

                                tk.driver().bump_timer_offset();
                                tk.proto().received_envelope(
                                    &make_prepare(
                                        &v4_secret_key,
                                        qset_hash.into_inner(),
                                        0,
                                        a2.clone(),
                                        Some(a2.clone()),
                                        0,
                                        0,
                                        None,
                                    )
                                    .unwrap(),
                                );

                                assert_eq!(tk.driver().envelopes.len(), 6);
                                assert!(!tk.driver().has_ballot_timer_upcoming());
                            });

                            section!(mixed_b2, "mixed B2", {
                                // causes computed_h=B2 ~ not set as h ~!= b
                                // -> noop
                                tk.driver().bump_timer_offset();
                                tk.proto().received_envelope(
                                    &make_prepare(
                                        &v3_secret_key,
                                        qset_hash.into_inner(),
                                        0,
                                        a2.clone(),
                                        Some(b2.clone()),
                                        0,
                                        0,
                                        None,
                                    )
                                    .unwrap(),
                                );

                                assert_eq!(tk.driver().envelopes.len(), 5);
                                assert!(!tk.driver().has_ballot_timer_upcoming());

                                tk.driver().bump_timer_offset();
                                tk.proto().received_envelope(
                                    &make_prepare(
                                        &v4_secret_key,
                                        qset_hash.into_inner(),
                                        0,
                                        b2.clone(),
                                        Some(b2.clone()),
                                        0,
                                        0,
                                        None,
                                    )
                                    .unwrap(),
                                );

                                assert_eq!(tk.driver().envelopes.len(), 5);
                                assert!(!tk.driver().has_ballot_timer_upcoming());
                            });
                        });
                    });

                    section!(switch_prepared_b1_from_a1, "switch prepared B1 from A1", {
                        // can't switch to B1
                        recv_quorum_checks(
                            &mut tk,
                            &make_prepare_gen(
                                qset_hash.into_inner(),
                                b1.clone(),
                                Some(b1.clone()),
                                0,
                                0,
                                None,
                            ),
                            false,
                            false,
                        );
                        assert_eq!(tk.driver().envelopes.len(), 2);
                        assert!(!tk.driver().has_ballot_timer_upcoming());
                    });
                    section!(switch_prepare_b1, "switch prepare B1", {
                        // doesn't switch as B1 < A1
                        recv_quorum_checks(
                            &mut tk,
                            &make_prepare_gen(qset_hash.into_inner(), b1.clone(), None, 0, 0, None),
                            false,
                            false,
                        );
                        assert_eq!(tk.driver().envelopes.len(), 2);
                        assert!(!tk.driver().has_ballot_timer_upcoming());
                    });
                    section!(
                        prepare_higher_counter_v_blocking,
                        "prepare higher counter (v-blocking)",
                        {
                            recv_v_blocking(
                                &mut tk,
                                &make_prepare_gen(
                                    qset_hash.into_inner(),
                                    b2.clone(),
                                    None,
                                    0,
                                    0,
                                    None,
                                ),
                            );
                            assert_eq!(tk.driver().envelopes.len(), 3);
                            verify_prepare(
                                &tk.driver().envelopes[2],
                                &v0_secret_key,
                                qset_hash0,
                                0,
                                a2.clone(),
                                Some(a1.clone()),
                                0,
                                0,
                                None,
                            );
                            assert!(!tk.driver().has_ballot_timer());

                            // more timeout from vBlocking set
                            recv_v_blocking(
                                &mut tk,
                                &make_prepare_gen(
                                    qset_hash.into_inner(),
                                    b3.clone(),
                                    None,
                                    0,
                                    0,
                                    None,
                                ),
                            );
                            assert_eq!(tk.driver().envelopes.len(), 4);
                            verify_prepare(
                                &tk.driver().envelopes[3],
                                &v0_secret_key,
                                qset_hash0,
                                0,
                                a3.clone(),
                                Some(a1.clone()),
                                0,
                                0,
                                None,
                            );
                            assert!(!tk.driver().has_ballot_timer());
                        }
                    );
                });

                section!(prepared_b_v_blocking, "prepared B (v-blocking)", {
                    recv_v_blocking(
                        &mut tk,
                        &make_prepare_gen(
                            qset_hash.into_inner(),
                            b1.clone(),
                            Some(b1.clone()),
                            0,
                            0,
                            None,
                        ),
                    );
                    assert_eq!(tk.driver().envelopes.len(), 2);
                    verify_prepare(
                        &tk.driver().envelopes[1],
                        &v0_secret_key,
                        qset_hash0,
                        0,
                        a1.clone(),
                        Some(b1.clone()),
                        0,
                        0,
                        None,
                    );
                    assert!(!tk.driver().has_ballot_timer());
                });
                section!(prepare_b_quorum, "prepare B (quorum)", {
                    recv_quorum_checks_ex(
                        &mut tk,
                        &make_prepare_gen(qset_hash.into_inner(), b1.clone(), None, 0, 0, None),
                        true,
                        true,
                        true,
                    );
                    assert_eq!(tk.driver().envelopes.len(), 2);
                    verify_prepare(
                        &tk.driver().envelopes[1],
                        &v0_secret_key,
                        qset_hash0,
                        0,
                        a1.clone(),
                        Some(b1.clone()),
                        0,
                        0,
                        None,
                    );
                });
                section!(confirm_v_blocking, "confirm (v-blocking)", {
                    section!(via_confirm, "via CONFIRM", {
                        tk.driver().bump_timer_offset();
                        tk.proto().received_envelope(
                            &make_confirm(
                                &v1_secret_key,
                                qset_hash.into_inner(),
                                0,
                                3,
                                a3.clone(),
                                3,
                                3,
                            )
                            .unwrap(),
                        );
                        tk.proto().received_envelope(
                            &make_confirm(
                                &v2_secret_key,
                                qset_hash.into_inner(),
                                0,
                                4,
                                a4.clone(),
                                2,
                                4,
                            )
                            .unwrap(),
                        );
                        assert_eq!(tk.driver().envelopes.len(), 2);
                        verify_confirm(
                            &tk.driver().envelopes[1],
                            &v0_secret_key,
                            qset_hash0,
                            0,
                            3,
                            a3.clone(),
                            3,
                            3,
                        );
                        assert!(!tk.driver().has_ballot_timer());
                    });
                    section!(via_externalize, "via EXTERNALIZE", {
                        tk.proto().received_envelope(
                            &make_externalize(
                                &v1_secret_key,
                                qset_hash.into_inner(),
                                0,
                                a2.clone(),
                                4,
                            )
                            .unwrap(),
                        );
                        tk.proto().received_envelope(
                            &make_externalize(
                                &v2_secret_key,
                                qset_hash.into_inner(),
                                0,
                                a3.clone(),
                                5,
                            )
                            .unwrap(),
                        );
                        assert_eq!(tk.driver().envelopes.len(), 2);
                        verify_confirm(
                            &tk.driver().envelopes[1],
                            &v0_secret_key,
                            qset_hash0,
                            0,
                            u32::MAX,
                            a_inf.clone(),
                            3,
                            u32::MAX,
                        );
                        assert!(!tk.driver().has_ballot_timer());
                    });
                });
            });

            // this is the same test suite than "start <1,x>" but only keeping
            // the transitions that are observable when starting from empty
            section!(start_pristine, "start from pristine", {
                let a_value = &values.x_value;
                let b_value = &values.z_value;

                let a1 = SCPBallot {
                    counter: 1,
                    value: a_value.clone(),
                };
                let b1 = SCPBallot {
                    counter: 1,
                    value: b_value.clone(),
                };

                let mut a2 = a1.clone();
                a2.counter += 1;

                let mut a3 = a2.clone();
                a3.counter += 1;

                let mut a4 = a3.clone();
                a4.counter += 1;

                let mut _a5 = a4.clone();
                _a5.counter += 1;

                let a_inf = SCPBallot {
                    counter: u32::MAX,
                    value: a_value.clone(),
                };
                let b_inf = SCPBallot {
                    counter: u32::MAX,
                    value: b_value.clone(),
                };

                let mut b2 = b1.clone();
                b2.counter += 1;

                let mut _b3 = b2.clone();
                _b3.counter += 1;

                assert_eq!(tk.driver().envelopes.len(), 0);

                section!(prepared_a1, "prepared A1", {
                    recv_quorum_checks(
                        &mut tk,
                        &make_prepare_gen(qset_hash.into_inner(), a1.clone(), None, 0, 0, None),
                        false,
                        false,
                    );
                    assert_eq!(tk.driver().envelopes.len(), 0);

                    section!(bump_prep_a2, "bump prepared A2", {
                        section!(confirm_prep_a2, "Confirm prepared A2", {
                            recv_v_blocking_checks(
                                &mut tk,
                                &make_prepare_gen(
                                    qset_hash.into_inner(),
                                    a2.clone(),
                                    Some(a2.clone()),
                                    0,
                                    0,
                                    None,
                                ),
                                false,
                            );
                            assert_eq!(tk.driver().envelopes.len(), 0);

                            section!(quorum_a2, "Quorum A2", {
                                recv_v_blocking_checks(
                                    &mut tk,
                                    &make_prepare_gen(
                                        qset_hash.into_inner(),
                                        a2.clone(),
                                        Some(a2.clone()),
                                        0,
                                        0,
                                        None,
                                    ),
                                    false,
                                );
                                assert_eq!(tk.driver().envelopes.len(), 0);
                                recv_quorum(
                                    &mut tk,
                                    &make_prepare_gen(
                                        qset_hash.into_inner(),
                                        a2.clone(),
                                        Some(a2.clone()),
                                        0,
                                        0,
                                        None,
                                    ),
                                );
                                assert_eq!(tk.driver().envelopes.len(), 1);
                                verify_prepare(
                                    &tk.driver().envelopes[0],
                                    &v0_secret_key,
                                    qset_hash0,
                                    0,
                                    a2.clone(),
                                    Some(a2.clone()),
                                    1,
                                    2,
                                    None,
                                );
                            });

                            section!(quorum_b2, "Quorum B2", {
                                recv_v_blocking_checks(
                                    &mut tk,
                                    &make_prepare_gen(
                                        qset_hash.into_inner(),
                                        b2.clone(),
                                        Some(b2.clone()),
                                        0,
                                        0,
                                        None,
                                    ),
                                    false,
                                );
                                assert_eq!(tk.driver().envelopes.len(), 0);
                                recv_quorum(
                                    &mut tk,
                                    &make_prepare_gen(
                                        qset_hash.into_inner(),
                                        b2.clone(),
                                        Some(b2.clone()),
                                        0,
                                        0,
                                        None,
                                    ),
                                );
                                assert_eq!(tk.driver().envelopes.len(), 1);
                                verify_prepare(
                                    &tk.driver().envelopes[0],
                                    &v0_secret_key,
                                    qset_hash0,
                                    0,
                                    b2.clone(),
                                    Some(b2.clone()),
                                    2,
                                    2,
                                    Some(a2.clone()),
                                );
                            });

                            section!(accept_commit, "Accept commit", {
                                section!(quorum_a2, "Quorum A2", {
                                    recv_quorum(
                                        &mut tk,
                                        &make_prepare_gen(
                                            qset_hash.into_inner(),
                                            a2.clone(),
                                            Some(a2.clone()),
                                            2,
                                            2,
                                            None,
                                        ),
                                    );
                                    assert_eq!(tk.driver().envelopes.len(), 1);
                                    verify_confirm(
                                        &tk.driver().envelopes[0],
                                        &v0_secret_key,
                                        qset_hash0,
                                        0,
                                        2,
                                        a2.clone(),
                                        2,
                                        2,
                                    );
                                });

                                section!(quorum_b2, "Quorum B2", {
                                    recv_quorum(
                                        &mut tk,
                                        &make_prepare_gen(
                                            qset_hash.into_inner(),
                                            b2.clone(),
                                            Some(b2.clone()),
                                            2,
                                            2,
                                            None,
                                        ),
                                    );
                                    assert_eq!(tk.driver().envelopes.len(), 1);
                                    verify_confirm(
                                        &tk.driver().envelopes[0],
                                        &v0_secret_key,
                                        qset_hash0,
                                        0,
                                        2,
                                        b2.clone(),
                                        2,
                                        2,
                                    );
                                });

                                section!(vblocking, "v-blocking", {
                                    section!(confirm, "CONFIRM", {
                                        section!(confirm_a2, "CONFIRM A2", {
                                            recv_v_blocking(
                                                &mut tk,
                                                &make_confirm_gen(
                                                    qset_hash.into_inner(),
                                                    2,
                                                    a2.clone(),
                                                    2,
                                                    2,
                                                ),
                                            );
                                            assert_eq!(tk.driver().envelopes.len(), 1);
                                            verify_confirm(
                                                &tk.driver().envelopes[0],
                                                &v0_secret_key,
                                                qset_hash0,
                                                0,
                                                2,
                                                a2.clone(),
                                                2,
                                                2,
                                            );
                                        });

                                        section!(confirm_a3_4, "CONFIRM A3..4", {
                                            recv_v_blocking(
                                                &mut tk,
                                                &make_confirm_gen(
                                                    qset_hash.into_inner(),
                                                    4,
                                                    a4.clone(),
                                                    3,
                                                    4,
                                                ),
                                            );
                                            assert_eq!(tk.driver().envelopes.len(), 1);
                                            verify_confirm(
                                                &tk.driver().envelopes[0],
                                                &v0_secret_key,
                                                qset_hash0,
                                                0,
                                                4,
                                                a4.clone(),
                                                3,
                                                4,
                                            );
                                        });

                                        section!(confirm_b2, "CONFIRM B2", {
                                            recv_v_blocking(
                                                &mut tk,
                                                &make_confirm_gen(
                                                    qset_hash.into_inner(),
                                                    2,
                                                    b2.clone(),
                                                    2,
                                                    2,
                                                ),
                                            );
                                            assert_eq!(tk.driver().envelopes.len(), 1);
                                            verify_confirm(
                                                &tk.driver().envelopes[0],
                                                &v0_secret_key,
                                                qset_hash0,
                                                0,
                                                2,
                                                b2.clone(),
                                                2,
                                                2,
                                            );
                                        });
                                    });

                                    section!(externalize, "EXTERNALIZE", {
                                        section!(externalize_a2, "EXTERNALIZE A2", {
                                            recv_v_blocking(
                                                &mut tk,
                                                &make_externalize_gen(
                                                    qset_hash.into_inner(),
                                                    a2.clone(),
                                                    2,
                                                ),
                                            );
                                            assert_eq!(tk.driver().envelopes.len(), 1);
                                            verify_confirm(
                                                &tk.driver().envelopes[0],
                                                &v0_secret_key,
                                                qset_hash0,
                                                0,
                                                u32::MAX,
                                                a_inf.clone(),
                                                2,
                                                u32::MAX,
                                            );
                                        });

                                        section!(externalize_b2, "EXTERNALIZE B2", {
                                            recv_v_blocking(
                                                &mut tk,
                                                &make_externalize_gen(
                                                    qset_hash.into_inner(),
                                                    b2.clone(),
                                                    2,
                                                ),
                                            );
                                            assert_eq!(tk.driver().envelopes.len(), 1);
                                            verify_confirm(
                                                &tk.driver().envelopes[0],
                                                &v0_secret_key,
                                                qset_hash0,
                                                0,
                                                u32::MAX,
                                                b_inf.clone(),
                                                2,
                                                u32::MAX,
                                            );
                                        });
                                    });
                                });
                            });
                        });

                        section!(confirm_prep_mixed, "Confirm prepared mixed", {
                            // a few nodes prepared A2
                            // causes p=A2
                            recv_v_blocking_checks(
                                &mut tk,
                                &make_prepare_gen(
                                    qset_hash.into_inner(),
                                    a2.clone(),
                                    Some(a2.clone()),
                                    0,
                                    0,
                                    None,
                                ),
                                false,
                            );
                            assert_eq!(tk.driver().envelopes.len(), 0);

                            // a few nodes prepared B2
                            // causes p=B2, p'=A2
                            recv_v_blocking_checks(
                                &mut tk,
                                &make_prepare_gen(
                                    qset_hash.into_inner(),
                                    a2.clone(),
                                    Some(b2.clone()),
                                    0,
                                    0,
                                    Some(a2.clone()),
                                ),
                                false,
                            );
                            assert_eq!(tk.driver().envelopes.len(), 0);

                            section!(mixed_a2, "mixed A2", {
                                // causes h=A2
                                // but c = 0, as p >!~ h
                                tk.proto().received_envelope(
                                    &make_prepare(
                                        &v3_secret_key,
                                        qset_hash.into_inner(),
                                        0,
                                        a2.clone(),
                                        Some(a2.clone()),
                                        0,
                                        0,
                                        None,
                                    )
                                    .unwrap(),
                                );

                                assert_eq!(tk.driver().envelopes.len(), 1);
                                verify_prepare(
                                    &tk.driver().envelopes[0],
                                    &v0_secret_key,
                                    qset_hash0,
                                    0,
                                    a2.clone(),
                                    Some(b2.clone()),
                                    0,
                                    2,
                                    Some(a2.clone()),
                                );

                                tk.proto().received_envelope(
                                    &make_prepare(
                                        &v4_secret_key,
                                        qset_hash.into_inner(),
                                        0,
                                        a2.clone(),
                                        Some(a2.clone()),
                                        0,
                                        0,
                                        None,
                                    )
                                    .unwrap(),
                                );

                                assert_eq!(tk.driver().envelopes.len(), 1);
                            });

                            section!(mixed_b2, "mixed B2", {
                                // causes h=B2, c=B2
                                tk.proto().received_envelope(
                                    &make_prepare(
                                        &v3_secret_key,
                                        qset_hash.into_inner(),
                                        0,
                                        b2.clone(),
                                        Some(b2.clone()),
                                        0,
                                        0,
                                        None,
                                    )
                                    .unwrap(),
                                );

                                assert_eq!(tk.driver().envelopes.len(), 1);
                                verify_prepare(
                                    &tk.driver().envelopes[0],
                                    &v0_secret_key,
                                    qset_hash0,
                                    0,
                                    b2.clone(),
                                    Some(b2.clone()),
                                    2,
                                    2,
                                    Some(a2.clone()),
                                );

                                tk.proto().received_envelope(
                                    &make_prepare(
                                        &v4_secret_key,
                                        qset_hash.into_inner(),
                                        0,
                                        b2.clone(),
                                        Some(b2.clone()),
                                        0,
                                        0,
                                        None,
                                    )
                                    .unwrap(),
                                );

                                assert_eq!(tk.driver().envelopes.len(), 1);
                            });
                        });
                    });

                    section!(switch_prep_b1, "switch prepared B1", {
                        recv_v_blocking_checks(
                            &mut tk,
                            &make_prepare_gen(
                                qset_hash.into_inner(),
                                b1.clone(),
                                Some(b1.clone()),
                                0,
                                0,
                                None,
                            ),
                            false,
                        );
                        assert_eq!(tk.driver().envelopes.len(), 0);
                    });
                });

                section!(prepared_b_vblocking, "prepared B (v-blocking)", {
                    recv_v_blocking_checks(
                        &mut tk,
                        &make_prepare_gen(
                            qset_hash.into_inner(),
                            b1.clone(),
                            Some(b1.clone()),
                            0,
                            0,
                            None,
                        ),
                        false,
                    );
                    assert_eq!(tk.driver().envelopes.len(), 0);
                });

                section!(confirm_vblocking, "confirm (v-blocking)", {
                    section!(via_confirm, "via CONFIRM", {
                        tk.proto().received_envelope(
                            &make_confirm(
                                &v1_secret_key,
                                qset_hash.into_inner(),
                                0,
                                3,
                                a3.clone(),
                                3,
                                3,
                            )
                            .unwrap(),
                        );
                        tk.proto().received_envelope(
                            &make_confirm(
                                &v2_secret_key,
                                qset_hash.into_inner(),
                                0,
                                4,
                                a4.clone(),
                                2,
                                4,
                            )
                            .unwrap(),
                        );
                        assert_eq!(tk.driver().envelopes.len(), 1);
                        verify_confirm(
                            &tk.driver().envelopes[0],
                            &v0_secret_key,
                            qset_hash0,
                            0,
                            3,
                            a3.clone(),
                            3,
                            3,
                        );
                    });

                    section!(via_externalize, "via EXTERNALIZE", {
                        tk.proto().received_envelope(
                            &make_externalize(
                                &v1_secret_key,
                                qset_hash.into_inner(),
                                0,
                                a2.clone(),
                                4,
                            )
                            .unwrap(),
                        );
                        tk.proto().received_envelope(
                            &make_externalize(
                                &v2_secret_key,
                                qset_hash.into_inner(),
                                0,
                                a3.clone(),
                                5,
                            )
                            .unwrap(),
                        );
                        assert_eq!(tk.driver().envelopes.len(), 1);
                        verify_confirm(
                            &tk.driver().envelopes[0],
                            &v0_secret_key,
                            qset_hash0,
                            0,
                            u32::MAX,
                            a_inf.clone(),
                            3,
                            u32::MAX,
                        );
                    });
                });
            });

            section!(normal_round_1x, "normal round (1,x)", {
                nodes_all_pledge_to_commit(&mut tk);
                assert_eq!(tk.driver().envelopes.len(), 3);

                let b = proto::SCPBallot {
                    counter: 1,
                    value: values.x_value.clone(),
                };

                // bunch of prepare messages with "commit b"
                let prepared_c1 = make_prepare(
                    &v1_secret_key,
                    qset_hash.into_inner(),
                    0,
                    b.clone(),
                    Some(b.clone()),
                    b.counter,
                    b.counter,
                    None,
                )
                .unwrap();
                let prepared_c2 = make_prepare(
                    &v2_secret_key,
                    qset_hash.into_inner(),
                    0,
                    b.clone(),
                    Some(b.clone()),
                    b.counter,
                    b.counter,
                    None,
                )
                .unwrap();
                let prepared_c3 = make_prepare(
                    &v3_secret_key,
                    qset_hash.into_inner(),
                    0,
                    b.clone(),
                    Some(b.clone()),
                    b.counter,
                    b.counter,
                    None,
                )
                .unwrap();
                let _prepared_c4 = make_prepare(
                    &v4_secret_key,
                    qset_hash.into_inner(),
                    0,
                    b.clone(),
                    Some(b.clone()),
                    b.counter,
                    b.counter,
                    None,
                )
                .unwrap();

                // those should not trigger anything just yet
                tk.proto().received_envelope(&prepared_c1);
                tk.proto().received_envelope(&prepared_c2);
                assert_eq!(tk.driver().envelopes.len(), 3);

                // this should cause the node to accept 'commit b' (quorum)
                // and therefore send a "CONFIRM" message
                tk.proto().received_envelope(&prepared_c3);
                assert_eq!(tk.driver().envelopes.len(), 4);

                verify_confirm(
                    &tk.driver().envelopes[3],
                    &v0_secret_key,
                    qset_hash0,
                    0,
                    1,
                    b.clone(),
                    b.counter,
                    b.counter,
                );

                // bunch of confirm messages
                let confirm1 = make_confirm(
                    &v1_secret_key,
                    qset_hash.into_inner(),
                    0,
                    b.counter,
                    b.clone(),
                    b.counter,
                    b.counter,
                )
                .unwrap();
                let confirm2 = make_confirm(
                    &v2_secret_key,
                    qset_hash.into_inner(),
                    0,
                    b.counter,
                    b.clone(),
                    b.counter,
                    b.counter,
                )
                .unwrap();
                let confirm3 = make_confirm(
                    &v3_secret_key,
                    qset_hash.into_inner(),
                    0,
                    b.counter,
                    b.clone(),
                    b.counter,
                    b.counter,
                )
                .unwrap();
                let confirm4 = make_confirm(
                    &v4_secret_key,
                    qset_hash.into_inner(),
                    0,
                    b.counter,
                    b.clone(),
                    b.counter,
                    b.counter,
                )
                .unwrap();

                // those should not trigger anything just yet
                tk.proto().received_envelope(&confirm1);
                tk.proto().received_envelope(&confirm2);
                assert_eq!(tk.driver().envelopes.len(), 4);

                tk.proto().received_envelope(&confirm3);
                // this causes our node to
                // externalize (confirm commit c)
                assert_eq!(tk.driver().envelopes.len(), 5);

                // The slot should have externalized the value
                assert_eq!(tk.driver().externalized_values.len(), 1);
                assert_eq!(
                    tk.driver().externalized_values.get(&0).unwrap(),
                    &values.x_value
                );

                verify_externalize(
                    &tk.driver().envelopes[4],
                    &v0_secret_key,
                    qset_hash0,
                    0,
                    b.clone(),
                    b.counter,
                );

                // extra vote should not do anything
                tk.proto().received_envelope(&confirm4);
                assert_eq!(tk.driver().envelopes.len(), 5);
                assert_eq!(tk.driver().externalized_values.len(), 1);

                // duplicate should just no-op
                tk.proto().received_envelope(&confirm2);
                assert_eq!(tk.driver().envelopes.len(), 5);
                assert_eq!(tk.driver().externalized_values.len(), 1);

                section!(
                    bump_prevented_committed,
                    "bumpToBallot prevented once committed",
                    {
                        section!(
                            by_value,
                            "bumpToBallot prevented once committed (by value)",
                            {
                                let b2 = proto::SCPBallot {
                                    counter: 1,
                                    value: values.z_value.clone(),
                                };

                                let confirm1_b2 = make_confirm(
                                    &v1_secret_key,
                                    qset_hash.into_inner(),
                                    0,
                                    b2.counter,
                                    b2.clone(),
                                    b2.counter,
                                    b2.counter,
                                )
                                .unwrap();
                                let confirm2_b2 = make_confirm(
                                    &v2_secret_key,
                                    qset_hash.into_inner(),
                                    0,
                                    b2.counter,
                                    b2.clone(),
                                    b2.counter,
                                    b2.counter,
                                )
                                .unwrap();
                                let confirm3_b2 = make_confirm(
                                    &v3_secret_key,
                                    qset_hash.into_inner(),
                                    0,
                                    b2.counter,
                                    b2.clone(),
                                    b2.counter,
                                    b2.counter,
                                )
                                .unwrap();
                                let confirm4_b2 = make_confirm(
                                    &v4_secret_key,
                                    qset_hash.into_inner(),
                                    0,
                                    b2.counter,
                                    b2.clone(),
                                    b2.counter,
                                    b2.counter,
                                )
                                .unwrap();

                                tk.proto().received_envelope(&confirm1_b2);
                                tk.proto().received_envelope(&confirm2_b2);
                                tk.proto().received_envelope(&confirm3_b2);
                                tk.proto().received_envelope(&confirm4_b2);
                                assert_eq!(tk.driver().envelopes.len(), 5);
                                assert_eq!(tk.driver().externalized_values.len(), 1);
                            }
                        );

                        section!(
                            by_counter,
                            "bumpToBallot prevented once committed (by counter)",
                            {
                                let b2 = proto::SCPBallot {
                                    counter: 2,
                                    value: values.x_value.clone(),
                                };

                                let confirm1_b2 = make_confirm(
                                    &v1_secret_key,
                                    qset_hash.into_inner(),
                                    0,
                                    b2.counter,
                                    b2.clone(),
                                    b2.counter,
                                    b2.counter,
                                )
                                .unwrap();
                                let confirm2_b2 = make_confirm(
                                    &v2_secret_key,
                                    qset_hash.into_inner(),
                                    0,
                                    b2.counter,
                                    b2.clone(),
                                    b2.counter,
                                    b2.counter,
                                )
                                .unwrap();
                                let confirm3_b2 = make_confirm(
                                    &v3_secret_key,
                                    qset_hash.into_inner(),
                                    0,
                                    b2.counter,
                                    b2.clone(),
                                    b2.counter,
                                    b2.counter,
                                )
                                .unwrap();
                                let confirm4_b2 = make_confirm(
                                    &v4_secret_key,
                                    qset_hash.into_inner(),
                                    0,
                                    b2.counter,
                                    b2.clone(),
                                    b2.counter,
                                    b2.counter,
                                )
                                .unwrap();

                                tk.proto().received_envelope(&confirm1_b2);
                                tk.proto().received_envelope(&confirm2_b2);
                                tk.proto().received_envelope(&confirm3_b2);
                                tk.proto().received_envelope(&confirm4_b2);
                                assert_eq!(tk.driver().envelopes.len(), 5);
                                assert_eq!(tk.driver().externalized_values.len(), 1);
                            }
                        );

                        section!(
                            by_value_and_counter,
                            "bumpToBallot prevented once committed (by value and counter)",
                            {
                                let b2 = proto::SCPBallot {
                                    counter: 2,
                                    value: values.z_value.clone(),
                                };

                                let confirm1_b2 = make_confirm(
                                    &v1_secret_key,
                                    qset_hash.into_inner(),
                                    0,
                                    b2.counter,
                                    b2.clone(),
                                    b2.counter,
                                    b2.counter,
                                )
                                .unwrap();
                                let confirm2_b2 = make_confirm(
                                    &v2_secret_key,
                                    qset_hash.into_inner(),
                                    0,
                                    b2.counter,
                                    b2.clone(),
                                    b2.counter,
                                    b2.counter,
                                )
                                .unwrap();
                                let confirm3_b2 = make_confirm(
                                    &v3_secret_key,
                                    qset_hash.into_inner(),
                                    0,
                                    b2.counter,
                                    b2.clone(),
                                    b2.counter,
                                    b2.counter,
                                )
                                .unwrap();
                                let confirm4_b2 = make_confirm(
                                    &v4_secret_key,
                                    qset_hash.into_inner(),
                                    0,
                                    b2.counter,
                                    b2.clone(),
                                    b2.counter,
                                    b2.counter,
                                )
                                .unwrap();

                                tk.proto().received_envelope(&confirm1_b2);
                                tk.proto().received_envelope(&confirm2_b2);
                                tk.proto().received_envelope(&confirm3_b2);
                                tk.proto().received_envelope(&confirm4_b2);
                                assert_eq!(tk.driver().envelopes.len(), 5);
                                assert_eq!(tk.driver().externalized_values.len(), 1);
                            }
                        );
                    }
                );
            });

            section!(range_check, "range check", {
                nodes_all_pledge_to_commit(&mut tk);
                assert_eq!(tk.driver().envelopes.len(), 3);

                let b = proto::SCPBallot {
                    counter: 1,
                    value: values.x_value.clone(),
                };

                // bunch of prepare messages with "commit b"
                let prepared_c1 = make_prepare(
                    &v1_secret_key,
                    qset_hash.into_inner(),
                    0,
                    b.clone(),
                    Some(b.clone()),
                    b.counter,
                    b.counter,
                    None,
                )
                .unwrap();
                let prepared_c2 = make_prepare(
                    &v2_secret_key,
                    qset_hash.into_inner(),
                    0,
                    b.clone(),
                    Some(b.clone()),
                    b.counter,
                    b.counter,
                    None,
                )
                .unwrap();
                let prepared_c3 = make_prepare(
                    &v3_secret_key,
                    qset_hash.into_inner(),
                    0,
                    b.clone(),
                    Some(b.clone()),
                    b.counter,
                    b.counter,
                    None,
                )
                .unwrap();
                let _prepared_c4 = make_prepare(
                    &v4_secret_key,
                    qset_hash.into_inner(),
                    0,
                    b.clone(),
                    Some(b.clone()),
                    b.counter,
                    b.counter,
                    None,
                )
                .unwrap();

                // those should not trigger anything just yet
                tk.proto().received_envelope(&prepared_c1);
                tk.proto().received_envelope(&prepared_c2);
                assert_eq!(tk.driver().envelopes.len(), 3);

                // this should cause the node to accept 'commit b' (quorum)
                // and therefore send a "CONFIRM" message
                tk.proto().received_envelope(&prepared_c3);
                assert_eq!(tk.driver().envelopes.len(), 4);

                verify_confirm(
                    &tk.driver().envelopes[3],
                    &v0_secret_key,
                    qset_hash0,
                    0,
                    1,
                    b.clone(),
                    b.counter,
                    b.counter,
                );

                // bunch of confirm messages with different ranges
                let confirm1 = make_confirm(
                    &v1_secret_key,
                    qset_hash.into_inner(),
                    0,
                    4,
                    proto::SCPBallot {
                        counter: 4,
                        value: values.x_value.clone(),
                    },
                    2,
                    4,
                )
                .unwrap();
                let confirm2 = make_confirm(
                    &v2_secret_key,
                    qset_hash.into_inner(),
                    0,
                    6,
                    proto::SCPBallot {
                        counter: 6,
                        value: values.x_value.clone(),
                    },
                    2,
                    6,
                )
                .unwrap();
                let confirm3 = make_confirm(
                    &v3_secret_key,
                    qset_hash.into_inner(),
                    0,
                    5,
                    proto::SCPBallot {
                        counter: 5,
                        value: values.x_value.clone(),
                    },
                    3,
                    5,
                )
                .unwrap();
                let confirm4 = make_confirm(
                    &v4_secret_key,
                    qset_hash.into_inner(),
                    0,
                    6,
                    proto::SCPBallot {
                        counter: 6,
                        value: values.x_value.clone(),
                    },
                    3,
                    6,
                )
                .unwrap();

                // this should not trigger anything just yet
                tk.proto().received_envelope(&confirm1);

                // v-blocking
                //   * b gets bumped to (4,x)
                //   * p gets bumped to (4,x)
                //   * (c,h) gets bumped to (2,4)
                tk.proto().received_envelope(&confirm2);
                assert_eq!(tk.driver().envelopes.len(), 5);
                verify_confirm(
                    &tk.driver().envelopes[4],
                    &v0_secret_key,
                    qset_hash0,
                    0,
                    4,
                    proto::SCPBallot {
                        counter: 4,
                        value: values.x_value.clone(),
                    },
                    2,
                    4,
                );

                // this causes to externalize
                // range is [3,4]
                tk.proto().received_envelope(&confirm4);
                assert_eq!(tk.driver().envelopes.len(), 6);

                // The slot should have externalized the value
                assert_eq!(tk.driver().externalized_values.len(), 1);
                assert_eq!(
                    tk.driver().externalized_values.get(&0).unwrap(),
                    &values.x_value
                );

                verify_externalize(
                    &tk.driver().envelopes[5],
                    &v0_secret_key,
                    qset_hash0,
                    0,
                    proto::SCPBallot {
                        counter: 3,
                        value: values.x_value.clone(),
                    },
                    4,
                );
            });

            section!(
                timeout_h_set,
                "timeout when h is set -> stay locked on h",
                {
                    let bx = proto::SCPBallot {
                        counter: 1,
                        value: values.x_value.clone(),
                    };
                    assert!(tk.bump_state(0, &values.x_value));
                    assert_eq!(tk.driver().envelopes.len(), 1);

                    // v-blocking -> prepared
                    // quorum -> confirm prepared
                    recv_quorum(
                        &mut tk,
                        &make_prepare_gen(
                            qset_hash.into_inner(),
                            bx.clone(),
                            Some(bx.clone()),
                            0,
                            0,
                            None,
                        ),
                    );
                    assert_eq!(tk.driver().envelopes.len(), 3);
                    verify_prepare(
                        &tk.driver().envelopes[2],
                        &v0_secret_key,
                        qset_hash0,
                        0,
                        bx.clone(),
                        Some(bx.clone()),
                        bx.counter,
                        bx.counter,
                        None,
                    );

                    // now, see if we can timeout and move to a different value
                    assert!(tk.bump_state(0, &values.y_value));
                    assert_eq!(tk.driver().envelopes.len(), 4);
                    let newbx = proto::SCPBallot {
                        counter: 2,
                        value: values.x_value.clone(),
                    };
                    verify_prepare(
                        &tk.driver().envelopes[3],
                        &v0_secret_key,
                        qset_hash0,
                        0,
                        newbx,
                        Some(bx.clone()),
                        bx.counter,
                        bx.counter,
                        None,
                    );
                }
            );

            section!(
                timeout_h_cant_set,
                "timeout when h exists but can't be set -> vote for h",
                {
                    // start with (1,y)
                    let by = proto::SCPBallot {
                        counter: 1,
                        value: values.y_value.clone(),
                    };
                    assert!(tk.bump_state(0, &values.y_value));
                    assert_eq!(tk.driver().envelopes.len(), 1);

                    let bx = proto::SCPBallot {
                        counter: 1,
                        value: values.x_value.clone(),
                    };
                    // but quorum goes with (1,x)
                    // v-blocking -> prepared
                    recv_v_blocking(
                        &mut tk,
                        &make_prepare_gen(
                            qset_hash.into_inner(),
                            bx.clone(),
                            Some(bx.clone()),
                            0,
                            0,
                            None,
                        ),
                    );
                    assert_eq!(tk.driver().envelopes.len(), 2);
                    verify_prepare(
                        &tk.driver().envelopes[1],
                        &v0_secret_key,
                        qset_hash0,
                        0,
                        by.clone(),
                        Some(bx.clone()),
                        0,
                        0,
                        None,
                    );
                    // quorum -> confirm prepared (no-op as b > h)
                    recv_quorum_checks(
                        &mut tk,
                        &make_prepare_gen(
                            qset_hash.into_inner(),
                            bx.clone(),
                            Some(bx.clone()),
                            0,
                            0,
                            None,
                        ),
                        false,
                        false,
                    );
                    assert_eq!(tk.driver().envelopes.len(), 2);

                    assert!(tk.bump_state(0, &values.y_value));
                    assert_eq!(tk.driver().envelopes.len(), 3);
                    let newbx = proto::SCPBallot {
                        counter: 2,
                        value: values.x_value.clone(),
                    };
                    // on timeout:
                    // * we should move to the quorum's h value
                    // * c can't be set yet as b > h
                    verify_prepare(
                        &tk.driver().envelopes[2],
                        &v0_secret_key,
                        qset_hash0,
                        0,
                        newbx,
                        Some(bx.clone()),
                        0,
                        bx.counter,
                        None,
                    );
                }
            );

            section!(timeout_multiple_nodes, "timeout from multiple nodes", {
                assert!(tk.bump_state(0, &values.x_value));

                let x1 = proto::SCPBallot {
                    counter: 1,
                    value: values.x_value.clone(),
                };

                assert_eq!(tk.driver().envelopes.len(), 1);
                verify_prepare(
                    &tk.driver().envelopes[0],
                    &v0_secret_key,
                    qset_hash0,
                    0,
                    x1.clone(),
                    None,
                    0,
                    0,
                    None,
                );

                recv_quorum(
                    &mut tk,
                    &make_prepare_gen(qset_hash.into_inner(), x1.clone(), None, 0, 0, None),
                );
                // quorum -> prepared (1,x)
                assert_eq!(tk.driver().envelopes.len(), 2);
                verify_prepare(
                    &tk.driver().envelopes[1],
                    &v0_secret_key,
                    qset_hash0,
                    0,
                    x1.clone(),
                    Some(x1.clone()),
                    0,
                    0,
                    None,
                );

                let x2 = proto::SCPBallot {
                    counter: 2,
                    value: values.x_value.clone(),
                };
                // timeout from local node
                assert!(tk.bump_state(0, &values.x_value));
                // prepares (2,x)
                assert_eq!(tk.driver().envelopes.len(), 3);
                verify_prepare(
                    &tk.driver().envelopes[2],
                    &v0_secret_key,
                    qset_hash0,
                    0,
                    x2.clone(),
                    Some(x1.clone()),
                    0,
                    0,
                    None,
                );

                recv_quorum(
                    &mut tk,
                    &make_prepare_gen(
                        qset_hash.into_inner(),
                        x1.clone(),
                        Some(x1.clone()),
                        0,
                        0,
                        None,
                    ),
                );
                // quorum -> set nH=1
                assert_eq!(tk.driver().envelopes.len(), 4);
                verify_prepare(
                    &tk.driver().envelopes[3],
                    &v0_secret_key,
                    qset_hash0,
                    0,
                    x2.clone(),
                    Some(x1.clone()),
                    0,
                    1,
                    None,
                );
                assert_eq!(tk.driver().envelopes.len(), 4);

                recv_v_blocking(
                    &mut tk,
                    &make_prepare_gen(
                        qset_hash.into_inner(),
                        x2.clone(),
                        Some(x2.clone()),
                        1,
                        1,
                        None,
                    ),
                );
                // v-blocking prepared (2,x) -> prepared (2,x)
                assert_eq!(tk.driver().envelopes.len(), 5);
                verify_prepare(
                    &tk.driver().envelopes[4],
                    &v0_secret_key,
                    qset_hash0,
                    0,
                    x2.clone(),
                    Some(x2.clone()),
                    0,
                    1,
                    None,
                );

                recv_quorum(
                    &mut tk,
                    &make_prepare_gen(
                        qset_hash.into_inner(),
                        x2.clone(),
                        Some(x2.clone()),
                        1,
                        1,
                        None,
                    ),
                );
                // quorum (including us) confirms (2,x) prepared -> set h=c=x2
                // we also get extra message: a quorum not including us confirms
                // (1,x) prepared
                //  -> we confirm c=h=x1
                assert_eq!(tk.driver().envelopes.len(), 7);
                verify_prepare(
                    &tk.driver().envelopes[5],
                    &v0_secret_key,
                    qset_hash0,
                    0,
                    x2.clone(),
                    Some(x2.clone()),
                    2,
                    2,
                    None,
                );
                verify_confirm(
                    &tk.driver().envelopes[6],
                    &v0_secret_key,
                    qset_hash0,
                    0,
                    2,
                    x2.clone(),
                    1,
                    1,
                );
            });

            section!(
                timeout_after_prepare,
                "timeout after prepare, receive old messages to prepare",
                {
                    assert!(tk.bump_state(0, &values.x_value));

                    let x1 = proto::SCPBallot {
                        counter: 1,
                        value: values.x_value.clone(),
                    };

                    assert_eq!(tk.driver().envelopes.len(), 1);
                    verify_prepare(
                        &tk.driver().envelopes[0],
                        &v0_secret_key,
                        qset_hash0,
                        0,
                        x1.clone(),
                        None,
                        0,
                        0,
                        None,
                    );

                    tk.proto().received_envelope(
                        &make_prepare(
                            &v1_secret_key,
                            qset_hash.into_inner(),
                            0,
                            x1.clone(),
                            None,
                            0,
                            0,
                            None,
                        )
                        .unwrap(),
                    );
                    tk.proto().received_envelope(
                        &make_prepare(
                            &v2_secret_key,
                            qset_hash.into_inner(),
                            0,
                            x1.clone(),
                            None,
                            0,
                            0,
                            None,
                        )
                        .unwrap(),
                    );
                    tk.proto().received_envelope(
                        &make_prepare(
                            &v3_secret_key,
                            qset_hash.into_inner(),
                            0,
                            x1.clone(),
                            None,
                            0,
                            0,
                            None,
                        )
                        .unwrap(),
                    );

                    // quorum -> prepared (1,x)
                    assert_eq!(tk.driver().envelopes.len(), 2);
                    verify_prepare(
                        &tk.driver().envelopes[1],
                        &v0_secret_key,
                        qset_hash0,
                        0,
                        x1.clone(),
                        Some(x1.clone()),
                        0,
                        0,
                        None,
                    );

                    let x2 = proto::SCPBallot {
                        counter: 2,
                        value: values.x_value.clone(),
                    };
                    // timeout from local node
                    assert!(tk.bump_state(0, &values.x_value));
                    // prepares (2,x)
                    assert_eq!(tk.driver().envelopes.len(), 3);
                    verify_prepare(
                        &tk.driver().envelopes[2],
                        &v0_secret_key,
                        qset_hash0,
                        0,
                        x2.clone(),
                        Some(x1.clone()),
                        0,
                        0,
                        None,
                    );

                    let x3 = proto::SCPBallot {
                        counter: 3,
                        value: values.x_value.clone(),
                    };
                    // timeout again
                    assert!(tk.bump_state(0, &values.x_value));
                    // prepares (3,x)
                    assert_eq!(tk.driver().envelopes.len(), 4);
                    verify_prepare(
                        &tk.driver().envelopes[3],
                        &v0_secret_key,
                        qset_hash0,
                        0,
                        x3.clone(),
                        Some(x1.clone()),
                        0,
                        0,
                        None,
                    );

                    // other nodes moved on with x2
                    tk.proto().received_envelope(
                        &make_prepare(
                            &v1_secret_key,
                            qset_hash.into_inner(),
                            0,
                            x2.clone(),
                            Some(x2.clone()),
                            1,
                            2,
                            None,
                        )
                        .unwrap(),
                    );
                    tk.proto().received_envelope(
                        &make_prepare(
                            &v2_secret_key,
                            qset_hash.into_inner(),
                            0,
                            x2.clone(),
                            Some(x2.clone()),
                            1,
                            2,
                            None,
                        )
                        .unwrap(),
                    );
                    // v-blocking -> prepared x2
                    assert_eq!(tk.driver().envelopes.len(), 5);
                    verify_prepare(
                        &tk.driver().envelopes[4],
                        &v0_secret_key,
                        qset_hash0,
                        0,
                        x3.clone(),
                        Some(x2.clone()),
                        0,
                        0,
                        None,
                    );

                    tk.proto().received_envelope(
                        &make_prepare(
                            &v3_secret_key,
                            qset_hash.into_inner(),
                            0,
                            x2.clone(),
                            Some(x2.clone()),
                            1,
                            2,
                            None,
                        )
                        .unwrap(),
                    );
                    // quorum -> set nH=2
                    assert_eq!(tk.driver().envelopes.len(), 6);
                    verify_prepare(
                        &tk.driver().envelopes[5],
                        &v0_secret_key,
                        qset_hash0,
                        0,
                        x3.clone(),
                        Some(x2.clone()),
                        0,
                        2,
                        None,
                    );
                }
            );

            section!(
                non_validator_watching,
                "non validator watching the network",
                {
                    // Create a non-validator node
                    let nv_node = TestNode::new(5); // NV = 5th node
                    let mut tk_nv =
                        TestKernel::new(nv_node.node_id().clone(), q_set.clone(), false);
                    tk_nv.driver().store_quorum_set(&q_set);
                    let qset_hash_nv = tk_nv.proto().local_proto().get_qset_hash().clone();

                    let b = proto::SCPBallot {
                        counter: 1,
                        value: values.x_value.clone(),
                    };
                    assert!(tk_nv.bump_state(0, &values.x_value));
                    assert_eq!(tk_nv.driver().envelopes.len(), 0);

                    // Non-validator should not emit envelopes but track state
                    let current_env = tk_nv.get_current_envelope(0, &nv_node.node_id());
                    verify_prepare(
                        &current_env,
                        &nv_node.secret_key,
                        qset_hash_nv,
                        0,
                        b.clone(),
                        None,
                        0,
                        0,
                        None,
                    );

                    let ext1 =
                        make_externalize(&v1_secret_key, qset_hash.into_inner(), 0, b.clone(), 1)
                            .unwrap();
                    let ext2 =
                        make_externalize(&v2_secret_key, qset_hash.into_inner(), 0, b.clone(), 1)
                            .unwrap();
                    let ext3 =
                        make_externalize(&v3_secret_key, qset_hash.into_inner(), 0, b.clone(), 1)
                            .unwrap();
                    let ext4 =
                        make_externalize(&v4_secret_key, qset_hash.into_inner(), 0, b.clone(), 1)
                            .unwrap();

                    tk_nv.proto().received_envelope(&ext1);
                    tk_nv.proto().received_envelope(&ext2);
                    tk_nv.proto().received_envelope(&ext3);
                    assert_eq!(tk_nv.driver().envelopes.len(), 0);

                    let current_env = tk_nv.get_current_envelope(0, &nv_node.node_id());
                    verify_confirm(
                        &current_env,
                        &nv_node.secret_key,
                        qset_hash_nv,
                        0,
                        u32::MAX,
                        proto::SCPBallot {
                            counter: u32::MAX,
                            value: values.x_value.clone(),
                        },
                        1,
                        u32::MAX,
                    );

                    tk_nv.proto().received_envelope(&ext4);
                    assert_eq!(tk_nv.driver().envelopes.len(), 0);

                    let current_env = tk_nv.get_current_envelope(0, &nv_node.node_id());
                    verify_externalize(
                        &current_env,
                        &nv_node.secret_key,
                        qset_hash_nv,
                        0,
                        b.clone(),
                        u32::MAX,
                    );
                    assert_eq!(
                        tk_nv.driver().externalized_values.get(&0).unwrap(),
                        &values.x_value
                    );
                }
            );

            section!(restore_ballot_protocol, "restore ballot protocol", {
                let b = proto::SCPBallot {
                    counter: 2,
                    value: values.x_value.clone(),
                };

                let mut tk2 = TestKernel::new(nodes[0].node_id().clone(), q_set.clone(), true);
                tk2.driver().store_quorum_set(&q_set);

                section!(restore_prepare, "prepare", {
                    let prepare_env =
                        make_prepare(&v0_secret_key, qset_hash0, 0, b.clone(), None, 0, 0, None)
                            .unwrap();
                    tk2.proto().set_state_from_envelope(0, &prepare_env);
                });

                section!(restore_confirm, "confirm", {
                    let confirm_env =
                        make_confirm(&v0_secret_key, qset_hash0, 0, 2, b.clone(), 1, 2).unwrap();
                    tk2.proto().set_state_from_envelope(0, &confirm_env);
                });

                section!(restore_externalize, "externalize", {
                    let externalize_env =
                        make_externalize(&v0_secret_key, qset_hash0, 0, b.clone(), 2).unwrap();
                    tk2.proto().set_state_from_envelope(0, &externalize_env);
                });
            });
        };

        for net_cfg in get_network_cfgs() {
            test(&net_cfg);
        }
    }
}
