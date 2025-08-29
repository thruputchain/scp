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
                    recv_quorum_checks_ex(tk, gen_, with_checks, delayed_quorum, true)
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
                assert_eq!(tk.driver().heard_from_quorums.get(&0).unwrap().len(), 0);

                tk.proto().received_envelope(&prepare2);
                assert_eq!(tk.driver().envelopes.len(), 1);
                assert_eq!(tk.driver().heard_from_quorums.get(&0).unwrap().len(), 0);

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
                                            //956
                                        });
                                    });
                                });
                            });
                        });
                    });
                });
            });
        };

        for net_cfg in get_network_cfgs() {
            test(&net_cfg);
        }
    }
}
