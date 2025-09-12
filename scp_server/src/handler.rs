use scp_kernel::*;
use std::sync::Arc;
use tokio::sync::RwLock as TokioRwLock;
use tokio::sync::RwLockReadGuard as TokioRwLockReadGuard;
use tokio::sync::RwLockWriteGuard as TokioRwLockWriteGuard;
use tokio::sync::mpsc::{Receiver, Sender};

// rpc <--async-- handler service <--async----sync-- kernel service

struct HandlerService(Arc<TokioRwLock<HandlerBridge>>);

struct HandlerBridge {
    pub rx_request: Receiver<HandleRequest>,
    pub tx_reply: Sender<HandleReply>,
    pub scp_handle: SCP_Handle_SCP_Handle1_Client,
}

impl HandlerService {
    pub fn new(
        rx_request: Receiver<HandleRequest>,
        tx_reply: Sender<HandleReply>,
        scp_handle: SCP_Handle_SCP_Handle1_Client,
    ) -> HandlerService {
        HandlerService(Arc::new(TokioRwLock::new(HandlerBridge {
            rx_request,
            tx_reply,
            scp_handle,
        })))
    }

    pub async fn start(&self) {
        loop {
            let req = self.lock_mut().await.rx_request.recv().await.unwrap();
            match req {
                HandleRequest::EmitEnvelope(env) => {
                    self.lock()
                        .await
                        .scp_handle
                        .scp_handle_emit_envelope(&env)
                        .await
                        .unwrap();
                    self.lock()
                        .await
                        .tx_reply
                        .send(HandleReply::EmitEnvelope())
                        .await
                        .unwrap();
                }
                HandleRequest::ValidateBallotValue(slot_index, value) => {
                    let r = self
                        .lock()
                        .await
                        .scp_handle
                        .scp_handle_validate_ballot_value(&slot_index, &value)
                        .await
                        .unwrap();
                    self.lock()
                        .await
                        .tx_reply
                        .send(HandleReply::ValidateBallotValue(r))
                        .await
                        .unwrap();
                }
                HandleRequest::ValidateNominationValue(slot_index, value) => {
                    let r = self
                        .lock()
                        .await
                        .scp_handle
                        .scp_handle_validate_nomination_value(&slot_index, &value)
                        .await
                        .unwrap();
                    self.lock()
                        .await
                        .tx_reply
                        .send(HandleReply::ValidateNominationValue(r))
                        .await
                        .unwrap();
                }
                HandleRequest::ExtractValidValue(slot_index, value) => {
                    let r = self
                        .lock()
                        .await
                        .scp_handle
                        .scp_handle_extract_valid_value(&slot_index, &value)
                        .await
                        .unwrap();
                    self.lock()
                        .await
                        .tx_reply
                        .send(HandleReply::ExtractValidValue(r))
                        .await
                        .unwrap();
                }
                HandleRequest::CombineCandidates(slot_index, values) => {
                    let r = self
                        .lock()
                        .await
                        .scp_handle
                        .scp_handle_combine_candidates(&slot_index, &values)
                        .await
                        .unwrap();
                    self.lock()
                        .await
                        .tx_reply
                        .send(HandleReply::CombineCandidates(r))
                        .await
                        .unwrap();
                }
                HandleRequest::GetQuorumSet(hash) => {
                    let r = self
                        .lock()
                        .await
                        .scp_handle
                        .scp_handle_get_quorum_set(&hash)
                        .await
                        .unwrap();
                    self.lock()
                        .await
                        .tx_reply
                        .send(HandleReply::GetQuorumSet(r))
                        .await
                        .unwrap();
                }
                HandleRequest::ValueExternalized(slot_index, value) => {
                    self.lock()
                        .await
                        .scp_handle
                        .scp_handle_value_externalized(&slot_index, &value)
                        .await
                        .unwrap();
                    self.lock()
                        .await
                        .tx_reply
                        .send(HandleReply::ValueExternalized())
                        .await
                        .unwrap();
                }
                HandleRequest::StartedBallotProto(slot_index, ballot) => {
                    self.lock()
                        .await
                        .scp_handle
                        .scp_handle_started_ballot_proto(&slot_index, &ballot)
                        .await
                        .unwrap();
                    self.lock()
                        .await
                        .tx_reply
                        .send(HandleReply::StartedBallotProto())
                        .await
                        .unwrap();
                }
                HandleRequest::AcceptedBallotPrepared(slot_index, ballot) => {
                    self.lock()
                        .await
                        .scp_handle
                        .scp_handle_accepted_ballot_prepared(&slot_index, &ballot)
                        .await
                        .unwrap();
                    self.lock()
                        .await
                        .tx_reply
                        .send(HandleReply::AcceptedBallotPrepared())
                        .await
                        .unwrap();
                }
                HandleRequest::ConfirmedBallotPrepared(slot_index, ballot) => {
                    self.lock()
                        .await
                        .scp_handle
                        .scp_handle_confirmed_ballot_prepared(&slot_index, &ballot)
                        .await
                        .unwrap();
                    self.lock()
                        .await
                        .tx_reply
                        .send(HandleReply::ConfirmedBallotPrepared())
                        .await
                        .unwrap();
                }
                HandleRequest::AcceptedCommit(slot_index, ballot) => {
                    self.lock()
                        .await
                        .scp_handle
                        .scp_handle_accepted_commit(&slot_index, &ballot)
                        .await
                        .unwrap();
                    self.lock()
                        .await
                        .tx_reply
                        .send(HandleReply::AcceptedCommit())
                        .await
                        .unwrap();
                }
                HandleRequest::BallotDidHearFromQuorum(slot_index, ballot) => {
                    self.lock()
                        .await
                        .scp_handle
                        .scp_handle_ballot_did_hear_from_quorum(&slot_index, &ballot)
                        .await
                        .unwrap();
                    self.lock()
                        .await
                        .tx_reply
                        .send(HandleReply::BallotDidHearFromQuorum())
                        .await
                        .unwrap();
                }
                HandleRequest::NominatingValue(slot_index, value) => {
                    self.lock()
                        .await
                        .scp_handle
                        .scp_handle_nominating_value(&slot_index, &value)
                        .await
                        .unwrap();
                    self.lock()
                        .await
                        .tx_reply
                        .send(HandleReply::NominatingValue())
                        .await
                        .unwrap();
                }
                HandleRequest::UpdatedCandidateValue(slot_index, value) => {
                    self.lock()
                        .await
                        .scp_handle
                        .scp_handle_updated_candidate_value(&slot_index, &value)
                        .await
                        .unwrap();
                    self.lock()
                        .await
                        .tx_reply
                        .send(HandleReply::UpdatedCandidateValue())
                        .await
                        .unwrap();
                }
            }
        }
    }

    async fn lock(&self) -> TokioRwLockReadGuard<'_, HandlerBridge> {
        self.0.read().await
    }

    async fn lock_mut(&self) -> TokioRwLockWriteGuard<'_, HandlerBridge> {
        self.0.write().await
    }
}

enum HandleRequest {
    EmitEnvelope(SCPEnvelope),
    ValidateBallotValue(SCPSlotIndex, SCPValue),
    ValidateNominationValue(SCPSlotIndex, SCPValue),
    ExtractValidValue(SCPSlotIndex, SCPValue),
    CombineCandidates(SCPSlotIndex, SCPValues),
    GetQuorumSet(SCPHash),
    //
    ValueExternalized(SlotIndex, SCPValue),
    StartedBallotProto(SlotIndex, SCPBallot),
    AcceptedBallotPrepared(SlotIndex, SCPBallot),
    ConfirmedBallotPrepared(SlotIndex, SCPBallot),
    AcceptedCommit(SlotIndex, SCPBallot),
    BallotDidHearFromQuorum(SlotIndex, SCPBallot),
    NominatingValue(SlotIndex, SCPValue),
    UpdatedCandidateValue(SlotIndex, SCPValue),
}

enum HandleReply {
    EmitEnvelope(),
    ValidateBallotValue(SCPValidationLevel),
    ValidateNominationValue(SCPValidationLevel),
    ExtractValidValue(SCPOptValue),
    CombineCandidates(SCPOptValue),
    GetQuorumSet(SCPOptQuorumSet),
    //
    ValueExternalized(),
    StartedBallotProto(),
    AcceptedBallotPrepared(),
    ConfirmedBallotPrepared(),
    AcceptedCommit(),
    BallotDidHearFromQuorum(),
    NominatingValue(),
    UpdatedCandidateValue(),
}
