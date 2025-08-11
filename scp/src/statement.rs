use crate::*;
use std::cmp::Ordering;
use std::mem::discriminant;

pub fn is_newer_statement(older: &proto::SCPStatement, newer: &proto::SCPStatement) -> bool {
    if discriminant(&older.pledges) != discriminant(&newer.pledges) {
        older.pledges.stmt_type() < newer.pledges.stmt_type()
    } else if newer.pledges.stmt_type() == proto::SCPStatementType::SCP_ST_EXTERNALIZE as i32 {
        false
    } else {
        match (&older.pledges, &newer.pledges) {
            // both confirm
            (
                proto::SCPStatement_pledges::SCP_ST_CONFIRM {
                    confirm: older_confirm,
                },
                proto::SCPStatement_pledges::SCP_ST_CONFIRM {
                    confirm: newer_confirm,
                },
            ) => match compare_ballots(&older_confirm.ballot, &newer_confirm.ballot) {
                Ordering::Less => true,
                Ordering::Equal => {
                    if older_confirm.n_prepared == newer_confirm.n_prepared {
                        older_confirm.n_h < newer_confirm.n_h
                    } else {
                        older_confirm.n_prepared < newer_confirm.n_prepared
                    }
                }
                Ordering::Greater => false,
            },
            // both prepare
            (
                proto::SCPStatement_pledges::SCP_ST_PREPARE {
                    prepare: older_prepare,
                },
                proto::SCPStatement_pledges::SCP_ST_PREPARE {
                    prepare: newer_prepare,
                },
            ) => match compare_ballots(&older_prepare.ballot, &newer_prepare.ballot) {
                Ordering::Less => true,
                Ordering::Equal => {
                    match compare_opt_ballots(&older_prepare.prepared, &newer_prepare.prepared) {
                        Ordering::Less => true,
                        Ordering::Equal => match compare_opt_ballots(
                            &older_prepare.prepared_prime,
                            &newer_prepare.prepared_prime,
                        ) {
                            Ordering::Less => true,
                            Ordering::Equal => older_prepare.n_h < newer_prepare.n_h,
                            Ordering::Greater => false,
                        },
                        Ordering::Greater => false,
                    }
                }
                Ordering::Greater => false,
            },
            // unreachable
            _ => panic!(),
        }
    }
}

// helper function to retrieve b for PREPARE, P for CONFIRM or
// c for EXTERNALIZE messages
pub fn get_working_ballot(statement: &proto::SCPStatement) -> &proto::SCPBallot {
    panic!("XXX")
}
