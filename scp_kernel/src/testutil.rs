use crate::*;
use sha2::{Digest, Sha256};
use xdr_common::VecM;

pub fn make_envelope(
    secret_key: &thruput_xdr::types::SecretKey,
    slot_index: thruput_scp::SCPSlotIndex,
    mut statement: thruput_scp::SCPStatement,
) -> Result<thruput_scp::SCPEnvelope, scp_util::SigningError> {
    statement.node_id = derive_public_key(secret_key);
    statement.slot_index = slot_index;
    let signature = scp_util::sign_xdr(secret_key, &statement)?;
    Ok(thruput_scp::SCPEnvelope {
        statement,
        signature,
    })
}

pub fn make_externalize(
    secret_key: &thruput_xdr::types::SecretKey,
    qset_hash: thruput_scp::SCPHash,
    slot_index: thruput_scp::SCPSlotIndex,
    commit_ballot: thruput_scp::SCPBallot,
    n_h: thruput_scp::SCPCount,
) -> Result<thruput_scp::SCPEnvelope, scp_util::SigningError> {
    let statement = thruput_scp::SCPStatement {
        node_id: derive_public_key(secret_key),
        slot_index,
        pledges: thruput_scp::SCPStatement_pledges::SCP_ST_EXTERNALIZE {
            externalize: thruput_scp::SCPStatementExternalize {
                commit: commit_ballot,
                n_h,
                commit_quorum_set_hash: qset_hash,
            },
        },
    };
    make_envelope(secret_key, slot_index, statement)
}

pub fn make_confirm(
    secret_key: &thruput_xdr::types::SecretKey,
    qset_hash: thruput_scp::SCPHash,
    slot_index: thruput_scp::SCPSlotIndex,
    prepare_counter: thruput_scp::SCPCount,
    ballot: thruput_scp::SCPBallot,
    n_c: thruput_scp::SCPCount,
    n_h: thruput_scp::SCPCount,
) -> Result<thruput_scp::SCPEnvelope, scp_util::SigningError> {
    let statement = thruput_scp::SCPStatement {
        node_id: derive_public_key(secret_key),
        slot_index,
        pledges: thruput_scp::SCPStatement_pledges::SCP_ST_CONFIRM {
            confirm: thruput_scp::SCPStatementConfirm {
                ballot,
                n_prepared: prepare_counter,
                n_commit: n_c,
                n_h,
                quorum_set_hash: qset_hash,
            },
        },
    };
    make_envelope(secret_key, slot_index, statement)
}

#[allow(clippy::too_many_arguments)]
pub fn make_prepare(
    secret_key: &thruput_xdr::types::SecretKey,
    qset_hash: thruput_scp::SCPHash,
    slot_index: thruput_scp::SCPSlotIndex,
    ballot: thruput_scp::SCPBallot,
    prepared: Option<thruput_scp::SCPBallot>,
    n_c: thruput_scp::SCPCount,
    n_h: thruput_scp::SCPCount,
    prepared_prime: Option<thruput_scp::SCPBallot>,
) -> Result<thruput_scp::SCPEnvelope, scp_util::SigningError> {
    let statement = thruput_scp::SCPStatement {
        node_id: derive_public_key(secret_key),
        slot_index,
        pledges: thruput_scp::SCPStatement_pledges::SCP_ST_PREPARE {
            prepare: thruput_scp::SCPStatementPrepare {
                quorum_set_hash: qset_hash,
                ballot,
                prepared,
                prepared_prime,
                n_c,
                n_h,
            },
        },
    };
    make_envelope(secret_key, slot_index, statement)
}

pub fn make_nominate(
    secret_key: &thruput_xdr::types::SecretKey,
    qset_hash: thruput_scp::SCPHash,
    slot_index: thruput_scp::SCPSlotIndex,
    mut votes: Vec<thruput_scp::SCPValue>,
    mut accepted: Vec<thruput_scp::SCPValue>,
) -> Result<thruput_scp::SCPEnvelope, scp_util::SigningError> {
    // Sort votes and accepted as in C++ version
    votes.sort();
    accepted.sort();

    // Convert to VecM
    let votes_vecm = VecM(votes);
    let accepted_vecm = VecM(accepted);

    let statement = thruput_scp::SCPStatement {
        node_id: derive_public_key(secret_key),
        slot_index,
        pledges: thruput_scp::SCPStatement_pledges::SCP_ST_NOMINATE {
            nominate: thruput_scp::SCPStatementNominate {
                quorum_set_hash: qset_hash,
                votes: votes_vecm,
                accepted: accepted_vecm,
            },
        },
    };
    make_envelope(secret_key, slot_index, statement)
}

// Verification functions - compare actual envelope with expected envelope
#[allow(clippy::too_many_arguments)]
pub fn verify_prepare(
    actual: &thruput_scp::SCPEnvelope,
    secret_key: &thruput_xdr::types::SecretKey,
    qset_hash: thruput_scp::SCPHash,
    slot_index: thruput_scp::SCPSlotIndex,
    ballot: thruput_scp::SCPBallot,
    prepared: Option<thruput_scp::SCPBallot>,
    n_c: thruput_scp::SCPCount,
    n_h: thruput_scp::SCPCount,
    prepared_prime: Option<thruput_scp::SCPBallot>,
) {
    let expected = make_prepare(
        secret_key,
        qset_hash,
        slot_index,
        ballot,
        prepared,
        n_c,
        n_h,
        prepared_prime,
    )
    .expect("Failed to create expected envelope");

    assert_eq!(expected.statement, actual.statement,);
}

#[allow(clippy::too_many_arguments)]
pub fn verify_confirm(
    actual: &thruput_scp::SCPEnvelope,
    secret_key: &thruput_xdr::types::SecretKey,
    qset_hash: thruput_scp::SCPHash,
    slot_index: thruput_scp::SCPSlotIndex,
    prepare_counter: thruput_scp::SCPCount,
    ballot: thruput_scp::SCPBallot,
    n_c: thruput_scp::SCPCount,
    n_h: thruput_scp::SCPCount,
) {
    let expected = make_confirm(
        secret_key,
        qset_hash,
        slot_index,
        prepare_counter,
        ballot,
        n_c,
        n_h,
    )
    .expect("Failed to create expected envelope");

    assert_eq!(expected.statement, actual.statement,);
}

pub fn verify_externalize(
    actual: &thruput_scp::SCPEnvelope,
    secret_key: &thruput_xdr::types::SecretKey,
    qset_hash: thruput_scp::SCPHash,
    slot_index: thruput_scp::SCPSlotIndex,
    commit_ballot: thruput_scp::SCPBallot,
    n_h: thruput_scp::SCPCount,
) {
    let expected =
        make_externalize(secret_key, qset_hash, slot_index, commit_ballot, n_h)
            .expect("Failed to create expected envelope");

    assert_eq!(expected.statement, actual.statement,);
}

pub fn verify_nominate(
    actual: &thruput_scp::SCPEnvelope,
    secret_key: &thruput_xdr::types::SecretKey,
    qset_hash: thruput_scp::SCPHash,
    slot_index: thruput_scp::SCPSlotIndex,
    votes: Vec<thruput_scp::SCPValue>,
    accepted: Vec<thruput_scp::SCPValue>,
) {
    let expected =
        make_nominate(secret_key, qset_hash, slot_index, votes, accepted)
            .expect("Failed to create expected envelope");

    assert_eq!(expected.statement, actual.statement,);
}

#[derive(Clone)]
pub struct TestValues {
    pub x_value: thruput_scp::SCPValue,
    pub y_value: thruput_scp::SCPValue,
    pub z_value: thruput_scp::SCPValue,
    pub zz_value: thruput_scp::SCPValue,
    pub k_value: thruput_scp::SCPValue,
}

impl Default for TestValues {
    fn default() -> Self {
        use sha2::{Digest, Sha256};
        use xdr_common::BytesM;

        let mut hasher = Sha256::new();
        hasher.update(format!("SEED_VALUE_DATA_{}", rand::random::<u64>()));
        let base_seed = format!("{:x}", hasher.finalize_reset());

        let mut values = Vec::new();
        for i in 0..4 {
            hasher.update(format!("{}/{}", base_seed, i));
            let hash = hasher.finalize_reset();
            values.push(BytesM(hash.to_vec()));
        }

        values.sort();

        hasher.update(&base_seed);
        let k_hash = hasher.finalize();
        let k_value = BytesM(k_hash.to_vec());

        TestValues {
            x_value: values[0].clone(),
            y_value: values[1].clone(),
            z_value: values[2].clone(),
            zz_value: values[3].clone(),
            k_value,
        }
    }
}

pub struct TestNode {
    pub secret_key: thruput_xdr::types::SecretKey,
}

impl TestNode {
    pub fn new_batch(n: i64) -> Vec<TestNode> {
        let mut v = Vec::new();
        for i in 0..n {
            v.push(TestNode::new(i));
        }
        v
    }

    pub fn new(n: i64) -> TestNode {
        let seed = Sha256::digest(format!("NODE_SEED_{}", n));
        let secret_key = scp_util::generate_secret_key(&seed);
        TestNode { secret_key }
    }

    pub fn node_id(&self) -> thruput_scp::SCPNodeID {
        scp_util::derive_public_key(&self.secret_key)
    }
}
