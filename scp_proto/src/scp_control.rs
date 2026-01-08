// -*-Rust-*-
// Generated Code: DO NOT CHECK IN

// Generated from xdrc commit: e159036f245540d9573617a3662096ea6df08cf8

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unexpected_cfgs)]
#![allow(unused_variables)]
#![allow(clippy::unnecessary_cast)]

use crate::*;
use core::option::Option;
use core::ops::{Deref, DerefMut};

#[cfg(feature="std")]
use std::io::{Read, Write};

#[cfg(feature="xdrs")]
use xdrs::*;
#[cfg(feature="spec")]
use xdrs::xdr_repr::*;





#[allow(non_camel_case_types)]
#[async_trait::async_trait]
pub trait SCP_Control_SCP_Control1: Send + Sync {    
    async fn scp_control_null(&self) -> Result<(), ::rpc_driver::ProgramError>;
        // arguments: slot index, value to nominate, previous value

    async fn scp_control_nominate(&self, arg0 : SCPSlotIndex, arg1 : SCPValue, arg2 : SCPValue) -> Result<bool, ::rpc_driver::ProgramError>;
    
    async fn scp_control_stop_nomination(&self, arg0 : SCPSlotIndex) -> Result<(), ::rpc_driver::ProgramError>;
    
    async fn scp_control_update_local_quorum_set(&self, arg0 : SCPQuorumSet) -> Result<(), ::rpc_driver::ProgramError>;
    
    async fn scp_control_received_envelope(&self, arg0 : SCPEnvelope) -> Result<(), ::rpc_driver::ProgramError>;
    
    async fn scp_control_set_qset(&self, arg0 : SCPQuorumSet) -> Result<SCPOptQuorumSet, ::rpc_driver::ProgramError>;
    
    async fn scp_control_get_qset(&self, arg0 : SCPHash) -> Result<SCPOptQuorumSet, ::rpc_driver::ProgramError>;
    
    async fn scp_control_remove_qset(&self, arg0 : SCPHash) -> Result<SCPOptQuorumSet, ::rpc_driver::ProgramError>;
}


#[allow(non_camel_case_types)]
pub struct SCP_Control_SCP_Control1_Dispatcher {
  inner: Box<dyn SCP_Control_SCP_Control1>,
}

impl SCP_Control_SCP_Control1_Dispatcher {
  pub fn new(inner: Box<dyn SCP_Control_SCP_Control1>) -> Self {
    Self { inner }
  }
}

#[async_trait::async_trait]
impl ::rpc_driver::ServerDispatch for SCP_Control_SCP_Control1_Dispatcher {
    fn prog_vers(&self) -> ::rpc_driver::ProgVers {
        ::rpc_driver::ProgVers::new(0x9aa93003, 1)
    }
    async fn dispatch(&self, proc: u32, data: &[u8]) -> Result<Vec<u8>, ::rpc_driver::RpcError> {
        match proc {
            // scp_control_null
            0 => {
                let args: () =
                    xdrs::serialize_txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_control_null().await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::serialize_txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_control_nominate
            10 => {
                let args: (SCPSlotIndex, SCPValue, SCPValue, ) =
                    xdrs::serialize_txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_control_nominate(args.0, args.1, args.2).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::serialize_txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_control_stop_nomination
            20 => {
                let args: (SCPSlotIndex, ) =
                    xdrs::serialize_txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_control_stop_nomination(args.0).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::serialize_txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_control_update_local_quorum_set
            100 => {
                let args: (SCPQuorumSet, ) =
                    xdrs::serialize_txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_control_update_local_quorum_set(args.0).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::serialize_txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_control_received_envelope
            200 => {
                let args: (SCPEnvelope, ) =
                    xdrs::serialize_txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_control_received_envelope(args.0).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::serialize_txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_control_set_qset
            300 => {
                let args: (SCPQuorumSet, ) =
                    xdrs::serialize_txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_control_set_qset(args.0).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::serialize_txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_control_get_qset
            301 => {
                let args: (SCPHash, ) =
                    xdrs::serialize_txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_control_get_qset(args.0).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::serialize_txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            // scp_control_remove_qset
            302 => {
                let args: (SCPHash, ) =
                    xdrs::serialize_txs::decode(data).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
                let res = self.inner.scp_control_remove_qset(args.0).await.map_err(|_| ::rpc_driver::RpcError::SystemError)?;
                xdrs::serialize_txs::encode(&res).map_err(|_| ::rpc_driver::RpcError::SystemError)
            }
            _ => Err(::rpc_driver::RpcError::InvalidProcedure),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SCP_Control_SCP_Control1_Client {
    handle: ::std::sync::Arc<::rpc_driver::ClientHandle>,
}

impl SCP_Control_SCP_Control1_Client {
    pub fn new(handle: ::std::sync::Arc<::rpc_driver::ClientHandle>) -> Self {
        Self { handle }
    }

    fn prog_vers() -> ::rpc_driver::ProgVers {
        ::rpc_driver::ProgVers::new(0x9aa93003, 1)
    }
    pub async fn scp_control_null(&self) -> Result<(), ::rpc_driver::RpcError> {
         let data = xdrs::serialize_txs::encode(&()).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
         let buf = self.handle.send_call(Self::prog_vers(), 0, &data).await?;
         xdrs::serialize_txs::decode(&buf).map_err(|_| ::rpc_driver::RpcError::SystemError)
    }
    pub async fn scp_control_nominate(&self, arg0 : &SCPSlotIndex, arg1 : &SCPValue, arg2 : &SCPValue) -> Result<bool, ::rpc_driver::RpcError> {
         let data = xdrs::serialize_txs::encode(&(&arg0, &arg1, &arg2)).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
         let buf = self.handle.send_call(Self::prog_vers(), 10, &data).await?;
         xdrs::serialize_txs::decode(&buf).map_err(|_| ::rpc_driver::RpcError::SystemError)
    }
    pub async fn scp_control_stop_nomination(&self, arg0 : &SCPSlotIndex) -> Result<(), ::rpc_driver::RpcError> {
         let data = xdrs::serialize_txs::encode(&(&arg0)).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
         let buf = self.handle.send_call(Self::prog_vers(), 20, &data).await?;
         xdrs::serialize_txs::decode(&buf).map_err(|_| ::rpc_driver::RpcError::SystemError)
    }
    pub async fn scp_control_update_local_quorum_set(&self, arg0 : &SCPQuorumSet) -> Result<(), ::rpc_driver::RpcError> {
         let data = xdrs::serialize_txs::encode(&(&arg0)).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
         let buf = self.handle.send_call(Self::prog_vers(), 100, &data).await?;
         xdrs::serialize_txs::decode(&buf).map_err(|_| ::rpc_driver::RpcError::SystemError)
    }
    pub async fn scp_control_received_envelope(&self, arg0 : &SCPEnvelope) -> Result<(), ::rpc_driver::RpcError> {
         let data = xdrs::serialize_txs::encode(&(&arg0)).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
         let buf = self.handle.send_call(Self::prog_vers(), 200, &data).await?;
         xdrs::serialize_txs::decode(&buf).map_err(|_| ::rpc_driver::RpcError::SystemError)
    }
    pub async fn scp_control_set_qset(&self, arg0 : &SCPQuorumSet) -> Result<SCPOptQuorumSet, ::rpc_driver::RpcError> {
         let data = xdrs::serialize_txs::encode(&(&arg0)).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
         let buf = self.handle.send_call(Self::prog_vers(), 300, &data).await?;
         xdrs::serialize_txs::decode(&buf).map_err(|_| ::rpc_driver::RpcError::SystemError)
    }
    pub async fn scp_control_get_qset(&self, arg0 : &SCPHash) -> Result<SCPOptQuorumSet, ::rpc_driver::RpcError> {
         let data = xdrs::serialize_txs::encode(&(&arg0)).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
         let buf = self.handle.send_call(Self::prog_vers(), 301, &data).await?;
         xdrs::serialize_txs::decode(&buf).map_err(|_| ::rpc_driver::RpcError::SystemError)
    }
    pub async fn scp_control_remove_qset(&self, arg0 : &SCPHash) -> Result<SCPOptQuorumSet, ::rpc_driver::RpcError> {
         let data = xdrs::serialize_txs::encode(&(&arg0)).map_err(|_| ::rpc_driver::RpcError::GarbageArguments)?;
         let buf = self.handle.send_call(Self::prog_vers(), 302, &data).await?;
         xdrs::serialize_txs::decode(&buf).map_err(|_| ::rpc_driver::RpcError::SystemError)
    }
}



//
// XDR internals below this point
//


