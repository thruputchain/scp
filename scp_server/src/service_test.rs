use crate::service::*;
use async_trait::async_trait;
use core::net::SocketAddr;
use proto::*;
use rpc_driver::{RpcServer, connect_client, server_state::ServiceMap};
use std::sync::Arc;
use tokio;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
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
        arg0: SCPSlotIndex,
        arg1: SCPValue,
    ) -> Result<SCPValidationLevel, ::rpc_driver::ProgramError> {
        println!("validate_ballot_value");
        Ok(SCPValidationLevel::SCP_FULLY_VALID)
    }

    async fn scp_handle_validate_nomination_value(
        &self,
        arg0: SCPSlotIndex,
        arg1: SCPValue,
    ) -> Result<SCPValidationLevel, ::rpc_driver::ProgramError> {
        println!("validate_nomination_value");
        Ok(SCPValidationLevel::SCP_FULLY_VALID)
    }

    async fn scp_handle_extract_valid_value(
        &self,
        arg0: SCPSlotIndex,
        arg1: SCPValue,
    ) -> Result<SCPOptValue, ::rpc_driver::ProgramError> {
        Ok(Some(arg1))
    }

    async fn scp_handle_combine_candidates(
        &self,
        arg0: SCPSlotIndex,
        arg1: SCPValues,
    ) -> Result<SCPOptValue, ::rpc_driver::ProgramError> {
        Ok(Some(arg1.0[0].clone()))
    }

    async fn scp_handle_value_externalized(
        &self,
        slot_index: SCPSlotIndex,
        value: SCPValue,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        println!("value_externalized");
        self.0.lock().await.externalized.push((slot_index, value));
        Ok(())
    }

    async fn scp_handle_nominating_value(
        &self,
        arg0: SCPSlotIndex,
        arg1: SCPValue,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        Ok(())
    }

    async fn scp_handle_updated_candidate_value(
        &self,
        arg0: SCPSlotIndex,
        arg1: SCPValue,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        Ok(())
    }

    async fn scp_handle_started_ballot_proto(
        &self,
        arg0: SCPSlotIndex,
        arg1: SCPBallot,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        Ok(())
    }

    async fn scp_handle_accepted_ballot_prepared(
        &self,
        arg0: SCPSlotIndex,
        arg1: SCPBallot,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        Ok(())
    }

    async fn scp_handle_confirmed_ballot_prepared(
        &self,
        arg0: SCPSlotIndex,
        arg1: SCPBallot,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        Ok(())
    }

    async fn scp_handle_accepted_commit(
        &self,
        arg0: SCPSlotIndex,
        arg1: SCPBallot,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        Ok(())
    }

    async fn scp_handle_ballot_did_hear_from_quorum(
        &self,
        arg0: SCPSlotIndex,
        arg1: SCPBallot,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        Ok(())
    }
}

#[tokio::test]
async fn three() {
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
    s0.scp_control_nominate(0, BytesM(vec![0]), BytesM(vec![]))
        .await
        .unwrap();
    s1.scp_control_nominate(0, BytesM(vec![1]), BytesM(vec![]))
        .await
        .unwrap();
    s2.scp_control_nominate(0, BytesM(vec![2]), BytesM(vec![]))
        .await
        .unwrap();
    flush_emits(&nodes).await;

    //XXX
}

async fn flush_emits(nodes: &Vec<(Service, TestHandlerAsync)>) {
    for (i, (_, h)) in nodes.iter().enumerate() {
        for e in h.0.lock().await.emitted.drain(..) {
            for (j, (s, _)) in nodes.iter().enumerate() {
                if i != j {
                    s.scp_control_received_envelope(e.clone()).await.unwrap();
                }
            }
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
