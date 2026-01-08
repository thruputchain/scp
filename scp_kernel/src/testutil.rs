use crate::*;
use sha2::{Digest, Sha256};
use xdr_common::VecM;

pub fn make_envelope(
    secret_key: &scp_proto::SecretKey,
    slot_index: scp_proto::SCPSlotIndex,
    mut statement: scp_proto::SCPStatement,
) -> Result<scp_proto::SCPEnvelope, scp_util::SigningError> {
    statement.node_id = derive_public_key(secret_key);
    statement.slot_index = slot_index;
    let signature = scp_util::sign_xdr(secret_key, &statement)?;
    Ok(scp_proto::SCPEnvelope {
        statement,
        signature,
    })
}

pub fn make_externalize(
    secret_key: &scp_proto::SecretKey,
    qset_hash: scp_proto::SCPHash,
    slot_index: scp_proto::SCPSlotIndex,
    commit_ballot: scp_proto::SCPBallot,
    n_h: scp_proto::SCPCount,
) -> Result<scp_proto::SCPEnvelope, scp_util::SigningError> {
    let statement = scp_proto::SCPStatement {
        node_id: derive_public_key(secret_key),
        slot_index,
        pledges: scp_proto::SCPStatement_pledges::SCP_ST_EXTERNALIZE {
            externalize: scp_proto::SCPStatementExternalize {
                commit: commit_ballot,
                n_h,
                commit_quorum_set_hash: qset_hash,
            },
        },
    };
    make_envelope(secret_key, slot_index, statement)
}

pub fn make_confirm(
    secret_key: &scp_proto::SecretKey,
    qset_hash: scp_proto::SCPHash,
    slot_index: scp_proto::SCPSlotIndex,
    prepare_counter: scp_proto::SCPCount,
    ballot: scp_proto::SCPBallot,
    n_c: scp_proto::SCPCount,
    n_h: scp_proto::SCPCount,
) -> Result<scp_proto::SCPEnvelope, scp_util::SigningError> {
    let statement = scp_proto::SCPStatement {
        node_id: derive_public_key(secret_key),
        slot_index,
        pledges: scp_proto::SCPStatement_pledges::SCP_ST_CONFIRM {
            confirm: scp_proto::SCPStatementConfirm {
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

pub fn make_prepare(
    secret_key: &scp_proto::SecretKey,
    qset_hash: scp_proto::SCPHash,
    slot_index: scp_proto::SCPSlotIndex,
    ballot: scp_proto::SCPBallot,
    prepared: Option<scp_proto::SCPBallot>,
    n_c: scp_proto::SCPCount,
    n_h: scp_proto::SCPCount,
    prepared_prime: Option<scp_proto::SCPBallot>,
) -> Result<scp_proto::SCPEnvelope, scp_util::SigningError> {
    let statement = scp_proto::SCPStatement {
        node_id: derive_public_key(secret_key),
        slot_index,
        pledges: scp_proto::SCPStatement_pledges::SCP_ST_PREPARE {
            prepare: scp_proto::SCPStatementPrepare {
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
    secret_key: &scp_proto::SecretKey,
    qset_hash: scp_proto::SCPHash,
    slot_index: scp_proto::SCPSlotIndex,
    mut votes: Vec<scp_proto::SCPValue>,
    mut accepted: Vec<scp_proto::SCPValue>,
) -> Result<scp_proto::SCPEnvelope, scp_util::SigningError> {
    // Sort votes and accepted as in C++ version
    votes.sort();
    accepted.sort();

    // Convert to VecM
    let votes_vecm = VecM(votes);
    let accepted_vecm = VecM(accepted);

    let statement = scp_proto::SCPStatement {
        node_id: derive_public_key(secret_key),
        slot_index,
        pledges: scp_proto::SCPStatement_pledges::SCP_ST_NOMINATE {
            nominate: scp_proto::SCPStatementNominate {
                quorum_set_hash: qset_hash,
                votes: votes_vecm,
                accepted: accepted_vecm,
            },
        },
    };
    make_envelope(secret_key, slot_index, statement)
}

// Verification functions - compare actual envelope with expected envelope
pub fn verify_prepare(
    actual: &scp_proto::SCPEnvelope,
    secret_key: &scp_proto::SecretKey,
    qset_hash: scp_proto::SCPHash,
    slot_index: scp_proto::SCPSlotIndex,
    ballot: scp_proto::SCPBallot,
    prepared: Option<scp_proto::SCPBallot>,
    n_c: scp_proto::SCPCount,
    n_h: scp_proto::SCPCount,
    prepared_prime: Option<scp_proto::SCPBallot>,
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

pub fn verify_confirm(
    actual: &scp_proto::SCPEnvelope,
    secret_key: &scp_proto::SecretKey,
    qset_hash: scp_proto::SCPHash,
    slot_index: scp_proto::SCPSlotIndex,
    prepare_counter: scp_proto::SCPCount,
    ballot: scp_proto::SCPBallot,
    n_c: scp_proto::SCPCount,
    n_h: scp_proto::SCPCount,
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
    actual: &scp_proto::SCPEnvelope,
    secret_key: &scp_proto::SecretKey,
    qset_hash: scp_proto::SCPHash,
    slot_index: scp_proto::SCPSlotIndex,
    commit_ballot: scp_proto::SCPBallot,
    n_h: scp_proto::SCPCount,
) {
    let expected = make_externalize(secret_key, qset_hash, slot_index, commit_ballot, n_h)
        .expect("Failed to create expected envelope");

    assert_eq!(expected.statement, actual.statement,);
}

pub fn verify_nominate(
    actual: &scp_proto::SCPEnvelope,
    secret_key: &scp_proto::SecretKey,
    qset_hash: scp_proto::SCPHash,
    slot_index: scp_proto::SCPSlotIndex,
    votes: Vec<scp_proto::SCPValue>,
    accepted: Vec<scp_proto::SCPValue>,
) {
    let expected = make_nominate(secret_key, qset_hash, slot_index, votes, accepted)
        .expect("Failed to create expected envelope");

    assert_eq!(expected.statement, actual.statement,);
}

#[derive(Clone)]
pub struct TestValues {
    pub x_value: scp_proto::SCPValue,
    pub y_value: scp_proto::SCPValue,
    pub z_value: scp_proto::SCPValue,
    pub zz_value: scp_proto::SCPValue,
    pub k_value: scp_proto::SCPValue,
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
    pub secret_key: scp_proto::SecretKey,
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

    pub fn node_id(&self) -> scp_proto::SCPNodeID {
        scp_util::derive_public_key(&self.secret_key)
    }
}
