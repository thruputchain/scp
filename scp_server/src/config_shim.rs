use scp_proto::{SCPConfig, SCPEd25519PublicKey, SCPPublicKey, SCPQuorumSet};
use serde::{Deserialize, Serialize};
use xdr_common::VecM;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SCPConfigShim {
    pub max_timeout_ms: u64,
    pub initial_ballot_timeout_ms: u64,
    pub increment_ballot_timeout_ms: u64,
    pub initial_nomination_timeout_ms: u64,
    pub increment_nomination_timeout_ms: u64,
    pub quorum_set: SCPQuorumSetShim,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SCPQuorumSetShim {
    pub threshold: u32,
    pub validators: Vec<SCPNodeIDShim>,
    pub inner_sets: Vec<SCPQuorumSetShim>,
}

pub type SCPNodeIDShim = [u8; 32];

impl SCPConfigShim {
    pub fn to_scp_config(self) -> SCPConfig {
        SCPConfig {
            max_timeout_ms: self.max_timeout_ms,
            initial_ballot_timeout_ms: self.initial_ballot_timeout_ms,
            increment_ballot_timeout_ms: self.increment_ballot_timeout_ms,
            initial_nomination_timeout_ms: self.initial_nomination_timeout_ms,
            increment_nomination_timeout_ms: self.increment_nomination_timeout_ms,
            quorum_set: self.quorum_set.to_scp_quorum_set(),
        }
    }
}

impl SCPQuorumSetShim {
    pub fn to_scp_quorum_set(self) -> SCPQuorumSet {
        SCPQuorumSet {
            threshold: self.threshold,
            validators: VecM::try_from(
                self.validators
                    .into_iter()
                    .map(|node_id| SCPPublicKey::Ed25519 {
                        ed25519: SCPEd25519PublicKey::from(node_id),
                    })
                    .collect::<Vec<_>>(),
            )
            .expect("Failed to create VecM"),
            inner_sets: VecM::try_from(
                self.inner_sets
                    .into_iter()
                    .map(|inner_set| inner_set.to_scp_quorum_set())
                    .collect::<Vec<_>>(),
            )
            .expect("Failed to create VecM"),
        }
    }
}
