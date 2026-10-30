#[cfg(test)]
mod unit_tests {
    use crate::testutil::*;
    use crate::*;
    use shfix::shfix;
    use std::collections::{HashMap, HashSet};
    use thruput_scp::*;
    use xdr_common::VecM;

    fn is_near(r: u64, target: f64) -> bool {
        let v = r as f64 / u64::MAX as f64;
        (v - target).abs() < 0.01
    }

    #[test]
    fn nomination_span_labels_entry_round() {
        use tracing_subscriber::prelude::*;

        // Isolate tracing callsite interest from concurrent kernel tests.
        if std::env::var_os("SCP_NOMINATION_SPAN_TEST").is_none() {
            assert!(std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "unit_test::unit_tests::nomination_span_labels_entry_round"])
                .env("SCP_NOMINATION_SPAN_TEST", "1")
                .status().unwrap().success());
            return;
        }

        #[derive(Clone)]
        struct Capture(std::sync::Arc<std::sync::Mutex<Vec<(String, i64)>>>);
        impl tracing::field::Visit for Capture {
            fn record_debug(
                &mut self,
                _: &tracing::field::Field,
                _: &dyn std::fmt::Debug,
            ) {
            }
            fn record_i64(
                &mut self,
                field: &tracing::field::Field,
                value: i64,
            ) {
                self.0
                    .lock()
                    .unwrap()
                    .push((field.name().to_string(), value));
            }
        }
        impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Capture {
            fn on_new_span(
                &self,
                attrs: &tracing::span::Attributes<'_>,
                _: &tracing::Id,
                _: tracing_subscriber::layer::Context<'_, S>,
            ) {
                if attrs.metadata().name() == "nominate" {
                    attrs.record(&mut self.clone());
                }
            }
        }
        let fields = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let subscriber =
            tracing_subscriber::registry()
                .with(Capture(fields.clone()).with_filter(
                    tracing_subscriber::filter::LevelFilter::INFO,
                ));
        tracing::subscriber::with_default(subscriber, || {
            let nodes = TestNode::new_batch(2);
            let qset = make_qset(
                &[nodes[0].node_id().clone(), nodes[1].node_id().clone()],
                2,
                2,
                0,
            );
            let mut kernel =
                TestKernel::new(nodes[0].node_id().clone(), qset, true);
            kernel.driver().priority_lookup = Box::new(|_| 1000);
            let value = SCPValue::default();
            kernel.nominate(42, &value, false);
            kernel.nominate(42, &value, true);
        });
        let fields = fields.lock().unwrap();
        assert!(fields.contains(&("slot_index".into(), 42)));
        assert!(fields.contains(&("entry_round_number".into(), 0)));
        assert!(fields.iter().any(|(name, value)| name == "entry_round_number" && *value > 0));
        assert!(!fields.iter().any(|(name, _)| name == "round_number"));
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

        let mut uk = UnitTestKernel::new(
            nodes[0].node_id().clone(),
            q_set.clone(),
            true,
        );

        let result = uk.proto().cd.driver.get_node_weight(
            &nodes[2].node_id(),
            &q_set,
            false,
        );
        assert!(is_near(result, 0.75));

        let result = uk.proto().cd.driver.get_node_weight(
            &nodes[4].node_id(),
            &q_set,
            false,
        );
        assert_eq!(result, 0);

        let inner_qset = SCPQuorumSet {
            threshold: 1,
            validators: VecM(vec![
                nodes[4].node_id().clone(),
                nodes[5].node_id().clone(),
            ]),
            inner_sets: VecM(vec![]),
        };
        q_set.inner_sets.0.push(inner_qset);
        normalize_qset(&mut q_set, None);

        let result = uk.proto().cd.driver.get_node_weight(
            &nodes[4].node_id(),
            &q_set,
            false,
        );
        assert!(is_near(result, 0.6 * 0.5));
    }

    fn make_qset(
        node_ids: &[thruput_scp::SCPNodeID],
        threshold: u32,
        total: usize,
        offset: usize,
    ) -> thruput_scp::SCPQuorumSet {
        let mut validators = Vec::new();
        for i in 0..total {
            validators.push(node_ids[i + offset].clone());
        }

        thruput_scp::SCPQuorumSet {
            threshold,
            validators: VecM(validators),
            inner_sets: VecM(vec![]),
        }
    }

    #[shfix]
    #[test]
    fn nomination_weight_stats() {
        let nodes = TestNode::new_batch(7);

        let node_ids: Vec<thruput_scp::SCPNodeID> =
            nodes.iter().map(|n| n.node_id()).collect();

        let total_slots = 1000;
        let max_round_per_slot = 5; // 5 -> 15 seconds
        let total_rounds = total_slots * max_round_per_slot;

        let run_tests = |qset: &thruput_scp::SCPQuorumSet| -> HashMap<thruput_scp::SCPNodeID, i32> {
            let mut wins = HashMap::<thruput_scp::SCPNodeID, i32>::new();

            let mut uk = UnitTestKernel::new(node_ids[0].clone(), qset.clone(), true);

            for s in 0..total_slots {
                let mut value = thruput_scp::SCPValue::default();
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

        section!(flat_quorum, "flat quorum", {
            let flat_test = |threshold: u32, total: usize| {
                let qset = make_qset(&node_ids, threshold, total, 0);

                let wins = run_tests(&qset);

                for (node_id, win_count) in &wins {
                    let stats =
                        (*win_count as f64 * 100.0) / total_rounds as f64;
                    let is_local = node_id == &node_ids[0];
                    println!(
                        "got {:.2}{}",
                        stats,
                        if is_local { " LOCAL" } else { "" }
                    );
                }
            };

            section!(three_out_of_five, "3 out of 5", {
                flat_test(3, 5);
            });

            section!(two_out_of_three, "2 out of 3", {
                flat_test(2, 3);
            });
        });

        section!(hierarchy, "hierarchy", {
            let mut qset = make_qset(&node_ids, 3, 4, 0);

            let qset_inner = make_qset(&node_ids, 2, 3, 4);
            qset.inner_sets.0.push(qset_inner);

            let wins = run_tests(&qset);

            for (node_id, win_count) in &wins {
                let stats = (*win_count as f64 * 100.0) / total_rounds as f64;
                let is_outer = qset.validators.0.iter().any(|k| k == node_id);
                let is_local = node_id == &node_ids[0];

                let label = if is_local {
                    "LOCAL"
                } else if is_outer {
                    "OUTER"
                } else {
                    "INNER"
                };

                println!("got {:.2} {}", stats, label);
            }
        });
    }

    #[shfix]
    #[test]
    fn nomination_two_nodes_win() {
        let nb_rounds_for_stats = 9;
        let nodes = TestNode::new_batch(7);
        let node_ids: Vec<thruput_scp::SCPNodeID> =
            nodes.iter().map(|n| n.node_id()).collect();

        let total_iter = 10000;

        // maxRounds is the number of rounds to evaluate in a row
        // the iteration is considered successful if validators could
        // agree on what to nominate before maxRounds is reached
        let nomination_leaders = |max_rounds: u32,
                                  qset_node0: &thruput_scp::SCPQuorumSet,
                                  qset_node1: &thruput_scp::SCPQuorumSet|
         -> i32 {
            let mut uk0 = UnitTestKernel::new(
                node_ids[0].clone(),
                qset_node0.clone(),
                true,
            );
            let mut uk1 = UnitTestKernel::new(
                node_ids[1].clone(),
                qset_node1.clone(),
                true,
            );

            let mut tot = 0;
            for g in 0..total_iter {
                let mut value = thruput_scp::SCPValue::default();
                value.0.push(g as u8);

                let mut proto0 = uk0.proto();
                let mut slot0 = proto0.slot_proto_or_create(0);
                let mut nom_proto0 = slot0.nomination_proto();
                nom_proto0.dbg_set_previous_value(&value);

                let mut proto1 = uk1.proto();
                let mut slot1 = proto1.slot_proto_or_create(0);
                let mut nom_proto1 = slot1.nomination_proto();
                nom_proto1.dbg_set_previous_value(&value);

                #[allow(unused_assignments)]
                let mut res = true;
                let mut v0_voted = false;
                let mut v1_voted = false;
                let mut r = 0;

                loop {
                    nom_proto0.dbg_set_round_number(r);
                    nom_proto0.update_round_leaders();

                    nom_proto1.dbg_set_round_number(r);
                    nom_proto1.update_round_leaders();

                    let l0 = nom_proto0.dbg_get_round_leaders();
                    assert!(!l0.is_empty());
                    let l1 = nom_proto1.dbg_get_round_leaders();
                    assert!(!l1.is_empty());

                    let update_voted =
                        |node_id: &thruput_scp::SCPNodeID,
                         leaders: &HashSet<thruput_scp::SCPNodeID>,
                         voted: &mut bool| {
                            if !*voted {
                                *voted = leaders.contains(node_id);
                            }
                        };

                    // checks if id voted (any past round, including this one)
                    // AND id is a leader this round
                    let find_node =
                        |node_id: &thruput_scp::SCPNodeID,
                         id_voted: bool,
                         other_leaders: &HashSet<thruput_scp::SCPNodeID>|
                         -> bool {
                            id_voted && other_leaders.contains(node_id)
                        };

                    update_voted(&node_ids[0], l0, &mut v0_voted);
                    update_voted(&node_ids[1], l1, &mut v1_voted);

                    // either both vote for v0 or both vote for v1
                    res = find_node(&node_ids[0], v0_voted, l1);
                    res = res || find_node(&node_ids[1], v1_voted, l0);

                    r += 1;
                    if res || r >= max_rounds {
                        break;
                    }
                }

                if res {
                    tot += 1;
                }
            }
            tot
        };

        section!(flat_quorum, "flat quorum", {
            // test using the same quorum on all nodes
            let flat_test = |threshold: u32, total: usize| {
                let qset = make_qset(&node_ids, threshold, total, 0);

                for max_rounds in 1..=nb_rounds_for_stats {
                    let tot = nomination_leaders(max_rounds, &qset, &qset);
                    let stats = (tot as f64 * 100.0) / total_iter as f64;
                    println!("win rate for {} : {:.2}", max_rounds, stats);
                }
            };

            section!(three_out_of_five, "3 out of 5", {
                flat_test(3, 5);
            });

            section!(two_out_of_three, "2 out of 3", {
                flat_test(2, 3);
            });
        });

        section!(hierarchy, "hierarchy", {
            section!(same_qset, "same qSet", {
                let mut qset = make_qset(&node_ids, 3, 4, 0);
                let qset_inner = make_qset(&node_ids, 2, 3, 4);
                qset.inner_sets.0.push(qset_inner);

                for max_rounds in 1..=nb_rounds_for_stats {
                    let tot = nomination_leaders(max_rounds, &qset, &qset);
                    let stats = (tot as f64 * 100.0) / total_iter as f64;
                    println!("win rate for {} : {:.2}", max_rounds, stats);
                }
            });

            section!(v0_is_inner_node_for_v1, "v0 is inner node for v1", {
                let mut qset0 = make_qset(&node_ids, 3, 4, 0);
                let qset_inner0 = make_qset(&node_ids, 2, 3, 4);
                qset0.inner_sets.0.push(qset_inner0);

                // v1's qset: we move v0 into the inner set
                let mut qset1 = qset0.clone();
                assert_eq!(qset1.validators.0[0], node_ids[0]);
                qset1.validators.0.swap(0, 0); // This is a no-op, but matches the C++ structure
                // Actually move v0 to inner set
                let v0_id = qset1.validators.0.remove(0);
                qset1.inner_sets.0[0].validators.0.insert(0, v0_id);

                for max_rounds in 1..=nb_rounds_for_stats {
                    let tot = nomination_leaders(max_rounds, &qset0, &qset1);
                    let stats = (tot as f64 * 100.0) / total_iter as f64;
                    println!("win rate for {} : {:.2}", max_rounds, stats);
                }
            });
        });
    }
}
