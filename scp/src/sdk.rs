use async_trait::async_trait;
use proto::*;
use tokio_util::sync::CancellationToken;
use xdr_common::*;

pub enum ValidationLevel {
    Invalid,    // definitively invalid
    MaybeValid, // unable to fully decide at this stage; value is possibly valid
    FullyValid, // definitively valid at this stage (validate_tentatively or validate_fully)
}

/// App defines the calls that SCP makes to the user.
#[async_trait]
pub trait App {
    /// Prompts the SCP user to nominate a value for the given slot.
    /// Nomination can consume time, so SCP reserves the right to
    /// cancel a nomination request via the given cancellation token.
    async fn nominate(
        &self,
        slot_index: SCPSlotIndex,
        cancel: CancellationToken,
    ) -> Result<SCPValue, CancelledError>;

    /// Asks the SCP user to validate a value for a given slot tentatively.
    /// Effectively, giving the user a chance to reject values early in the balloting protocol, or
    /// to kick off expensive conclusive validation computation, whose outcome can be surfaced
    /// later when validate_fully is called.
    /// In groundhog, a validator may tenatively agree to a nomination based on a block hash
    /// before the block contents is available for a conclusive decision.
    async fn validate_tentatively(
        &self,
        slot_index: SCPSlotIndex,
        value: &SCPValue,
        cancel: CancellationToken,
    ) -> Result<ValidationLevel, CancelledError>;

    /// Asks the SCP user to validate a value for a given slot conclusively.
    /// Calls to validate_fully will always be preceded by a call to validate_tentatively for a given slot,
    /// but a call to validate_tentatively is not guaranteed to be followed by a call to validate_fully.
    async fn validate_fully(
        &self,
        slot_index: SCPSlotIndex,
        value: &SCPValue,
        cancel: CancellationToken,
    ) -> Result<ValidationLevel, CancelledError>;

    /// Notifies the SCP user that a value has been externalized for a given slot.
    fn externalized(&self, slot_index: SCPSlotIndex, value: SCPValue);
}

/// Send specifies the functionality that SCP requires in order to communicate
/// with peers, which includes networking and a sense of time.
#[async_trait]
pub trait Send {
    async fn disseminate(&self, msg: BytesM);
}

/// Receive specifies the external events that SCP handles.
pub trait Receive {
    fn received(&mut self, msg: BytesM);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelledError;
