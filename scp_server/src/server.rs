use scp_kernel::*;
use std::cell::{RefCell, RefMut};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Duration;
use xdr_common::*;

#[derive(Debug)]
pub struct Server {
    max_timeout: Duration,
    initial_ballot_timeout: Duration,
    increment_ballot_timeout: Duration,
    initial_nomination_timeout: Duration,
    increment_nomination_timeout: Duration,
    //
    secret_key: proto::SecretKey,
    quorum_sets: HashMap<proto::SCPHash, proto::SCPQuorumSet>,
}

impl Server {
    pub fn new(cfg: proto::SCPConfig, secret_key: proto::SecretKey) -> Server {
        Server {
            max_timeout: Duration::from_millis(cfg.max_timeout_ms),
            initial_ballot_timeout: Duration::from_millis(cfg.initial_ballot_timeout_ms),
            increment_ballot_timeout: Duration::from_millis(cfg.increment_ballot_timeout_ms),
            initial_nomination_timeout: Duration::from_millis(cfg.initial_nomination_timeout_ms),
            increment_nomination_timeout: Duration::from_millis(
                cfg.increment_nomination_timeout_ms,
            ),
            secret_key,
            quorum_sets: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AServer(pub Arc<RwLock<Server>>);

impl AServer {
    fn lock(&self) -> RwLockReadGuard<'_, Server> {
        self.0.read().unwrap()
    }

    fn lock_mut(&mut self) -> RwLockWriteGuard<'_, Server> {
        self.0.write().unwrap()
    }
}

impl Consensus for AServer {
    fn validate_ballot_value(
        &self,
        slot_index: SCPSlotIndex,
        value: &proto::SCPValue,
    ) -> ValidationLevel {
        todo!()
    }

    fn validate_nomination_value(
        &self,
        slot_index: SCPSlotIndex,
        value: &proto::SCPValue,
    ) -> ValidationLevel {
        todo!()
    }

    fn extract_valid_value(
        &self,
        slot_index: SCPSlotIndex,
        value: &proto::SCPValue,
    ) -> Option<proto::SCPValue> {
        todo!()
    }

    fn combine_candidates(
        &self,
        slot_index: SCPSlotIndex,
        candidates: &HashSet<proto::SCPValue>,
    ) -> Option<proto::SCPValue> {
        todo!()
    }
}

impl ConsensusNotify for AServer {
    fn value_externalized(&self, _slot_index: SlotIndex, _value: &proto::SCPValue) {}
    fn started_ballot_proto(&self, _slot_index: SlotIndex, _ballot: &proto::SCPBallot) {}
    fn accepted_ballot_prepared(&self, _slot_index: SlotIndex, _ballot: &proto::SCPBallot) {}
    fn confirmed_ballot_prepared(&self, _slot_index: SlotIndex, _ballot: &proto::SCPBallot) {}
    fn accepted_commit(&self, _slot_index: SlotIndex, _ballot: &proto::SCPBallot) {}
    fn ballot_did_hear_from_quorum(&self, _slot_index: SlotIndex, _ballot: &proto::SCPBallot) {}
    fn nominating_value(&self, _slot_index: SlotIndex, _value: &proto::SCPValue) {}
    fn updated_candidate_value(&self, _slot_index: SlotIndex, _value: &proto::SCPValue) {}
}

impl Driver for AServer {
    fn get_qset(&self, h: &proto::SCPHash) -> Option<proto::SCPQuorumSet> {
        self.lock().quorum_sets.get(h).cloned()
    }

    fn to_short_string(&self, pk: &proto::SCPNodeID) -> String {
        self.to_str_key(pk, false)
    }

    fn to_str_key(&self, pk: &proto::SCPNodeID, full_key: bool) -> String {
        match pk {
            PublicKey::Ed25519 { ed25519 } => {
                if full_key {
                    hex_full(&ed25519.into_inner()[..])
                } else {
                    hex_abbrev(&ed25519.into_inner()[..])
                }
            }
        }
    }

    fn from_str_key(&self, s: &str) -> Result<proto::SCPNodeID, String> {
        parse_node_id_from_hex(s)
    }

    fn get_value_string(&self, value: &proto::SCPValue) -> String {
        format!("{}", hex_abbrev(&value.0))
    }

    fn get_hash_of(&self, opaque: &[u8]) -> proto::SCPHash {
        hash_bytes(opaque)
    }

    fn get_node_weight(
        &self,
        node_id: &proto::SCPNodeID,
        qset: &proto::SCPQuorumSet,
        is_local_node: bool,
    ) -> u64 {
        get_node_weight(node_id, qset, is_local_node)
    }

    fn compute_hash_node(
        &self,
        slot_index: proto::SCPSlotIndex,
        prev: &proto::SCPValue,
        is_priority: bool,
        round_number: proto::SCPCount,
        node_id: &proto::SCPNodeID,
    ) -> u64 {
        hash_helper(
            slot_index,
            prev,
            |v| {
                v.push(BytesM(
                    proto::marshal_xdr(if is_priority { &HASH_P } else { &HASH_N })
                        .unwrap()
                        .to_vec(),
                ));
                v.push(BytesM(proto::marshal_xdr(&round_number).unwrap().to_vec()));
                v.push(BytesM(proto::marshal_xdr(node_id).unwrap().to_vec()));
            },
            |b| self.get_hash_of(b),
        )
    }

    fn compute_value_hash(
        &self,
        slot_index: proto::SCPSlotIndex,
        prev: &proto::SCPValue,
        round_number: proto::SCPCount,
        value: &proto::SCPValue,
    ) -> u64 {
        hash_helper(
            slot_index,
            prev,
            |v| {
                v.push(BytesM(proto::marshal_xdr(&HASH_K).unwrap().to_vec()));
                v.push(BytesM(proto::marshal_xdr(&round_number).unwrap().to_vec()));
                v.push(BytesM(proto::marshal_xdr(value).unwrap().to_vec()));
            },
            |b| self.get_hash_of(b),
        )
    }

    fn sign_envelope(&self, env: &mut proto::SCPEnvelope) {
        env.signature = sign_xdr(&self.lock().secret_key, &env.statement).unwrap()
    }

    fn emit_envelope(&self, env: proto::SCPEnvelope) {
        todo!()
    }

    fn compute_timeout(&self, round_number: proto::SCPCount, is_nomination: bool) -> Duration {
        let (initial_timeout_ms, increment_ms) = if is_nomination {
            (
                self.lock().initial_nomination_timeout.as_millis(),
                self.lock().increment_nomination_timeout.as_millis(),
            )
        } else {
            (
                self.lock().initial_ballot_timeout.as_millis(),
                self.lock().increment_ballot_timeout.as_millis(),
            )
        };

        let timeout_ms =
            initial_timeout_ms as u64 + (round_number as u64 - 1) * increment_ms as u64;
        let timeout_ms = std::cmp::min(timeout_ms, self.lock().max_timeout.as_millis() as u64);

        Duration::from_millis(timeout_ms as u64)
    }

    fn start_timer(
        &self,
        slot_index: SlotIndex,
        timer: Timer,
        timeout: Duration,
        cb: DriverCallback,
    ) {
        todo!()
    }

    fn stop_timer(&self, slot_index: SlotIndex, timer: Timer) {
        todo!()
    }
}
