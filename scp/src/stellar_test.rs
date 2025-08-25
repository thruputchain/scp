#[cfg(test)]
mod stellar_tests {
    use crate::testkernel::*;
    use crate::testutil::*;
    use crate::*;
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

    struct NetworkConfig {
        NOMINATION_TIMEOUT_INITIAL_MILLISECONDS: u32,
        NOMINATION_TIMEOUT_INCREMENT_MILLISECONDS: u32,
        BALLOT_TIMEOUT_INITIAL_MILLISECONDS: u32,
        BALLOT_TIMEOUT_INCREMENT_MILLISECONDS: u32,
    }

    const MinNetworkConfig: NetworkConfig = NetworkConfig {
        NOMINATION_TIMEOUT_INITIAL_MILLISECONDS: 750,
        NOMINATION_TIMEOUT_INCREMENT_MILLISECONDS: 750,
        BALLOT_TIMEOUT_INITIAL_MILLISECONDS: 750,
        BALLOT_TIMEOUT_INCREMENT_MILLISECONDS: 750,
    };

    const MaxNetworkConfig: NetworkConfig = NetworkConfig {
        NOMINATION_TIMEOUT_INITIAL_MILLISECONDS: 2500,
        NOMINATION_TIMEOUT_INCREMENT_MILLISECONDS: 2000,
        BALLOT_TIMEOUT_INITIAL_MILLISECONDS: 2500,
        BALLOT_TIMEOUT_INCREMENT_MILLISECONDS: 2000,
    };

    const InitNetworkConfig: NetworkConfig = NetworkConfig {
        NOMINATION_TIMEOUT_INITIAL_MILLISECONDS: 1000,
        NOMINATION_TIMEOUT_INCREMENT_MILLISECONDS: 1000,
        BALLOT_TIMEOUT_INITIAL_MILLISECONDS: 1000,
        BALLOT_TIMEOUT_INCREMENT_MILLISECONDS: 1000,
    };

    trait Fixture: Default {
        fn tk<'a>(&'a mut self) -> &'a mut TestKernel;
    }

    // Testing matrix that covers interesting min/max values for each timeout
    // parameter
    fn test_timeouts<X: Fixture, F>(f: F)
    where
        F: Fn(X),
    {
        // minimum values
        let mut xmin = X::default();
        xmin.tk().driver().initial_nomination_timeout_ms =
            MinNetworkConfig.NOMINATION_TIMEOUT_INITIAL_MILLISECONDS;
        xmin.tk().driver().initial_ballot_timeout_ms =
            MinNetworkConfig.BALLOT_TIMEOUT_INITIAL_MILLISECONDS;
        xmin.tk().driver().increment_nomination_timeout_ms =
            MinNetworkConfig.NOMINATION_TIMEOUT_INCREMENT_MILLISECONDS;
        xmin.tk().driver().increment_ballot_timeout_ms =
            MinNetworkConfig.BALLOT_TIMEOUT_INCREMENT_MILLISECONDS;
        f(xmin);

        // initial values
        let mut xinit = X::default();
        xinit.tk().driver().initial_nomination_timeout_ms =
            InitNetworkConfig.NOMINATION_TIMEOUT_INITIAL_MILLISECONDS;
        xinit.tk().driver().initial_ballot_timeout_ms =
            InitNetworkConfig.BALLOT_TIMEOUT_INITIAL_MILLISECONDS;
        xinit.tk().driver().increment_nomination_timeout_ms =
            InitNetworkConfig.NOMINATION_TIMEOUT_INCREMENT_MILLISECONDS;
        xinit.tk().driver().increment_ballot_timeout_ms =
            InitNetworkConfig.BALLOT_TIMEOUT_INCREMENT_MILLISECONDS;
        f(xinit);

        // maximum values
        let mut xmax = X::default();
        xmax.tk().driver().initial_nomination_timeout_ms =
            MaxNetworkConfig.NOMINATION_TIMEOUT_INITIAL_MILLISECONDS;
        xmax.tk().driver().initial_ballot_timeout_ms =
            MaxNetworkConfig.BALLOT_TIMEOUT_INITIAL_MILLISECONDS;
        xmax.tk().driver().increment_nomination_timeout_ms =
            MaxNetworkConfig.NOMINATION_TIMEOUT_INCREMENT_MILLISECONDS;
        xmax.tk().driver().increment_ballot_timeout_ms =
            MaxNetworkConfig.BALLOT_TIMEOUT_INCREMENT_MILLISECONDS;
        f(xmax);
    }

    #[test]
    fn ballot_protocol_core5() {
        struct Fix0 {
            values: TestValues,
            v0_secret_key: SecretKey,
            v1_secret_key: SecretKey,
            v2_secret_key: SecretKey,
            v3_secret_key: SecretKey,
            v4_secret_key: SecretKey,
            v0_node_id: PublicKey,
            v1_node_id: PublicKey,
            v2_node_id: PublicKey,
            v3_node_id: PublicKey,
            v4_node_id: PublicKey,
            q_set: SCPQuorumSet,
            qset_hash: SHA256Hash,
            tk: TestKernel,
        }
        impl Fixture for Fix0 {
            fn tk<'a>(&'a mut self) -> &'a mut TestKernel {
                &mut self.tk
            }
        }
        impl Default for Fix0 {
            fn default() -> Fix0 {
                let values = TestValues::default();
                let nodes = TestNode::new_batch(5);
                let q_set = SCPQuorumSet {
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
                let qset_hash = proto::hash_xdr(&q_set).unwrap();
                let tk = TestKernel::new(nodes[0].node_id().clone(), q_set.clone(), true);
                Fix0 {
                    values: values,
                    v0_secret_key: nodes[0].secret_key.clone(),
                    v1_secret_key: nodes[1].secret_key.clone(),
                    v2_secret_key: nodes[2].secret_key.clone(),
                    v3_secret_key: nodes[3].secret_key.clone(),
                    v4_secret_key: nodes[4].secret_key.clone(),
                    v0_node_id: nodes[0].node_id(),
                    v1_node_id: nodes[1].node_id(),
                    v2_node_id: nodes[2].node_id(),
                    v3_node_id: nodes[3].node_id(),
                    v4_node_id: nodes[4].node_id(),
                    q_set: q_set,
                    qset_hash: qset_hash,
                    tk: tk,
                }
            }
        }

        struct Fix1 {
            x: Fix0,
        }
        impl Fix1 {
            fn new(mut x: Fix0) -> Fix1 {
                x.tk.driver().store_quorum_set(&x.q_set);

                // Verify value ordering
                assert!(x.values.x_value < x.values.y_value);
                assert!(x.values.y_value < x.values.z_value);
                assert!(x.values.z_value < x.values.zz_value);

                println!("");
                println!("BEGIN TEST");

                Fix1 { x }
            }
            fn qset_hash0(&mut self) -> proto::SCPHash {
                self.x.tk.proto().local_proto().get_qset_hash().clone()
            }
            fn recv_v_blocking_checks(&mut self, gen_: &GenEnvelope, with_checks: bool) {
                let e1 = gen_(&self.x.v1_secret_key);
                let e2 = gen_(&self.x.v2_secret_key);

                self.x.tk.driver().bump_timer_offset();

                // Nothing should happen with first message
                let i = self.x.tk.driver().envelopes.len();
                self.x.tk.proto().received_envelope(&e1);
                if with_checks {
                    assert_eq!(self.x.tk.driver().envelopes.len(), i);
                }
                let i = i + 1;
                self.x.tk.proto().received_envelope(&e2);
                if with_checks {
                    assert_eq!(self.x.tk.driver().envelopes.len(), i);
                }
            }

            fn recv_v_blocking(&mut self, gen_: &GenEnvelope) {
                self.recv_v_blocking_checks(gen_, true)
            }

            fn recv_quorum_checks_ex(
                &mut self,
                gen_: &GenEnvelope,
                with_checks: bool,
                delayed_quorum: bool,
                check_upcoming: bool,
            ) {
                let e1 = gen_(&self.x.v1_secret_key);
                let e2 = gen_(&self.x.v2_secret_key);
                let e3 = gen_(&self.x.v3_secret_key);
                let e4 = gen_(&self.x.v4_secret_key);

                self.x.tk.driver().bump_timer_offset();

                self.x.tk.proto().received_envelope(&e1);
                self.x.tk.proto().received_envelope(&e2);
                let i = self.x.tk.driver().envelopes.len() + 1;
                self.x.tk.proto().received_envelope(&e3);
                if with_checks && !delayed_quorum {
                    assert_eq!(self.x.tk.driver().envelopes.len(), i);
                }
                if check_upcoming && !delayed_quorum {
                    self.x.tk.driver().has_ballot_timer_upcoming();
                }
                // Nothing happens with an extra vote (unless we're in delayed_quorum)
                self.x.tk.proto().received_envelope(&e4);
                if with_checks && delayed_quorum {
                    assert_eq!(self.x.tk.driver().envelopes.len(), i);
                }
                if check_upcoming && delayed_quorum {
                    self.x.tk.driver().has_ballot_timer_upcoming();
                }
            }

            fn recv_quorum_checks(
                &mut self,
                gen_: &GenEnvelope,
                with_checks: bool,
                delayed_quorum: bool,
            ) {
                self.recv_quorum_checks_ex(gen_, with_checks, delayed_quorum, true)
            }

            fn recv_quorum_ex(&mut self, gen_: &GenEnvelope, check_upcoming: bool) {
                self.recv_quorum_checks_ex(gen_, true, false, check_upcoming)
            }

            fn recv_quorum(&mut self, gen_: &GenEnvelope) {
                self.recv_quorum_ex(gen_, false)
            }

            fn nodes_all_pledge_to_commit(&mut self) {
                let qset_hash0 = self.qset_hash0();

                let b = SCPBallot {
                    counter: 1,
                    value: self.x.values.x_value.clone(),
                };

                let prepare1 = make_prepare(
                    &self.x.v1_secret_key,
                    self.x.qset_hash.into_inner(),
                    0,
                    b.clone(),
                    None,
                    0,
                    0,
                    None,
                )
                .unwrap();
                let prepare2 = make_prepare(
                    &self.x.v2_secret_key,
                    self.x.qset_hash.into_inner(),
                    0,
                    b.clone(),
                    None,
                    0,
                    0,
                    None,
                )
                .unwrap();
                let prepare3 = make_prepare(
                    &self.x.v3_secret_key,
                    self.x.qset_hash.into_inner(),
                    0,
                    b.clone(),
                    None,
                    0,
                    0,
                    None,
                )
                .unwrap();
                let prepare4 = make_prepare(
                    &self.x.v4_secret_key,
                    self.x.qset_hash.into_inner(),
                    0,
                    b.clone(),
                    None,
                    0,
                    0,
                    None,
                )
                .unwrap();

                assert!(self.x.tk.bump_state(0, &self.x.values.x_value));
                assert_eq!(self.x.tk.driver().envelopes.len(), 1);

                verify_prepare(
                    &self.x.tk.driver().envelopes[0],
                    &self.x.v0_secret_key,
                    qset_hash0,
                    0,
                    b.clone(),
                    None,
                    0,
                    0,
                    None,
                );

                self.x.tk.proto().received_envelope(&prepare1);
                assert_eq!(self.x.tk.driver().envelopes.len(), 1);
                assert_eq!(
                    self.x.tk.driver().heard_from_quorums.get(&0).unwrap().len(),
                    0
                );

                self.x.tk.proto().received_envelope(&prepare2);
                assert_eq!(self.x.tk.driver().envelopes.len(), 1);
                assert_eq!(
                    self.x.tk.driver().heard_from_quorums.get(&0).unwrap().len(),
                    0
                );

                self.x.tk.proto().received_envelope(&prepare3);
                assert_eq!(self.x.tk.driver().envelopes.len(), 2);
                assert_eq!(
                    self.x.tk.driver().heard_from_quorums.get(&0).unwrap().len(),
                    1
                );
                assert_eq!(self.x.tk.driver().heard_from_quorums.get(&0).unwrap()[0], b);

                // We have a quorum including us
                verify_prepare(
                    &self.x.tk.driver().envelopes[1],
                    &self.x.v0_secret_key,
                    qset_hash0,
                    0,
                    b.clone(),
                    Some(b.clone()),
                    0,
                    0,
                    None,
                );

                self.x.tk.proto().received_envelope(&prepare4);
                assert_eq!(self.x.tk.driver().envelopes.len(), 2);

                let prepared1 = make_prepare(
                    &self.x.v1_secret_key,
                    self.x.qset_hash.into_inner(),
                    0,
                    b.clone(),
                    Some(b.clone()),
                    0,
                    0,
                    None,
                )
                .unwrap();
                let prepared2 = make_prepare(
                    &self.x.v2_secret_key,
                    self.x.qset_hash.into_inner(),
                    0,
                    b.clone(),
                    Some(b.clone()),
                    0,
                    0,
                    None,
                )
                .unwrap();
                let prepared3 = make_prepare(
                    &self.x.v3_secret_key,
                    self.x.qset_hash.into_inner(),
                    0,
                    b.clone(),
                    Some(b.clone()),
                    0,
                    0,
                    None,
                )
                .unwrap();
                let prepared4 = make_prepare(
                    &self.x.v4_secret_key,
                    self.x.qset_hash.into_inner(),
                    0,
                    b.clone(),
                    Some(b.clone()),
                    0,
                    0,
                    None,
                )
                .unwrap();

                self.x.tk.proto().received_envelope(&prepared4);
                self.x.tk.proto().received_envelope(&prepared3);
                assert_eq!(self.x.tk.driver().envelopes.len(), 2);

                self.x.tk.proto().received_envelope(&prepared2);
                assert_eq!(self.x.tk.driver().envelopes.len(), 3);

                // confirms prepared
                verify_prepare(
                    &self.x.tk.driver().envelopes[2],
                    &self.x.v0_secret_key,
                    qset_hash0,
                    0,
                    b.clone(),
                    Some(b.clone()),
                    b.counter,
                    b.counter,
                    None,
                );

                // extra statement doesn't do anything
                self.x.tk.proto().received_envelope(&prepared1);
                assert_eq!(self.x.tk.driver().envelopes.len(), 3);
            }

            fn section_bump_state_x(&mut self) {
                println!("SECTION: bumpState x");
                assert!(self.x.tk.bump_state(0, &self.x.values.x_value.clone()));
                assert_eq!(self.x.tk.driver().envelopes.len(), 1);

                let expected_ballot = SCPBallot {
                    counter: 1,
                    value: self.x.values.x_value.clone(),
                };

                let qset_hash0 = self.qset_hash0();
                verify_prepare(
                    &self.x.tk.driver().envelopes[0],
                    &self.x.v0_secret_key,
                    qset_hash0,
                    0,
                    expected_ballot,
                    None,
                    0,
                    0,
                    None,
                );
            }

            fn section_start_1x(&mut self) {
                println!("SECTION: start <1,x>");
                // No timer should be set initially
                assert!(!self.x.tk.driver().has_ballot_timer());

                let a_value = &self.x.values.x_value;
                let b_value = &self.x.values.z_value;
                let mid_value = &self.x.values.y_value;
                let big_value = &self.x.values.zz_value;

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

                assert!(self.x.tk.bump_state(0, a_value));
                assert_eq!(self.x.tk.driver().envelopes.len(), 1);
                assert!(!self.x.tk.driver().has_ballot_timer());

                println!("SECTION: prepared A1");
                {}

                println!("SECTION: prepared B (v-blocking)");
                {}

                println!("SECTION: prepare B (quorum)");
                {}

                println!("SECTION: confirm (v-blocking)");
                {}
            }
        }

        let test = |x: Fix0| {
            // Fix1::new(x).section_bump_state_x();
            // Fix1::new(x).section_start_1x();
        };

        test_timeouts(test);
    }
}
