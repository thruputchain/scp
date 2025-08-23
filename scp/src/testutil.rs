use crate::*;
use xdr_common::VecM;

pub fn make_envelope(
    secret_key: &proto::SecretKey,
    slot_index: proto::SCPSlotIndex,
    mut statement: proto::SCPStatement,
    public_key: proto::PublicKey,
) -> Result<proto::SCPEnvelope, proto::SigningError> {
    statement.node_id = public_key;
    statement.slot_index = slot_index;
    let signature = proto::sign_xdr(secret_key, &statement)?;
    Ok(proto::SCPEnvelope {
        statement,
        signature,
    })
}

pub fn make_externalize(
    secret_key: &proto::SecretKey,
    qset_hash: proto::SCPHash,
    slot_index: proto::SCPSlotIndex,
    commit_ballot: proto::SCPBallot,
    n_h: proto::SCPCount,
    public_key: proto::PublicKey,
) -> Result<proto::SCPEnvelope, proto::SigningError> {
    let statement = proto::SCPStatement {
        node_id: public_key.clone(),
        slot_index,
        pledges: proto::SCPStatement_pledges::SCP_ST_EXTERNALIZE {
            externalize: proto::SCPStatementExternalize {
                commit: commit_ballot,
                n_h,
                commit_quorum_set_hash: qset_hash,
            },
        },
    };
    make_envelope(secret_key, slot_index, statement, public_key)
}

pub fn make_confirm(
    secret_key: &proto::SecretKey,
    qset_hash: proto::SCPHash,
    slot_index: proto::SCPSlotIndex,
    prepare_counter: proto::SCPCount,
    ballot: proto::SCPBallot,
    n_c: proto::SCPCount,
    n_h: proto::SCPCount,
    public_key: proto::PublicKey,
) -> Result<proto::SCPEnvelope, proto::SigningError> {
    let statement = proto::SCPStatement {
        node_id: public_key.clone(),
        slot_index,
        pledges: proto::SCPStatement_pledges::SCP_ST_CONFIRM {
            confirm: proto::SCPStatementConfirm {
                ballot,
                n_prepared: prepare_counter,
                n_commit: n_c,
                n_h,
                quorum_set_hash: qset_hash,
            },
        },
    };
    make_envelope(secret_key, slot_index, statement, public_key)
}

pub fn make_prepare(
    secret_key: &proto::SecretKey,
    qset_hash: proto::SCPHash,
    slot_index: proto::SCPSlotIndex,
    ballot: proto::SCPBallot,
    prepared: Option<proto::SCPBallot>,
    n_c: proto::SCPCount,
    n_h: proto::SCPCount,
    prepared_prime: Option<proto::SCPBallot>,
    public_key: proto::PublicKey,
) -> Result<proto::SCPEnvelope, proto::SigningError> {
    let statement = proto::SCPStatement {
        node_id: public_key.clone(),
        slot_index,
        pledges: proto::SCPStatement_pledges::SCP_ST_PREPARE {
            prepare: proto::SCPStatementPrepare {
                quorum_set_hash: qset_hash,
                ballot,
                prepared,
                prepared_prime,
                n_c,
                n_h,
            },
        },
    };
    make_envelope(secret_key, slot_index, statement, public_key)
}

pub fn make_nominate(
    secret_key: &proto::SecretKey,
    qset_hash: proto::SCPHash,
    slot_index: proto::SCPSlotIndex,
    mut votes: Vec<proto::SCPValue>,
    mut accepted: Vec<proto::SCPValue>,
    public_key: proto::PublicKey,
) -> Result<proto::SCPEnvelope, proto::SigningError> {
    // Sort votes and accepted as in C++ version
    votes.sort();
    accepted.sort();
    
    // Convert to VecM
    let votes_vecm = VecM(votes);
    let accepted_vecm = VecM(accepted);
    
    let statement = proto::SCPStatement {
        node_id: public_key.clone(),
        slot_index,
        pledges: proto::SCPStatement_pledges::SCP_ST_NOMINATE {
            nominate: proto::SCPStatementNominate {
                quorum_set_hash: qset_hash,
                votes: votes_vecm,
                accepted: accepted_vecm,
            },
        },
    };
    make_envelope(secret_key, slot_index, statement, public_key)
}

