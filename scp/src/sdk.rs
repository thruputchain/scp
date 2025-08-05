use async_trait::async_trait;
use proto::*;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use xdr_common::*;

pub enum ValidationLevel {
    Invalid,
    MaybeValid,
    FullyValid,
}

#[async_trait]
pub trait SCP2User {
    async fn nominate(
        &self,
        slot_index: SCPSlotIndex,
        cancel: CancellationToken,
    ) -> Result<SCPValue, CancelledError>;
    async fn validate(
        &self,
        slot_index: SCPSlotIndex,
        cancel: CancellationToken,
    ) -> ValidationLevel;

    fn externalized(&self, slot_index: SCPSlotIndex, value: SCPValue);
}

#[async_trait]
pub trait SCP2World {
    async fn timeout(&self, duration: Duration);
    async fn with_timeout<F, T>(future: F, duration: Duration) -> Result<T, TimeoutError>;
    async fn disseminate(&self, msg: BytesM);
}

pub trait World2SCP {
    fn received(&mut self, msg: BytesM);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeoutError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelledError;
