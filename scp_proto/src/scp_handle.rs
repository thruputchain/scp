// -*-Rust-*-
// Generated Code: DO NOT CHECK IN

// Generated from xdrc commit: 3f7aa40465e2d2834a4cf154e51fc9d19cc97d83

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(clippy::unnecessary_cast)]

use crate::*;
use core::option::Option;
use core::ops::{Deref, DerefMut};

#[cfg(feature = "std")]
use std::io::{Read, Write};
use xdrs::*;



use ::serde::{{Serialize, Deserialize}};



// clang-format on

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
#[derive(ReadXDR, WriteXDR)]

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(Copy)]
#[repr(transparent)]
pub struct SCPValidationLevel(pub i32);

impl SCPValidationLevel {
    pub const SCP_INVALID: Self = Self(0);
    pub const SCP_MAYBE_VALID: Self = Self(1);
    pub const SCP_FULLY_VALID: Self = Self(2);
}
impl From<i32> for SCPValidationLevel {
    fn from(v: i32) -> Self {
        Self(v)
    }
}
#[cfg(feature = "std")]
impl core::fmt::Display for SCPValidationLevel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0 {
            x if x == Self::SCP_INVALID.0 => {
                write!(f, "SCPValidationLevel::SCP_INVALID")
            },
            x if x == Self::SCP_MAYBE_VALID.0 => {
                write!(f, "SCPValidationLevel::SCP_MAYBE_VALID")
            },
            x if x == Self::SCP_FULLY_VALID.0 => {
                write!(f, "SCPValidationLevel::SCP_FULLY_VALID")
            },
            _ => {
                write!(f, "SCPValidationLevel({})", self.0)
            }
        }
    }
}



#[allow(non_camel_case_types)]
#[async_trait::async_trait]
pub trait SCP_Handle_SCP_Handle1: Send + Sync {    
    async fn scp_handle_null(&self) -> Result<(), ::rpc_driver::ProgramError>;
        // hooks


    async fn scp_handle_emit_envelope(&self, arg0 : SCPEnvelope) -> Result<(), ::rpc_driver::ProgramError>;
    
    async fn scp_handle_validate_ballot_value(&self, arg0 : SCPSlotIndex, arg1 : SCPValue) -> Result<SCPValidationLevel, ::rpc_driver::ProgramError>;
    
    async fn scp_handle_validate_nomination_value(&self, arg0 : SCPSlotIndex, arg1 : SCPValue) -> Result<SCPValidationLevel, ::rpc_driver::ProgramError>;
    
    async fn scp_handle_extract_valid_value(&self, arg0 : SCPSlotIndex, arg1 : SCPValue) -> Result<SCPOptValue, ::rpc_driver::ProgramError>;
    
    async fn scp_handle_combine_candidates(&self, arg0 : SCPSlotIndex, arg1 : SCPValues) -> Result<SCPOptValue, ::rpc_driver::ProgramError>;
        // notifications


    async fn scp_handle_value_externalized(&self, arg0 : SCPSlotIndex, arg1 : SCPValue) -> Result<(), ::rpc_driver::ProgramError>;
    
    async fn scp_handle_nominating_value(&self, arg0 : SCPSlotIndex, arg1 : SCPValue) -> Result<(), ::rpc_driver::ProgramError>;
    
    async fn scp_handle_updated_candidate_value(&self, arg0 : SCPSlotIndex, arg1 : SCPValue) -> Result<(), ::rpc_driver::ProgramError>;
    
    async fn scp_handle_started_ballot_proto(&self, arg0 : SCPSlotIndex, arg1 : SCPBallot) -> Result<(), ::rpc_driver::ProgramError>;
    
    async fn scp_handle_accepted_ballot_prepared(&self, arg0 : SCPSlotIndex, arg1 : SCPBallot) -> Result<(), ::rpc_driver::ProgramError>;
    
    async fn scp_handle_confirmed_ballot_prepared(&self, arg0 : SCPSlotIndex, arg1 : SCPBallot) -> Result<(), ::rpc_driver::ProgramError>;
    
    async fn scp_handle_accepted_commit(&self, arg0 : SCPSlotIndex, arg1 : SCPBallot) -> Result<(), ::rpc_driver::ProgramError>;
    
    async fn scp_handle_ballot_did_hear_from_quorum(&self, arg0 : SCPSlotIndex, arg1 : SCPBallot) -> Result<(), ::rpc_driver::ProgramError>;
        // utils

    async fn scp_handle_get_quorum_set(&self, arg0 : SCPHash) -> Result<SCPOptQuorumSet, ::rpc_driver::ProgramError>;
}


#[allow(non_camel_case_types)]
pub struct SCP_Handle_SCP_Handle1_Dispatcher {
  inner: Box<dyn SCP_Handle_SCP_Handle1>,
}

impl SCP_Handle_SCP_Handle1_Dispatcher {
  pub fn new(inner: Box<dyn SCP_Handle_SCP_Handle1>) -> Self {
    Self { inner }
  }
}

#[async_trait::async_trait]
impl ::rpc_driver::handler::RpcHandler for SCP_Handle_SCP_Handle1_Dispatcher {
    fn prog_vers(&self) -> ::rpc_driver::ProgVers {
        ::rpc_driver::ProgVers::new(0x9aa92002, 1)
    }
    async fn dispatch(&self, proc: u32, data: &[u8]) -> Result<Vec<u8>, ::rpc_driver::RpcError> {
        match proc {
            // scp_handle_null
            0 => {
                let args: () =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_null().await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_emit_envelope
            10 => {
                let args: (SCPEnvelope, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_emit_envelope(args.0).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_validate_ballot_value
            20 => {
                let args: (SCPSlotIndex, SCPValue, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_validate_ballot_value(args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_validate_nomination_value
            30 => {
                let args: (SCPSlotIndex, SCPValue, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_validate_nomination_value(args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_extract_valid_value
            40 => {
                let args: (SCPSlotIndex, SCPValue, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_extract_valid_value(args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_combine_candidates
            50 => {
                let args: (SCPSlotIndex, SCPValues, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_combine_candidates(args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_value_externalized
            100 => {
                let args: (SCPSlotIndex, SCPValue, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_value_externalized(args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_nominating_value
            200 => {
                let args: (SCPSlotIndex, SCPValue, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_nominating_value(args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_updated_candidate_value
            210 => {
                let args: (SCPSlotIndex, SCPValue, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_updated_candidate_value(args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_started_ballot_proto
            310 => {
                let args: (SCPSlotIndex, SCPBallot, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_started_ballot_proto(args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_accepted_ballot_prepared
            320 => {
                let args: (SCPSlotIndex, SCPBallot, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_accepted_ballot_prepared(args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_confirmed_ballot_prepared
            330 => {
                let args: (SCPSlotIndex, SCPBallot, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_confirmed_ballot_prepared(args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_accepted_commit
            340 => {
                let args: (SCPSlotIndex, SCPBallot, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_accepted_commit(args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_ballot_did_hear_from_quorum
            350 => {
                let args: (SCPSlotIndex, SCPBallot, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_ballot_did_hear_from_quorum(args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_get_quorum_set
            1000 => {
                let args: (SCPHash, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_get_quorum_set(args.0).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            _ => Err(::rpc_driver::RpcError::InvalidProcedure),
        }
    }
}

#[derive(Clone)]
pub struct SCP_Handle_SCP_Handle1_Client {
    handle: ::std::sync::Arc<dyn ::rpc_driver::RpcSender>,
}

impl SCP_Handle_SCP_Handle1_Client {
    pub fn new(handle: ::std::sync::Arc<dyn ::rpc_driver::RpcSender>) -> Self {
        Self { handle }
    }

    fn prog_vers() -> ::rpc_driver::ProgVers {
        ::rpc_driver::ProgVers::new(0x9aa92002, 1)
    }
    pub async fn scp_handle_null(&self) -> Result<(), ::rpc_driver::RpcError> {
         let data = xdrs::txs::encode(&()).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
         let buf = self.handle.send_call(Self::prog_vers(), 0, &data).await?;
         xdrs::txs::decode(&buf).map_err(|_| ::rpc_driver::RpcError::SystemError)
    }
    pub async fn scp_handle_emit_envelope(&self, arg0 : &SCPEnvelope) -> Result<(), ::rpc_driver::RpcError> {
         let data = xdrs::txs::encode(&(&arg0)).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
         let buf = self.handle.send_call(Self::prog_vers(), 10, &data).await?;
         xdrs::txs::decode(&buf).map_err(|_| ::rpc_driver::RpcError::SystemError)
    }
    pub async fn scp_handle_validate_ballot_value(&self, arg0 : &SCPSlotIndex, arg1 : &SCPValue) -> Result<SCPValidationLevel, ::rpc_driver::RpcError> {
         let data = xdrs::txs::encode(&(&arg0, &arg1)).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
         let buf = self.handle.send_call(Self::prog_vers(), 20, &data).await?;
         xdrs::txs::decode(&buf).map_err(|_| ::rpc_driver::RpcError::SystemError)
    }
    pub async fn scp_handle_validate_nomination_value(&self, arg0 : &SCPSlotIndex, arg1 : &SCPValue) -> Result<SCPValidationLevel, ::rpc_driver::RpcError> {
         let data = xdrs::txs::encode(&(&arg0, &arg1)).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
         let buf = self.handle.send_call(Self::prog_vers(), 30, &data).await?;
         xdrs::txs::decode(&buf).map_err(|_| ::rpc_driver::RpcError::SystemError)
    }
    pub async fn scp_handle_extract_valid_value(&self, arg0 : &SCPSlotIndex, arg1 : &SCPValue) -> Result<SCPOptValue, ::rpc_driver::RpcError> {
         let data = xdrs::txs::encode(&(&arg0, &arg1)).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
         let buf = self.handle.send_call(Self::prog_vers(), 40, &data).await?;
         xdrs::txs::decode(&buf).map_err(|_| ::rpc_driver::RpcError::SystemError)
    }
    pub async fn scp_handle_combine_candidates(&self, arg0 : &SCPSlotIndex, arg1 : &SCPValues) -> Result<SCPOptValue, ::rpc_driver::RpcError> {
         let data = xdrs::txs::encode(&(&arg0, &arg1)).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
         let buf = self.handle.send_call(Self::prog_vers(), 50, &data).await?;
         xdrs::txs::decode(&buf).map_err(|_| ::rpc_driver::RpcError::SystemError)
    }
    pub async fn scp_handle_value_externalized(&self, arg0 : &SCPSlotIndex, arg1 : &SCPValue) -> Result<(), ::rpc_driver::RpcError> {
         let data = xdrs::txs::encode(&(&arg0, &arg1)).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
         let buf = self.handle.send_call(Self::prog_vers(), 100, &data).await?;
         xdrs::txs::decode(&buf).map_err(|_| ::rpc_driver::RpcError::SystemError)
    }
    pub async fn scp_handle_nominating_value(&self, arg0 : &SCPSlotIndex, arg1 : &SCPValue) -> Result<(), ::rpc_driver::RpcError> {
         let data = xdrs::txs::encode(&(&arg0, &arg1)).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
         let buf = self.handle.send_call(Self::prog_vers(), 200, &data).await?;
         xdrs::txs::decode(&buf).map_err(|_| ::rpc_driver::RpcError::SystemError)
    }
    pub async fn scp_handle_updated_candidate_value(&self, arg0 : &SCPSlotIndex, arg1 : &SCPValue) -> Result<(), ::rpc_driver::RpcError> {
         let data = xdrs::txs::encode(&(&arg0, &arg1)).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
         let buf = self.handle.send_call(Self::prog_vers(), 210, &data).await?;
         xdrs::txs::decode(&buf).map_err(|_| ::rpc_driver::RpcError::SystemError)
    }
    pub async fn scp_handle_started_ballot_proto(&self, arg0 : &SCPSlotIndex, arg1 : &SCPBallot) -> Result<(), ::rpc_driver::RpcError> {
         let data = xdrs::txs::encode(&(&arg0, &arg1)).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
         let buf = self.handle.send_call(Self::prog_vers(), 310, &data).await?;
         xdrs::txs::decode(&buf).map_err(|_| ::rpc_driver::RpcError::SystemError)
    }
    pub async fn scp_handle_accepted_ballot_prepared(&self, arg0 : &SCPSlotIndex, arg1 : &SCPBallot) -> Result<(), ::rpc_driver::RpcError> {
         let data = xdrs::txs::encode(&(&arg0, &arg1)).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
         let buf = self.handle.send_call(Self::prog_vers(), 320, &data).await?;
         xdrs::txs::decode(&buf).map_err(|_| ::rpc_driver::RpcError::SystemError)
    }
    pub async fn scp_handle_confirmed_ballot_prepared(&self, arg0 : &SCPSlotIndex, arg1 : &SCPBallot) -> Result<(), ::rpc_driver::RpcError> {
         let data = xdrs::txs::encode(&(&arg0, &arg1)).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
         let buf = self.handle.send_call(Self::prog_vers(), 330, &data).await?;
         xdrs::txs::decode(&buf).map_err(|_| ::rpc_driver::RpcError::SystemError)
    }
    pub async fn scp_handle_accepted_commit(&self, arg0 : &SCPSlotIndex, arg1 : &SCPBallot) -> Result<(), ::rpc_driver::RpcError> {
         let data = xdrs::txs::encode(&(&arg0, &arg1)).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
         let buf = self.handle.send_call(Self::prog_vers(), 340, &data).await?;
         xdrs::txs::decode(&buf).map_err(|_| ::rpc_driver::RpcError::SystemError)
    }
    pub async fn scp_handle_ballot_did_hear_from_quorum(&self, arg0 : &SCPSlotIndex, arg1 : &SCPBallot) -> Result<(), ::rpc_driver::RpcError> {
         let data = xdrs::txs::encode(&(&arg0, &arg1)).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
         let buf = self.handle.send_call(Self::prog_vers(), 350, &data).await?;
         xdrs::txs::decode(&buf).map_err(|_| ::rpc_driver::RpcError::SystemError)
    }
    pub async fn scp_handle_get_quorum_set(&self, arg0 : &SCPHash) -> Result<SCPOptQuorumSet, ::rpc_driver::RpcError> {
         let data = xdrs::txs::encode(&(&arg0)).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
         let buf = self.handle.send_call(Self::prog_vers(), 1000, &data).await?;
         xdrs::txs::decode(&buf).map_err(|_| ::rpc_driver::RpcError::SystemError)
    }
}



//
// XDR internals below this point
//


