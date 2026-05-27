use crate::{Interval, federated};
use std::collections::HashMap;
use thiserror::Error;

/// XDR-encoded quorum set bytes keyed by their SCP hash.
pub type SCPQuorumSetBytes = Vec<u8>;

/// Maximum envelopes accepted in one externalized-slot proof.
pub const MAX_PROOF_ENVELOPES: usize = 1000;

/// Maximum companion quorum sets accepted in one externalized-slot proof.
pub const MAX_PROOF_COMPANION_QSETS: usize = 1000;

/// Maximum XDR bytes accepted for one companion quorum set.
pub const MAX_COMPANION_QSET_BYTES: usize = 64 * 1024;

/// Maximum aggregate companion quorum set XDR bytes accepted in one proof.
pub const MAX_TOTAL_COMPANION_QSET_BYTES: usize = 4 * 1024 * 1024;

/// Specific reason an SCP proof envelope is malformed for externalization
/// verification.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum MalformedEnvelopeReason {
    #[error("unsupported statement type")]
    UnsupportedStatementType,
    #[error("missing statement pledge")]
    MissingStatementPledge,
    #[error("CONFIRM ballot counter is zero")]
    ConfirmZeroBallotCounter,
    #[error("CONFIRM high counter is above ballot counter")]
    ConfirmHighAboveBallotCounter,
    #[error("CONFIRM commit counter is above high counter")]
    ConfirmCommitAboveHigh,
    #[error("EXTERNALIZE commit counter is zero")]
    ExternalizeZeroCommitCounter,
    #[error("EXTERNALIZE commit counter is above high counter")]
    ExternalizeCommitAboveHigh,
}

impl From<federated::CompanionQuorumSetHashError> for MalformedEnvelopeReason {
    fn from(error: federated::CompanionQuorumSetHashError) -> Self {
        match error {
            federated::CompanionQuorumSetHashError::MissingStatementPledge => {
                Self::MissingStatementPledge
            }
        }
    }
}

/// Verification failure for an SCP externalization proof.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum VerifyError {
    #[error("bad signature from node {node_id:?}")]
    BadSignature { node_id: scp_proto::SCPNodeID },
    #[error("duplicate statement from node {node_id:?}")]
    DuplicateNodeStatement { node_id: scp_proto::SCPNodeID },
    #[error("unknown quorum set {hash:?}")]
    UnknownQuorumSet { hash: scp_proto::SCPHash },
    #[error("quorum set hash mismatch: claimed {claimed:?}, actual {actual:?}")]
    QuorumSetHashMismatch {
        claimed: scp_proto::SCPHash,
        actual: scp_proto::SCPHash,
    },
    #[error("malformed quorum set {hash:?}")]
    MalformedQuorumSet { hash: scp_proto::SCPHash },
    #[error("too many proof envelopes: max {max}, actual {actual}")]
    TooManyEnvelopes { max: usize, actual: usize },
    #[error("too many companion quorum sets: max {max}, actual {actual}")]
    TooManyCompanionQuorumSets { max: usize, actual: usize },
    #[error(
        "companion quorum set too large: max {max} bytes, actual {actual} bytes"
    )]
    CompanionQuorumSetTooLarge { max: usize, actual: usize },
    #[error(
        "companion quorum sets too large: max {max} bytes, actual {actual} bytes"
    )]
    CompanionQuorumSetsTooLarge { max: usize, actual: usize },
    #[error("slot mismatch: expected {expected}, found {found}")]
    SlotMismatch {
        expected: scp_proto::SCPSlotIndex,
        found: scp_proto::SCPSlotIndex,
    },
    #[error("proof statements do not bind to the supplied value")]
    ValueMismatch,
    #[error("proof statements do not form a quorum-witnessed commit interval")]
    SubQuorum,
    #[error("malformed proof envelope: {reason}")]
    MalformedEnvelope { reason: MalformedEnvelopeReason },
}

/// Verifies that `envelopes` prove SCP externalization of `value` at
/// `slot_index` under `trust_qset`.
///
/// `peer_qsets` must contain the XDR-encoded companion quorum set bytes for
/// every quorum-set hash referenced by the supplied envelopes. `envelopes`
/// must contain at most one statement per signer node. The verifier rejects
/// oversized proof material before allocating or decoding it.
pub fn verify_externalized(
    trust_qset: &scp_proto::SCPQuorumSet,
    peer_qsets: &HashMap<scp_proto::SCPHash, SCPQuorumSetBytes>,
    slot_index: scp_proto::SCPSlotIndex,
    value: &scp_proto::SCPValue,
    envelopes: &[scp_proto::SCPEnvelope],
) -> Result<(), VerifyError> {
    validate_proof_bounds(peer_qsets, envelopes)?;

    let decoded_qsets = decode_peer_qsets(peer_qsets)?;
    let envelopes_by_node = verify_envelopes(slot_index, envelopes)?;

    for envelope in envelopes_by_node.values() {
        let hash = proof_companion_qset_hash(&envelope.statement)?;
        if !decoded_qsets.contains_key(&hash) {
            return Err(VerifyError::UnknownQuorumSet { hash });
        }
    }

    let boundaries = commit_boundaries(value, envelopes_by_node.values())?;
    if boundaries.is_empty() {
        return Err(VerifyError::SubQuorum);
    }

    let mut candidate = Interval { lo: 0, hi: 0 };
    find_extended_interval(&mut candidate, &boundaries, |cur| {
        federated::is_quorum(
            trust_qset,
            &envelopes_by_node,
            &|st| {
                let hash = proof_companion_qset_hash(st).ok()?;
                decoded_qsets.get(&hash).cloned()
            },
            &|st| commit_predicate(value, cur, st),
        )
    });

    if candidate.lo == 0 {
        return Err(VerifyError::SubQuorum);
    }

    Ok(())
}

fn validate_proof_bounds(
    peer_qsets: &HashMap<scp_proto::SCPHash, SCPQuorumSetBytes>,
    envelopes: &[scp_proto::SCPEnvelope],
) -> Result<(), VerifyError> {
    if envelopes.len() > MAX_PROOF_ENVELOPES {
        return Err(VerifyError::TooManyEnvelopes {
            max: MAX_PROOF_ENVELOPES,
            actual: envelopes.len(),
        });
    }

    if peer_qsets.len() > MAX_PROOF_COMPANION_QSETS {
        return Err(VerifyError::TooManyCompanionQuorumSets {
            max: MAX_PROOF_COMPANION_QSETS,
            actual: peer_qsets.len(),
        });
    }

    let mut total_qset_bytes = 0usize;
    for bytes in peer_qsets.values() {
        if bytes.len() > MAX_COMPANION_QSET_BYTES {
            return Err(VerifyError::CompanionQuorumSetTooLarge {
                max: MAX_COMPANION_QSET_BYTES,
                actual: bytes.len(),
            });
        }

        total_qset_bytes = total_qset_bytes.checked_add(bytes.len()).ok_or(
            VerifyError::CompanionQuorumSetsTooLarge {
                max: MAX_TOTAL_COMPANION_QSET_BYTES,
                actual: usize::MAX,
            },
        )?;
        if total_qset_bytes > MAX_TOTAL_COMPANION_QSET_BYTES {
            return Err(VerifyError::CompanionQuorumSetsTooLarge {
                max: MAX_TOTAL_COMPANION_QSET_BYTES,
                actual: total_qset_bytes,
            });
        }
    }

    Ok(())
}

fn decode_peer_qsets(
    peer_qsets: &HashMap<scp_proto::SCPHash, SCPQuorumSetBytes>,
) -> Result<HashMap<scp_proto::SCPHash, scp_proto::SCPQuorumSet>, VerifyError> {
    let mut decoded = HashMap::with_capacity(peer_qsets.len());

    for (claimed, bytes) in peer_qsets {
        let qset: scp_proto::SCPQuorumSet = scp_util::unmarshal_xdr(bytes)
            .map_err(|_| VerifyError::MalformedQuorumSet { hash: *claimed })?;

        federated::is_qset_sane(&qset, false)
            .map_err(|_| VerifyError::MalformedQuorumSet { hash: *claimed })?;

        let actual = scp_util::hash_xdr_scp(&qset)
            .map_err(|_| VerifyError::MalformedQuorumSet { hash: *claimed })?;
        if actual != *claimed {
            return Err(VerifyError::QuorumSetHashMismatch {
                claimed: *claimed,
                actual,
            });
        }

        decoded.insert(*claimed, qset);
    }

    Ok(decoded)
}

fn verify_envelopes(
    slot_index: scp_proto::SCPSlotIndex,
    envelopes: &[scp_proto::SCPEnvelope],
) -> Result<HashMap<scp_proto::SCPNodeID, scp_proto::SCPEnvelope>, VerifyError>
{
    let mut by_node = HashMap::with_capacity(envelopes.len());

    for envelope in envelopes {
        let found = envelope.statement.slot_index;
        if found != slot_index {
            return Err(VerifyError::SlotMismatch {
                expected: slot_index,
                found,
            });
        }

        scp_util::verify_envelope(envelope).map_err(|_| {
            VerifyError::BadSignature {
                node_id: envelope.statement.node_id.clone(),
            }
        })?;

        let node_id = envelope.statement.node_id.clone();
        if by_node.insert(node_id.clone(), envelope.clone()).is_some() {
            return Err(VerifyError::DuplicateNodeStatement { node_id });
        }
    }

    Ok(by_node)
}

fn proof_companion_qset_hash(
    statement: &scp_proto::SCPStatement,
) -> Result<scp_proto::SCPHash, VerifyError> {
    if matches!(
        statement.pledges,
        scp_proto::SCPStatement_pledges::SCP_ST_NOMINATE { .. }
    ) {
        return Err(VerifyError::MalformedEnvelope {
            reason: MalformedEnvelopeReason::UnsupportedStatementType,
        });
    }

    federated::try_get_companion_quorum_set_hash_from_statement(statement)
        .map_err(|error| VerifyError::MalformedEnvelope {
            reason: error.into(),
        })
}

fn commit_boundaries<'a>(
    value: &scp_proto::SCPValue,
    envelopes: impl Iterator<Item = &'a scp_proto::SCPEnvelope>,
) -> Result<std::collections::HashSet<scp_proto::SCPCount>, VerifyError> {
    let mut res = std::collections::HashSet::new();
    let mut saw_ballot_statement = false;
    let mut saw_value_match = false;

    for envelope in envelopes {
        match &envelope.statement.pledges {
            scp_proto::SCPStatement_pledges::SCP_ST_PREPARE { .. } => {}
            scp_proto::SCPStatement_pledges::SCP_ST_CONFIRM { confirm } => {
                saw_ballot_statement = true;
                check_confirm_sane(confirm)?;
                if confirm_value_matches(value, confirm) {
                    saw_value_match = true;
                    res.insert(confirm.n_commit);
                    res.insert(confirm.n_h);
                }
            }
            scp_proto::SCPStatement_pledges::SCP_ST_EXTERNALIZE {
                externalize,
            } => {
                saw_ballot_statement = true;
                check_externalize_sane(externalize)?;
                if externalize_value_matches(value, externalize) {
                    saw_value_match = true;
                    res.insert(externalize.commit.counter);
                    res.insert(externalize.n_h);
                    res.insert(u32::MAX);
                }
            }
            scp_proto::SCPStatement_pledges::SCP_ST_NOMINATE { .. } => {
                return Err(malformed_envelope(
                    MalformedEnvelopeReason::UnsupportedStatementType,
                ));
            }
            _ => {
                return Err(malformed_envelope(
                    MalformedEnvelopeReason::MissingStatementPledge,
                ));
            }
        }
    }

    if saw_ballot_statement && !saw_value_match {
        return Err(VerifyError::ValueMismatch);
    }

    Ok(res)
}

fn malformed_envelope(reason: MalformedEnvelopeReason) -> VerifyError {
    VerifyError::MalformedEnvelope { reason }
}

fn check_confirm_sane(
    confirm: &scp_proto::SCPStatementConfirm,
) -> Result<(), VerifyError> {
    if confirm.ballot.counter == 0 {
        return Err(malformed_envelope(
            MalformedEnvelopeReason::ConfirmZeroBallotCounter,
        ));
    }
    if confirm.n_h > confirm.ballot.counter {
        return Err(malformed_envelope(
            MalformedEnvelopeReason::ConfirmHighAboveBallotCounter,
        ));
    }
    if confirm.n_commit > confirm.n_h {
        return Err(malformed_envelope(
            MalformedEnvelopeReason::ConfirmCommitAboveHigh,
        ));
    }
    Ok(())
}

fn check_externalize_sane(
    externalize: &scp_proto::SCPStatementExternalize,
) -> Result<(), VerifyError> {
    if externalize.commit.counter == 0 {
        return Err(malformed_envelope(
            MalformedEnvelopeReason::ExternalizeZeroCommitCounter,
        ));
    }
    if externalize.commit.counter > externalize.n_h {
        return Err(malformed_envelope(
            MalformedEnvelopeReason::ExternalizeCommitAboveHigh,
        ));
    }
    Ok(())
}

fn confirm_value_matches(
    value: &scp_proto::SCPValue,
    confirm: &scp_proto::SCPStatementConfirm,
) -> bool {
    &confirm.ballot.value == value
}

fn externalize_value_matches(
    value: &scp_proto::SCPValue,
    externalize: &scp_proto::SCPStatementExternalize,
) -> bool {
    &externalize.commit.value == value
}

fn commit_predicate(
    value: &scp_proto::SCPValue,
    check: &Interval,
    st: &scp_proto::SCPStatement,
) -> bool {
    match &st.pledges {
        scp_proto::SCPStatement_pledges::SCP_ST_PREPARE { .. } => false,
        scp_proto::SCPStatement_pledges::SCP_ST_CONFIRM { confirm } => {
            check_confirm_sane(confirm).is_ok()
                && confirm_value_matches(value, confirm)
                && confirm.n_commit <= check.lo
                && check.hi <= confirm.n_h
        }
        scp_proto::SCPStatement_pledges::SCP_ST_EXTERNALIZE { externalize } => {
            check_externalize_sane(externalize).is_ok()
                && externalize_value_matches(value, externalize)
                && externalize.commit.counter <= check.lo
        }
        _ => false,
    }
}

fn find_extended_interval<F>(
    candidate: &mut Interval,
    boundaries: &std::collections::HashSet<scp_proto::SCPCount>,
    pred: F,
) where
    F: Fn(&Interval) -> bool,
{
    let mut sorted_boundaries: Vec<scp_proto::SCPCount> =
        boundaries.iter().copied().collect();
    sorted_boundaries.sort();

    for &b in sorted_boundaries.iter().rev() {
        let cur = if candidate.lo == 0 {
            Interval { lo: b, hi: b }
        } else if b > candidate.hi {
            continue;
        } else {
            Interval {
                lo: b,
                hi: candidate.hi,
            }
        };

        if pred(&cur) {
            *candidate = cur;
        } else if candidate.lo != 0 {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::*;
    use scp_proto::*;
    use xdr_common::VecM;

    struct ProofFixture {
        trust_qset: SCPQuorumSet,
        peer_qsets: HashMap<SCPHash, SCPQuorumSetBytes>,
        envelopes: Vec<SCPEnvelope>,
        value: SCPValue,
        qset_hash: SCPHash,
        nodes: Vec<TestNode>,
    }

    fn flat_qset(nodes: &[TestNode], threshold: SCPCount) -> SCPQuorumSet {
        SCPQuorumSet {
            threshold,
            validators: VecM(nodes.iter().map(TestNode::node_id).collect()),
            inner_sets: VecM(vec![]),
        }
    }

    fn qset_bytes(qset: &SCPQuorumSet) -> SCPQuorumSetBytes {
        scp_util::marshal_xdr(qset).unwrap()
    }

    fn fixture() -> ProofFixture {
        let nodes = TestNode::new_batch(4);
        let trust_qset = flat_qset(&nodes, 3);
        let qset_hash = scp_util::hash_xdr_scp(&trust_qset).unwrap();
        let value = TestValues::default().x_value;
        let ballot = SCPBallot {
            counter: 4,
            value: value.clone(),
        };
        let envelopes = nodes
            .iter()
            .take(3)
            .map(|node| {
                make_confirm(
                    &node.secret_key,
                    qset_hash,
                    7,
                    2,
                    ballot.clone(),
                    2,
                    4,
                )
                .unwrap()
            })
            .collect();
        let mut peer_qsets = HashMap::new();
        peer_qsets.insert(qset_hash, qset_bytes(&trust_qset));

        ProofFixture {
            trust_qset,
            peer_qsets,
            envelopes,
            value,
            qset_hash,
            nodes,
        }
    }

    fn verify(fixture: &ProofFixture) -> Result<(), VerifyError> {
        verify_externalized(
            &fixture.trust_qset,
            &fixture.peer_qsets,
            7,
            &fixture.value,
            &fixture.envelopes,
        )
    }

    fn test_hash(index: usize) -> SCPHash {
        let mut hash = [0u8; 32];
        hash[..std::mem::size_of::<usize>()]
            .copy_from_slice(&index.to_le_bytes());
        hash
    }

    #[test]
    fn verifies_confirm_quorum() {
        assert_eq!(verify(&fixture()), Ok(()));
    }

    #[test]
    fn rejects_too_many_proof_envelopes() {
        let mut fixture = fixture();
        fixture.envelopes =
            vec![fixture.envelopes[0].clone(); MAX_PROOF_ENVELOPES + 1];

        assert_eq!(
            verify(&fixture),
            Err(VerifyError::TooManyEnvelopes {
                max: MAX_PROOF_ENVELOPES,
                actual: MAX_PROOF_ENVELOPES + 1
            })
        );
    }

    #[test]
    fn rejects_too_many_companion_qsets() {
        let mut fixture = fixture();
        fixture.peer_qsets.clear();
        for i in 0..=MAX_PROOF_COMPANION_QSETS {
            fixture.peer_qsets.insert(test_hash(i), Vec::new());
        }

        assert_eq!(
            verify(&fixture),
            Err(VerifyError::TooManyCompanionQuorumSets {
                max: MAX_PROOF_COMPANION_QSETS,
                actual: MAX_PROOF_COMPANION_QSETS + 1
            })
        );
    }

    #[test]
    fn rejects_oversized_companion_qset() {
        let mut fixture = fixture();
        fixture.peer_qsets.clear();
        fixture
            .peer_qsets
            .insert(fixture.qset_hash, vec![0u8; MAX_COMPANION_QSET_BYTES + 1]);

        assert_eq!(
            verify(&fixture),
            Err(VerifyError::CompanionQuorumSetTooLarge {
                max: MAX_COMPANION_QSET_BYTES,
                actual: MAX_COMPANION_QSET_BYTES + 1
            })
        );
    }

    #[test]
    fn rejects_oversized_total_companion_qsets() {
        let mut fixture = fixture();
        fixture.peer_qsets.clear();
        let qset_count =
            (MAX_TOTAL_COMPANION_QSET_BYTES / MAX_COMPANION_QSET_BYTES) + 1;
        for i in 0..qset_count {
            fixture
                .peer_qsets
                .insert(test_hash(i), vec![0u8; MAX_COMPANION_QSET_BYTES]);
        }

        assert_eq!(
            verify(&fixture),
            Err(VerifyError::CompanionQuorumSetsTooLarge {
                max: MAX_TOTAL_COMPANION_QSET_BYTES,
                actual: qset_count * MAX_COMPANION_QSET_BYTES
            })
        );
    }

    #[test]
    fn verifies_externalize_quorum() {
        let mut fixture = fixture();
        let ballot = SCPBallot {
            counter: 2,
            value: fixture.value.clone(),
        };
        fixture.envelopes = fixture
            .nodes
            .iter()
            .take(3)
            .map(|node| {
                make_externalize(
                    &node.secret_key,
                    fixture.qset_hash,
                    7,
                    ballot.clone(),
                    4,
                )
                .unwrap()
            })
            .collect();

        assert_eq!(verify(&fixture), Ok(()));
    }

    #[test]
    fn rejects_bad_signature() {
        let mut fixture = fixture();
        let SCPEnvelope { signature, .. } = &mut fixture.envelopes[0];
        let SCPSignature::Ed25519 { ed25519 } = signature else {
            panic!("expected ed25519 signature")
        };
        let mut bytes = ed25519.into_inner();
        bytes[0] ^= 0xff;
        *ed25519 = SCPEd25519Signature::new(bytes);
        assert!(matches!(
            verify(&fixture),
            Err(VerifyError::BadSignature { .. })
        ));
    }

    #[test]
    fn rejects_sub_quorum() {
        let mut fixture = fixture();
        fixture.envelopes.pop();
        assert_eq!(verify(&fixture), Err(VerifyError::SubQuorum));
    }

    #[test]
    fn rejects_value_mismatch() {
        let mut fixture = fixture();
        fixture.value = TestValues::default().y_value;
        assert_eq!(verify(&fixture), Err(VerifyError::ValueMismatch));
    }

    #[test]
    fn rejects_slot_mismatch() {
        let fixture = fixture();
        let err = verify_externalized(
            &fixture.trust_qset,
            &fixture.peer_qsets,
            8,
            &fixture.value,
            &fixture.envelopes,
        );
        assert_eq!(
            err,
            Err(VerifyError::SlotMismatch {
                expected: 8,
                found: 7
            })
        );
    }

    #[test]
    fn rejects_unknown_qset() {
        let mut fixture = fixture();
        fixture.peer_qsets.clear();
        assert_eq!(
            verify(&fixture),
            Err(VerifyError::UnknownQuorumSet {
                hash: fixture.qset_hash
            })
        );
    }

    #[test]
    fn rejects_qset_hash_mismatch() {
        let mut fixture = fixture();
        let other = flat_qset(&fixture.nodes[0..3], 2);
        fixture
            .peer_qsets
            .insert(fixture.qset_hash, qset_bytes(&other));
        assert!(matches!(
            verify(&fixture),
            Err(VerifyError::QuorumSetHashMismatch { .. })
        ));
    }

    #[test]
    fn rejects_malformed_qset() {
        let mut fixture = fixture();
        let bad_qset = SCPQuorumSet {
            threshold: 0,
            validators: VecM(vec![]),
            inner_sets: VecM(vec![]),
        };
        let bad_hash = scp_util::hash_xdr_scp(&bad_qset).unwrap();
        for envelope in &mut fixture.envelopes {
            if let SCPStatement_pledges::SCP_ST_CONFIRM { confirm } =
                &mut envelope.statement.pledges
            {
                confirm.quorum_set_hash = bad_hash;
            }
        }
        fixture.peer_qsets.clear();
        fixture.peer_qsets.insert(bad_hash, qset_bytes(&bad_qset));
        assert_eq!(
            verify(&fixture),
            Err(VerifyError::MalformedQuorumSet { hash: bad_hash })
        );
    }

    #[test]
    fn rejects_duplicate_signer() {
        let mut fixture = fixture();
        fixture.envelopes.push(fixture.envelopes[0].clone());
        assert!(matches!(
            verify(&fixture),
            Err(VerifyError::DuplicateNodeStatement { .. })
        ));
    }

    #[test]
    fn rejects_disjoint_intervals() {
        let mut fixture = fixture();
        let value = fixture.value.clone();
        fixture.envelopes = vec![
            make_confirm(
                &fixture.nodes[0].secret_key,
                fixture.qset_hash,
                7,
                1,
                SCPBallot {
                    counter: 2,
                    value: value.clone(),
                },
                1,
                2,
            )
            .unwrap(),
            make_confirm(
                &fixture.nodes[1].secret_key,
                fixture.qset_hash,
                7,
                4,
                SCPBallot {
                    counter: 5,
                    value: value.clone(),
                },
                4,
                5,
            )
            .unwrap(),
            make_confirm(
                &fixture.nodes[2].secret_key,
                fixture.qset_hash,
                7,
                7,
                SCPBallot { counter: 8, value },
                7,
                8,
            )
            .unwrap(),
        ];

        assert_eq!(verify(&fixture), Err(VerifyError::SubQuorum));
    }

    #[test]
    fn rejects_nominate_statement_as_unsupported() {
        let mut fixture = fixture();
        fixture.envelopes = vec![
            make_nominate(
                &fixture.nodes[0].secret_key,
                fixture.qset_hash,
                7,
                vec![fixture.value.clone()],
                vec![],
            )
            .unwrap(),
        ];

        assert_eq!(
            verify(&fixture),
            Err(VerifyError::MalformedEnvelope {
                reason: MalformedEnvelopeReason::UnsupportedStatementType
            })
        );
    }

    fn malformed_confirm_fixture(
        ballot_counter: SCPCount,
        n_commit: SCPCount,
        n_h: SCPCount,
    ) -> ProofFixture {
        let mut fixture = fixture();
        let value = fixture.value.clone();
        fixture.envelopes = fixture
            .nodes
            .iter()
            .take(3)
            .map(|node| {
                make_confirm(
                    &node.secret_key,
                    fixture.qset_hash,
                    7,
                    ballot_counter,
                    SCPBallot {
                        counter: ballot_counter,
                        value: value.clone(),
                    },
                    n_commit,
                    n_h,
                )
                .unwrap()
            })
            .collect();
        fixture
    }

    fn malformed_externalize_fixture(
        commit_counter: SCPCount,
        n_h: SCPCount,
    ) -> ProofFixture {
        let mut fixture = fixture();
        let value = fixture.value.clone();
        fixture.envelopes = fixture
            .nodes
            .iter()
            .take(3)
            .map(|node| {
                make_externalize(
                    &node.secret_key,
                    fixture.qset_hash,
                    7,
                    SCPBallot {
                        counter: commit_counter,
                        value: value.clone(),
                    },
                    n_h,
                )
                .unwrap()
            })
            .collect();
        fixture
    }

    #[test]
    fn rejects_confirm_with_zero_ballot_counter() {
        let fixture = malformed_confirm_fixture(0, 0, 0);
        assert_eq!(
            verify(&fixture),
            Err(VerifyError::MalformedEnvelope {
                reason: MalformedEnvelopeReason::ConfirmZeroBallotCounter
            })
        );
    }

    #[test]
    fn rejects_confirm_with_high_counter_above_ballot_counter() {
        let fixture = malformed_confirm_fixture(2, 2, 3);
        assert_eq!(
            verify(&fixture),
            Err(VerifyError::MalformedEnvelope {
                reason: MalformedEnvelopeReason::ConfirmHighAboveBallotCounter
            })
        );
    }

    #[test]
    fn rejects_confirm_with_commit_counter_above_high_counter() {
        let fixture = malformed_confirm_fixture(4, 5, 4);
        assert_eq!(
            verify(&fixture),
            Err(VerifyError::MalformedEnvelope {
                reason: MalformedEnvelopeReason::ConfirmCommitAboveHigh
            })
        );
    }

    #[test]
    fn rejects_externalize_with_zero_commit_counter() {
        let fixture = malformed_externalize_fixture(0, 0);
        assert_eq!(
            verify(&fixture),
            Err(VerifyError::MalformedEnvelope {
                reason: MalformedEnvelopeReason::ExternalizeZeroCommitCounter
            })
        );
    }

    #[test]
    fn rejects_externalize_with_commit_counter_above_high_counter() {
        let fixture = malformed_externalize_fixture(2, 1);
        assert_eq!(
            verify(&fixture),
            Err(VerifyError::MalformedEnvelope {
                reason: MalformedEnvelopeReason::ExternalizeCommitAboveHigh
            })
        );
    }
}
