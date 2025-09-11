use crate::service::*;
use async_trait::async_trait;
use core::net::SocketAddr;
use proto::*;
use rpc_driver::{RpcServer, connect_client, server_state::ServiceMap};
use tokio;
use tokio::net::TcpListener;
use xdr_common::VecM;

struct TestHandler {}

#[async_trait]
impl proto::SCP_Handle_SCP_Handle1 for TestHandler {
    async fn scp_handle_null(&self) -> Result<(), ::rpc_driver::ProgramError> {
        todo!()
    }

    async fn scp_handle_emit_envelope(
        &self,
        arg0: SCPEnvelope,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        todo!()
    }

    async fn scp_handle_validate_ballot_value(
        &self,
        arg0: SCPSlotIndex,
        arg1: SCPValue,
    ) -> Result<SCPValidationLevel, ::rpc_driver::ProgramError> {
        todo!()
    }

    async fn scp_handle_validate_nomination_value(
        &self,
        arg0: SCPSlotIndex,
        arg1: SCPValue,
    ) -> Result<SCPValidationLevel, ::rpc_driver::ProgramError> {
        todo!()
    }

    async fn scp_handle_extract_valid_value(
        &self,
        arg0: SCPSlotIndex,
        arg1: SCPValue,
    ) -> Result<SCPOptValue, ::rpc_driver::ProgramError> {
        todo!()
    }

    async fn scp_handle_combine_candidates(
        &self,
        arg0: SCPSlotIndex,
        arg1: SCPValues,
    ) -> Result<SCPOptValue, ::rpc_driver::ProgramError> {
        todo!()
    }

    async fn scp_handle_value_externalized(
        &self,
        arg0: SCPSlotIndex,
        arg1: SCPValue,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        todo!()
    }

    async fn scp_handle_nominating_value(
        &self,
        arg0: SCPSlotIndex,
        arg1: SCPValue,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        todo!()
    }

    async fn scp_handle_updated_candidate_value(
        &self,
        arg0: SCPSlotIndex,
        arg1: SCPValue,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        todo!()
    }

    async fn scp_handle_started_ballot_proto(
        &self,
        arg0: SCPSlotIndex,
        arg1: SCPBallot,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        todo!()
    }

    async fn scp_handle_accepted_ballot_prepared(
        &self,
        arg0: SCPSlotIndex,
        arg1: SCPBallot,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        todo!()
    }

    async fn scp_handle_confirmed_ballot_prepared(
        &self,
        arg0: SCPSlotIndex,
        arg1: SCPBallot,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        todo!()
    }

    async fn scp_handle_accepted_commit(
        &self,
        arg0: SCPSlotIndex,
        arg1: SCPBallot,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        todo!()
    }

    async fn scp_handle_ballot_did_hear_from_quorum(
        &self,
        arg0: SCPSlotIndex,
        arg1: SCPBallot,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        todo!()
    }
}

#[tokio::test]
async fn three() {
    // clients
    let handler0_sockaddr: SocketAddr = "localhost:7770".parse().unwrap();
    let client0 = proto::SCP_Handle_SCP_Handle1_Client::new(
        connect_client(&handler0_sockaddr).await.unwrap(),
    );

    let handler1_sockaddr: SocketAddr = "localhost:7771".parse().unwrap();
    let client1 = proto::SCP_Handle_SCP_Handle1_Client::new(
        connect_client(&handler1_sockaddr).await.unwrap(),
    );

    let handler2_sockaddr: SocketAddr = "localhost:7772".parse().unwrap();
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

    // control servers
    // start_control_server(":6660", s0).await;
    // start_control_server(":6661", s1).await;
    // start_control_server(":6662", s2).await;
}

async fn start_control_server(addr: &str, svc: Service) -> SocketAddr {
    let ctl_listener = TcpListener::bind(addr).await.unwrap();
    let ctl_sockaddr = ctl_listener.local_addr().unwrap();

    let mut ctl_service_map = ServiceMap::new();
    ctl_service_map.add_service(Box::new(
        groundhog_scp::SCP_Control_SCP_Control1_Dispatcher::new(Box::new(svc)),
    ));

    RpcServer::new(ctl_service_map, ctl_listener).start().await;

    ctl_sockaddr
}
