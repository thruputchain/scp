use thiserror::Error;

#[derive(Debug, Error)]
pub enum QSetError {
    #[error("verifying error")]
    XXXError,
}

pub fn is_quorum_set_sane(qset: &proto::SCPQuorumSet, extra_checks: bool) -> Result<(), QSetError> {
    panic!("XXX")
}
