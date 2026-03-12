// -*-Rust-*-
// Generated Code: DO NOT CHECK IN

// Generated from xdrc commit: 2696fbbb9386179cc9913c691fce5654f491da71

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
use ::serde_arrays;



// clang-format on

#[derive(Serialize, Deserialize)]
#[derive(ReadXDR, WriteXDR)]

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(Copy)]
#[repr(i32)]
pub enum SCPValidationLevel {
    SCP_INVALID = 0 as i32,
    SCP_MAYBE_VALID = 1 as i32,
    SCP_FULLY_VALID = 2 as i32,
}
impl TryFrom<i32> for SCPValidationLevel {
    type Error = ();
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::SCP_INVALID),
            1 => Ok(Self::SCP_MAYBE_VALID),
            2 => Ok(Self::SCP_FULLY_VALID),
            _ => Err(()),
        }
    }
}



#[allow(non_camel_case_types)]
#[async_trait::async_trait]
pub trait SCP_Handle_SCP_Handle1: Send + Sync {    
    async fn scp_handle_null(&self, client: &::std::sync::Arc<::rpc_driver::ClientHandle>) -> Result<(), ::rpc_driver::ProgramError>;
        // hooks


    async fn scp_handle_emit_envelope(&self, client: &::std::sync::Arc<::rpc_driver::ClientHandle>, arg0 : SCPEnvelope) -> Result<(), ::rpc_driver::ProgramError>;
    
    async fn scp_handle_validate_ballot_value(&self, client: &::std::sync::Arc<::rpc_driver::ClientHandle>, arg0 : SCPSlotIndex, arg1 : SCPValue) -> Result<SCPValidationLevel, ::rpc_driver::ProgramError>;
    
    async fn scp_handle_validate_nomination_value(&self, client: &::std::sync::Arc<::rpc_driver::ClientHandle>, arg0 : SCPSlotIndex, arg1 : SCPValue) -> Result<SCPValidationLevel, ::rpc_driver::ProgramError>;
    
    async fn scp_handle_extract_valid_value(&self, client: &::std::sync::Arc<::rpc_driver::ClientHandle>, arg0 : SCPSlotIndex, arg1 : SCPValue) -> Result<SCPOptValue, ::rpc_driver::ProgramError>;
    
    async fn scp_handle_combine_candidates(&self, client: &::std::sync::Arc<::rpc_driver::ClientHandle>, arg0 : SCPSlotIndex, arg1 : SCPValues) -> Result<SCPOptValue, ::rpc_driver::ProgramError>;
        // notifications


    async fn scp_handle_value_externalized(&self, client: &::std::sync::Arc<::rpc_driver::ClientHandle>, arg0 : SCPSlotIndex, arg1 : SCPValue) -> Result<(), ::rpc_driver::ProgramError>;
    
    async fn scp_handle_nominating_value(&self, client: &::std::sync::Arc<::rpc_driver::ClientHandle>, arg0 : SCPSlotIndex, arg1 : SCPValue) -> Result<(), ::rpc_driver::ProgramError>;
    
    async fn scp_handle_updated_candidate_value(&self, client: &::std::sync::Arc<::rpc_driver::ClientHandle>, arg0 : SCPSlotIndex, arg1 : SCPValue) -> Result<(), ::rpc_driver::ProgramError>;
    
    async fn scp_handle_started_ballot_proto(&self, client: &::std::sync::Arc<::rpc_driver::ClientHandle>, arg0 : SCPSlotIndex, arg1 : SCPBallot) -> Result<(), ::rpc_driver::ProgramError>;
    
    async fn scp_handle_accepted_ballot_prepared(&self, client: &::std::sync::Arc<::rpc_driver::ClientHandle>, arg0 : SCPSlotIndex, arg1 : SCPBallot) -> Result<(), ::rpc_driver::ProgramError>;
    
    async fn scp_handle_confirmed_ballot_prepared(&self, client: &::std::sync::Arc<::rpc_driver::ClientHandle>, arg0 : SCPSlotIndex, arg1 : SCPBallot) -> Result<(), ::rpc_driver::ProgramError>;
    
    async fn scp_handle_accepted_commit(&self, client: &::std::sync::Arc<::rpc_driver::ClientHandle>, arg0 : SCPSlotIndex, arg1 : SCPBallot) -> Result<(), ::rpc_driver::ProgramError>;
    
    async fn scp_handle_ballot_did_hear_from_quorum(&self, client: &::std::sync::Arc<::rpc_driver::ClientHandle>, arg0 : SCPSlotIndex, arg1 : SCPBallot) -> Result<(), ::rpc_driver::ProgramError>;
        // utils

    async fn scp_handle_get_quorum_set(&self, client: &::std::sync::Arc<::rpc_driver::ClientHandle>, arg0 : SCPHash) -> Result<SCPOptQuorumSet, ::rpc_driver::ProgramError>;
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
impl ::rpc_driver::ServerDispatch for SCP_Handle_SCP_Handle1_Dispatcher {
    fn prog_vers(&self) -> ::rpc_driver::ProgVers {
        ::rpc_driver::ProgVers::new(0x9aa92002, 1)
    }
    async fn dispatch(&self, proc: u32, data: &[u8], client: &::std::sync::Arc<::rpc_driver::ClientHandle>) -> Result<Vec<u8>, ::rpc_driver::RpcError> {
        match proc {
            // scp_handle_null
            0 => {
                let args: () =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_null(client).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_emit_envelope
            10 => {
                let args: (SCPEnvelope, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_emit_envelope(client, args.0).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_validate_ballot_value
            20 => {
                let args: (SCPSlotIndex, SCPValue, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_validate_ballot_value(client, args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_validate_nomination_value
            30 => {
                let args: (SCPSlotIndex, SCPValue, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_validate_nomination_value(client, args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_extract_valid_value
            40 => {
                let args: (SCPSlotIndex, SCPValue, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_extract_valid_value(client, args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_combine_candidates
            50 => {
                let args: (SCPSlotIndex, SCPValues, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_combine_candidates(client, args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_value_externalized
            100 => {
                let args: (SCPSlotIndex, SCPValue, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_value_externalized(client, args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_nominating_value
            200 => {
                let args: (SCPSlotIndex, SCPValue, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_nominating_value(client, args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_updated_candidate_value
            210 => {
                let args: (SCPSlotIndex, SCPValue, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_updated_candidate_value(client, args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_started_ballot_proto
            310 => {
                let args: (SCPSlotIndex, SCPBallot, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_started_ballot_proto(client, args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_accepted_ballot_prepared
            320 => {
                let args: (SCPSlotIndex, SCPBallot, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_accepted_ballot_prepared(client, args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_confirmed_ballot_prepared
            330 => {
                let args: (SCPSlotIndex, SCPBallot, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_confirmed_ballot_prepared(client, args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_accepted_commit
            340 => {
                let args: (SCPSlotIndex, SCPBallot, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_accepted_commit(client, args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_ballot_did_hear_from_quorum
            350 => {
                let args: (SCPSlotIndex, SCPBallot, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_ballot_did_hear_from_quorum(client, args.0, args.1).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_handle_get_quorum_set
            1000 => {
                let args: (SCPHash, ) =
                    xdrs::txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_handle_get_quorum_set(client, args.0).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            _ => Err(::rpc_driver::RpcError::InvalidProcedure),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SCP_Handle_SCP_Handle1_Client {
    handle: ::std::sync::Arc<::rpc_driver::ClientHandle>,
}

impl SCP_Handle_SCP_Handle1_Client {
    pub fn new(handle: ::std::sync::Arc<::rpc_driver::ClientHandle>) -> Self {
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


