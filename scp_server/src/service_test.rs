use crate::service::*;
use async_trait::async_trait;
use core::net::SocketAddr;
use proto::*;
use rpc_driver::{RpcServer, connect_client, server_state::ServiceMap};
use std::sync::Arc;
use tokio;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing_subscriber;
use xdr_common::{BytesM, VecM};

struct TestHandler {
    externalized: Vec<(SCPSlotIndex, SCPValue)>,
    emitted: Vec<SCPEnvelope>,
}

#[derive(Clone)]
struct TestHandlerAsync(Arc<Mutex<TestHandler>>);

impl TestHandlerAsync {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(TestHandler {
            externalized: vec![],
            emitted: vec![],
        })))
    }
}

#[async_trait]
impl proto::SCP_Handle_SCP_Handle1 for TestHandlerAsync {
    async fn scp_handle_null(&self) -> Result<(), ::rpc_driver::ProgramError> {
        Ok(())
    }

    async fn scp_handle_emit_envelope(
        &self,
        env: SCPEnvelope,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        self.0.lock().await.emitted.push(env);
        Ok(())
    }

    async fn scp_handle_validate_ballot_value(
        &self,
        _slot_index: SCPSlotIndex,
        _value: SCPValue,
    ) -> Result<SCPValidationLevel, ::rpc_driver::ProgramError> {
        println!("validate_ballot_value");
        Ok(SCPValidationLevel::SCP_FULLY_VALID)
    }

    async fn scp_handle_validate_nomination_value(
        &self,
        _slot_index: SCPSlotIndex,
        _value: SCPValue,
    ) -> Result<SCPValidationLevel, ::rpc_driver::ProgramError> {
        println!("validate_nomination_value");
        Ok(SCPValidationLevel::SCP_FULLY_VALID)
    }

    async fn scp_handle_extract_valid_value(
        &self,
        _slot_index: SCPSlotIndex,
        value: SCPValue,
    ) -> Result<SCPOptValue, ::rpc_driver::ProgramError> {
        Ok(Some(value))
    }

    async fn scp_handle_combine_candidates(
        &self,
        _slot_index: SCPSlotIndex,
        arg1: SCPValues,
    ) -> Result<SCPOptValue, ::rpc_driver::ProgramError> {
        Ok(Some(arg1.0[0].clone()))
    }

    async fn scp_handle_value_externalized(
        &self,
        slot_index: SCPSlotIndex,
        value: SCPValue,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        println!("value_externalized {}", value);
        self.0.lock().await.externalized.push((slot_index, value));
        Ok(())
    }

    async fn scp_handle_nominating_value(
        &self,
        _slot_index: SCPSlotIndex,
        _value: SCPValue,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        Ok(())
    }

    async fn scp_handle_updated_candidate_value(
        &self,
        _slot_index: SCPSlotIndex,
        _value: SCPValue,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        Ok(())
    }

    async fn scp_handle_started_ballot_proto(
        &self,
        _slot_index: SCPSlotIndex,
        _ballot: SCPBallot,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        Ok(())
    }

    async fn scp_handle_accepted_ballot_prepared(
        &self,
        _slot_index: SCPSlotIndex,
        _ballot: SCPBallot,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        Ok(())
    }

    async fn scp_handle_confirmed_ballot_prepared(
        &self,
        _slot_index: SCPSlotIndex,
        _ballot: SCPBallot,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        Ok(())
    }

    async fn scp_handle_accepted_commit(
        &self,
        _slot_index: SCPSlotIndex,
        _ballot: SCPBallot,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        Ok(())
    }

    async fn scp_handle_ballot_did_hear_from_quorum(
        &self,
        _slot_index: SCPSlotIndex,
        _ballot: SCPBallot,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        Ok(())
    }
}

#[tokio::test]
async fn three() {
    // let _ = tracing_subscriber::fmt()
    //     .with_max_level(tracing::metadata::LevelFilter::DEBUG)
    //     .with_test_writer() // important: routes to test harness writer
    //     .try_init();

    // handlers
    let h0 = TestHandlerAsync::new();
    let h1 = TestHandlerAsync::new();
    let h2 = TestHandlerAsync::new();

    let handler0_sockaddr = start_handle_server("0.0.0.0:7770", h0.clone()).await;
    let handler1_sockaddr = start_handle_server("0.0.0.0:7771", h1.clone()).await;
    let handler2_sockaddr = start_handle_server("0.0.0.0:7772", h2.clone()).await;

    // clients
    // let handler0_sockaddr: SocketAddr = "localhost:7770".parse().unwrap();
    let client0 = proto::SCP_Handle_SCP_Handle1_Client::new(
        connect_client(&handler0_sockaddr).await.unwrap(),
    );

    // let handler1_sockaddr: SocketAddr = "localhost:7771".parse().unwrap();
    let client1 = proto::SCP_Handle_SCP_Handle1_Client::new(
        connect_client(&handler1_sockaddr).await.unwrap(),
    );

    // let handler2_sockaddr: SocketAddr = "localhost:7772".parse().unwrap();
    let client2 = proto::SCP_Handle_SCP_Handle1_Client::new(
        connect_client(&handler2_sockaddr).await.unwrap(),
    );

    // secret keys
    let sk0 = proto::generate_secret_key(&[0]);
    let sk1 = proto::generate_secret_key(&[1]);
    let sk2 = proto::generate_secret_key(&[2]);

    // configs
    let vs = VecM(vec![
        proto::derive_public_key(&sk0),
        proto::derive_public_key(&sk1),
        proto::derive_public_key(&sk2),
    ]);
    let cfg = proto::SCPConfig {
        max_timeout_ms: (30 * 60) * 1000,
        initial_ballot_timeout_ms: 750,
        increment_ballot_timeout_ms: 750,
        initial_nomination_timeout_ms: 750,
        increment_nomination_timeout_ms: 750,
        quorum_set: proto::SCPQuorumSet {
            threshold: 2,
            validators: vs.clone(),
            inner_sets: Default::default(),
        },
    };

    // services
    let s0 = Service::new(client0, cfg.clone(), &sk0, true);
    let s1 = Service::new(client1, cfg.clone(), &sk1, true);
    let s2 = Service::new(client2, cfg.clone(), &sk2, true);

    let nodes = vec![(s0.clone(), h0), (s1.clone(), h1), (s2.clone(), h2)];

    // experiment
    let nom0 = s0
        .scp_control_nominate(0, BytesM(vec![0]), BytesM(vec![]))
        .await
        .unwrap();
    let nom1 = s1
        .scp_control_nominate(0, BytesM(vec![1]), BytesM(vec![]))
        .await
        .unwrap();
    let nom2 = s2
        .scp_control_nominate(0, BytesM(vec![2]), BytesM(vec![]))
        .await
        .unwrap();
    println!("noms: {} {} {}", nom0, nom1, nom2);
    flush_emits(&nodes).await;

    for _ in 0..4 {
        println!("sleeping");
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        flush_emits(&nodes).await;
    }

    //XXX
}

async fn flush_emits(nodes: &Vec<(Service, TestHandlerAsync)>) {
    loop {
        let mut delivered = false;
        println!("flush loop");
        for (i, (_, h)) in nodes.iter().enumerate() {
            println!("draining node {}", i);
            for e in h.0.lock().await.emitted.drain(..) {
                println!("draining envelope");
                // println!("draining envelope {:?}", e);
                for (j, (s, _)) in nodes.iter().enumerate() {
                    if i != j {
                        s.scp_control_received_envelope(e.clone()).await.unwrap();
                        delivered = true;
                    }
                }
            }
        }
        if !delivered {
            break;
        }
    }
}

async fn start_handle_server<S: SCP_Handle_SCP_Handle1 + 'static>(
    addr: &str,
    svc: S,
) -> SocketAddr {
    let listener = TcpListener::bind(addr).await.unwrap();
    let sockaddr = listener.local_addr().unwrap();

    let mut service_map = ServiceMap::new();
    service_map.add_service(Box::new(
        groundhog_scp::SCP_Handle_SCP_Handle1_Dispatcher::new(Box::new(svc)),
    ));

    RpcServer::new(service_map, listener).start().await;

    sockaddr
}

#[allow(dead_code)]
async fn start_control_server<S: SCP_Control_SCP_Control1 + 'static>(
    addr: &str,
    svc: S,
) -> SocketAddr {
    let listener = TcpListener::bind(addr).await.unwrap();
    let sockaddr = listener.local_addr().unwrap();

    let mut service_map = ServiceMap::new();
    service_map.add_service(Box::new(
        groundhog_scp::SCP_Control_SCP_Control1_Dispatcher::new(Box::new(svc)),
    ));

    RpcServer::new(service_map, listener).start().await;

    sockaddr
}
