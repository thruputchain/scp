use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use thruput_rpc_driver::connect_client;
use thruput_scp::*;
use tokio;
use tokio::sync::Mutex;
use tracing_subscriber;
use xdr_common::{BytesM, VecM};

fn instrument_test() {
    let _ = tracing_subscriber::fmt()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
        // .with_max_level(tracing::metadata::LevelFilter::DEBUG)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    tracing_subscriber::EnvFilter::new("debug")
                }),
        )
        .event_format(tracing_subscriber::fmt::format())
        .with_level(true)
        .with_test_writer() // important: routes to test harness writer
        .with_file(true)
        .with_line_number(true)
        .with_level(true)
        .with_target(false)
        .init();
}

struct TestHandlerState {
    quorum_sets: HashMap<SCPHash, SCPQuorumSet>,
    externalized: HashMap<SCPSlotIndex, SCPValue>,
    n_emitted: i64,
    emitted: Vec<(i64, SCPEnvelope)>,
    control: Option<ControlService>,
}

#[derive(Clone)]
struct TestHandler(Arc<Mutex<TestHandlerState>>);

impl TestHandler {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(TestHandlerState {
            quorum_sets: HashMap::new(),
            externalized: HashMap::new(),
            n_emitted: 0,
            emitted: vec![],
            control: None,
        })))
    }

    async fn set_control_service(&self, s: ControlService) {
        self.0.lock().await.control = Some(s);
    }

    async fn add_qset(&self, q: SCPQuorumSet) {
        let mut s = self.0.lock().await;
        s.quorum_sets
            .insert(scp_util::hash_xdr_scp(&q).unwrap(), q.clone());
        s.control
            .as_ref()
            .unwrap()
            .control_set_qset(q)
            .await
            .unwrap();
    }

    async fn num_externalized(&self) -> usize {
        self.0.lock().await.externalized.len()
    }
}

// The handler trait returns `impl Future`; these async fns name their
// concrete reply types.
#[allow(refining_impl_trait)]
impl thruput_scp::SCP_Handle_SCP_Handle1 for TestHandler {
    async fn scp_handle_null(
        &self,
    ) -> Result<(), ::thruput_rpc_driver::ProgramError> {
        Ok(())
    }

    async fn scp_handle_get_quorum_set(
        &self,
        h: SCPHash,
    ) -> Result<Option<SCPQuorumSet>, ::thruput_rpc_driver::ProgramError> {
        Ok(self.0.lock().await.quorum_sets.get(&h).cloned())
    }

    async fn scp_handle_emit_envelope(
        &self,
        env: SCPEnvelope,
    ) -> Result<(), ::thruput_rpc_driver::ProgramError> {
        println!(
            "scp_handle_emit_envelope, slot {}",
            env.statement.slot_index,
        );
        let mut lk = self.0.lock().await;
        let seq_no = lk.n_emitted;
        lk.emitted.push((seq_no, env));
        lk.n_emitted += 1;
        Ok(())
    }

    async fn scp_handle_validate_ballot_value(
        &self,
        _slot_index: SCPSlotIndex,
        _value: SCPValue,
    ) -> Result<SCPValidationLevel, ::thruput_rpc_driver::ProgramError> {
        Ok(SCPValidationLevel::SCP_FULLY_VALID)
    }

    async fn scp_handle_validate_nomination_value(
        &self,
        _slot_index: SCPSlotIndex,
        _value: SCPValue,
    ) -> Result<SCPValidationLevel, ::thruput_rpc_driver::ProgramError> {
        Ok(SCPValidationLevel::SCP_FULLY_VALID)
    }

    async fn scp_handle_extract_valid_value(
        &self,
        _slot_index: SCPSlotIndex,
        value: SCPValue,
    ) -> Result<SCPOptValue, ::thruput_rpc_driver::ProgramError> {
        Ok(Some(value))
    }

    async fn scp_handle_combine_candidates(
        &self,
        _slot_index: SCPSlotIndex,
        mut values: SCPValues,
    ) -> Result<SCPOptValue, ::thruput_rpc_driver::ProgramError> {
        if values.len() > 0 {
            values.0.sort();
            Ok(Some(values.0[0].clone()))
        } else {
            Ok(None)
        }
    }

    async fn scp_handle_value_externalized(
        &self,
        slot_index: SCPSlotIndex,
        value: SCPValue,
    ) -> Result<(), ::thruput_rpc_driver::ProgramError> {
        println!(
            "scp_handle_value_externalized, slot {}, value {}",
            slot_index, value
        );
        self.0
            .lock()
            .await
            .externalized
            .insert(slot_index, value.clone());
        // start next nomination
        if let Some(s) = &self.0.lock().await.control {
            let s2 = s.clone();
            let v2 = value.clone();
            tokio::spawn(async move {
                // s2.scp_control_stop_nomination(slot_index).await.unwrap();
                // println!("  stopped nomination for {}", slot_index);
                s2.control_nominate(slot_index + 1, BytesM(vec![0]), v2)
                    .await
                    .unwrap();
                println!("  started nomination for {}", slot_index + 1);
            });
        };
        Ok(())
    }

    async fn scp_handle_nominating_value(
        &self,
        slot_index: SCPSlotIndex,
        value: SCPValue,
    ) -> Result<(), ::thruput_rpc_driver::ProgramError> {
        println!(
            "scp_handle_nominating_value, slot {}, value {}",
            slot_index, value
        );
        Ok(())
    }

    async fn scp_handle_updated_candidate_value(
        &self,
        _slot_index: SCPSlotIndex,
        _value: SCPValue,
    ) -> Result<(), ::thruput_rpc_driver::ProgramError> {
        Ok(())
    }

    async fn scp_handle_started_ballot_proto(
        &self,
        _slot_index: SCPSlotIndex,
        _ballot: SCPBallot,
    ) -> Result<(), ::thruput_rpc_driver::ProgramError> {
        Ok(())
    }

    async fn scp_handle_accepted_ballot_prepared(
        &self,
        _slot_index: SCPSlotIndex,
        _ballot: SCPBallot,
    ) -> Result<(), ::thruput_rpc_driver::ProgramError> {
        Ok(())
    }

    async fn scp_handle_confirmed_ballot_prepared(
        &self,
        _slot_index: SCPSlotIndex,
        _ballot: SCPBallot,
    ) -> Result<(), ::thruput_rpc_driver::ProgramError> {
        Ok(())
    }

    async fn scp_handle_accepted_commit(
        &self,
        _slot_index: SCPSlotIndex,
        _ballot: SCPBallot,
    ) -> Result<(), ::thruput_rpc_driver::ProgramError> {
        Ok(())
    }

    async fn scp_handle_ballot_did_hear_from_quorum(
        &self,
        _slot_index: SCPSlotIndex,
        _ballot: SCPBallot,
    ) -> Result<(), ::thruput_rpc_driver::ProgramError> {
        Ok(())
    }
}

use crate::controller::*;
use crate::system::*;

#[tokio::test]
async fn five() {
    instrument_test();
    // console_subscriber::init();
    // tracing_subscriber::fmt()
    //     .with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
    //     .fmt_fields(tracing_subscriber::fmt::format::DefaultFields::new())
    //     .with_target(false)
    //     .with_file(true)
    //     .with_line_number(true)
    //     .with_level(false)
    //     .with_ansi(false)
    //     .compact()
    //     .init();

    // handlers

    let h0 = TestHandler::new();
    let h1 = TestHandler::new();
    let h2 = TestHandler::new();

    let handler0_sockaddr =
        start_handle_server("0.0.0.0:7770", h0.clone()).await;
    let handler1_sockaddr =
        start_handle_server("0.0.0.0:7771", h1.clone()).await;
    let handler2_sockaddr =
        start_handle_server("0.0.0.0:7772", h2.clone()).await;

    // clients

    // let handler0_sockaddr: SocketAddr = "localhost:7770".parse().unwrap();
    let client0 = thruput_scp::SCP_Handle_SCP_Handle1_Client::new(
        connect_client(&handler0_sockaddr)
            .await
            .unwrap()
            .into_handle(),
    );

    // let handler1_sockaddr: SocketAddr = "localhost:7771".parse().unwrap();
    let client1 = thruput_scp::SCP_Handle_SCP_Handle1_Client::new(
        connect_client(&handler1_sockaddr)
            .await
            .unwrap()
            .into_handle(),
    );

    // let handler2_sockaddr: SocketAddr = "localhost:7772".parse().unwrap();
    let client2 = thruput_scp::SCP_Handle_SCP_Handle1_Client::new(
        connect_client(&handler2_sockaddr)
            .await
            .unwrap()
            .into_handle(),
    );

    // secret keys
    let sk0 = scp_util::generate_secret_key(&[0]);
    let sk1 = scp_util::generate_secret_key(&[1]);
    let sk2 = scp_util::generate_secret_key(&[2]);

    // configs
    let vs = VecM(vec![
        scp_util::derive_public_key(&sk0),
        scp_util::derive_public_key(&sk1),
        scp_util::derive_public_key(&sk2),
    ]);
    let qset = SCPQuorumSet {
        threshold: 2,
        validators: vs.clone(),
        inner_sets: Default::default(),
    };
    let cfg = SCPConfig {
        max_timeout_ms: 1000,
        initial_ballot_timeout_ms: 750,
        increment_ballot_timeout_ms: 750,
        initial_nomination_timeout_ms: 750,
        increment_nomination_timeout_ms: 750,
        quorum_set: qset.clone(),
    };

    // services
    let s0 = start_service(client0, cfg.clone(), &sk0, true).await;
    let s1 = start_service(client1, cfg.clone(), &sk1, true).await;
    let s2 = start_service(client2, cfg.clone(), &sk2, true).await;

    h0.set_control_service(s0.clone()).await;
    h1.set_control_service(s1.clone()).await;
    h2.set_control_service(s2.clone()).await;

    h0.add_qset(qset.clone()).await;
    h1.add_qset(qset.clone()).await;
    h2.add_qset(qset.clone()).await;

    // experiment
    let nom0 = s0
        .control_nominate(1, BytesM(vec![0]), BytesM(vec![9]))
        .await
        .unwrap();
    let nom1 = s1
        .control_nominate(1, BytesM(vec![1]), BytesM(vec![9]))
        .await
        .unwrap();
    let nom2 = s2
        .control_nominate(1, BytesM(vec![2]), BytesM(vec![9]))
        .await
        .unwrap();
    println!("noms: {} {} {}", nom0, nom1, nom2);

    let nodes = vec![(s0, h0.clone()), (s1, h1.clone()), (s2, h2.clone())];

    flush_emits(&nodes, async || {
        h0.num_externalized().await >= 3
            && h1.num_externalized().await >= 3
            && h2.num_externalized().await >= 3
    })
    .await;

    // future::pending::<()>().await;
}

async fn flush_emits<F, Fut>(
    nodes: &Vec<(ControlService, TestHandler)>,
    exit: F,
) where
    F: Fn() -> Fut,
    Fut: Future<Output = bool>,
{
    loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let mut delivered = false;
        println!("FLUSHING");
        for (i, (_, h)) in nodes.iter().enumerate() {
            for (_, e) in h.0.lock().await.emitted.drain(..) {
                for (j, (s, _)) in nodes.iter().enumerate() {
                    if i != j {
                        s.control_received_envelope(e.clone()).await.unwrap();
                        delivered = true;
                    }
                }
            }
        }
        if !delivered {
            assert!(false);
            break;
        }
        if exit().await {
            break;
        }
    }
}
