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


#[cfg(not(feature="xdrs"))]
use xdr_common::*;



// clang-format on

pub type SCPHash = [u8; 32 as usize];

pub type SCPNodeID = PublicKey;

pub type SCPCount = u32;

pub type SCPSlotIndex = u64;

pub type SCPValue = BytesM;

pub type SCPValues = VecM::<SCPValue>;

pub type SCPOptValue = Option::<SCPValue>;

#[cfg_attr(feature = "xdrs", derive(ReadXDR, WriteXDR))]
#[cfg_attr(feature = "spec", derive(XDR))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(C)]
pub struct SCPBallot {
    // n
    pub counter: SCPCount,
    // x
    pub value: SCPValue,
}

#[cfg_attr(feature = "xdrs", derive(ReadXDR, WriteXDR))]
#[cfg_attr(feature = "spec", derive(XDR))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(Copy)]
#[repr(i32)]
pub enum SCPStatementType {
    SCP_ST_PREPARE = 0 as i32,
    SCP_ST_CONFIRM = 1 as i32,
    SCP_ST_EXTERNALIZE = 2 as i32,
    SCP_ST_NOMINATE = 3 as i32,
}
pub use SCPStatementType::*;

impl TryFrom<i32> for SCPStatementType {
    type Error = ();
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::SCP_ST_PREPARE),
            1 => Ok(Self::SCP_ST_CONFIRM),
            2 => Ok(Self::SCP_ST_EXTERNALIZE),
            3 => Ok(Self::SCP_ST_NOMINATE),
            _ => Err(()),
        }
    }
}


#[cfg_attr(feature = "xdrs", derive(ReadXDR, WriteXDR))]
#[cfg_attr(feature = "spec", derive(XDR))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(C)]
pub struct SCPStatementNominate {
    // D
    pub quorum_set_hash: SCPHash,
    // X
    pub votes: VecM<SCPValue>,
    // Y
    pub accepted: VecM<SCPValue>,
}

#[cfg_attr(feature = "xdrs", derive(ReadXDR, WriteXDR))]
#[cfg_attr(feature = "spec", derive(XDR))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(C)]
pub struct SCPStatementPrepare {
    // D
    pub quorum_set_hash: SCPHash,
    // b
    pub ballot: SCPBallot,
    // p
    pub prepared: Option<SCPBallot>,
    // p'
    pub prepared_prime: Option<SCPBallot>,
    // c.n
    pub n_c: SCPCount,
    // h.n
    pub n_h: SCPCount,
}

#[cfg_attr(feature = "xdrs", derive(ReadXDR, WriteXDR))]
#[cfg_attr(feature = "spec", derive(XDR))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(C)]
pub struct SCPStatementConfirm {
    // b
    pub ballot: SCPBallot,
    // p.n
    pub n_prepared: SCPCount,
    // c.n
    pub n_commit: SCPCount,
    // h.n
    pub n_h: SCPCount,
    // D
    pub quorum_set_hash: SCPHash,
}

#[cfg_attr(feature = "xdrs", derive(ReadXDR, WriteXDR))]
#[cfg_attr(feature = "spec", derive(XDR))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(C)]
pub struct SCPStatementExternalize {
    // c
    pub commit: SCPBallot,
    // h.n
    pub n_h: SCPCount,
    // D used before EXTERNALIZE
    pub commit_quorum_set_hash: SCPHash,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "xdrs", derive(ReadXDR, WriteXDR))]
#[cfg_attr(feature = "spec", derive(XDR))]
#[repr(i32)]
pub enum SCPStatement_pledges {
    // case SCP_ST_PREPARE:
    //   SCPStatementPrepare prepare;
    SCP_ST_PREPARE { prepare: SCPStatementPrepare } = SCPStatementType::SCP_ST_PREPARE as i32,
    // case SCP_ST_CONFIRM:
    //   SCPStatementConfirm confirm;
    SCP_ST_CONFIRM { confirm: SCPStatementConfirm } = SCPStatementType::SCP_ST_CONFIRM as i32,
    // case SCP_ST_EXTERNALIZE:
    //   SCPStatementExternalize externalize;
    SCP_ST_EXTERNALIZE { externalize: SCPStatementExternalize } = SCPStatementType::SCP_ST_EXTERNALIZE as i32,
    // case SCP_ST_NOMINATE:
    //   SCPStatementNominate nominate;
    SCP_ST_NOMINATE { nominate: SCPStatementNominate } = SCPStatementType::SCP_ST_NOMINATE as i32,
    
}

#[cfg_attr(feature = "xdrs", derive(ReadXDR, WriteXDR))]
#[cfg_attr(feature = "spec", derive(XDR))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(C)]
pub struct SCPStatement {
    // v
    pub node_id: SCPNodeID,
    // i
    pub slot_index: SCPSlotIndex,
    pub pledges: SCPStatement_pledges,
}

#[cfg_attr(feature = "xdrs", derive(ReadXDR, WriteXDR))]
#[cfg_attr(feature = "spec", derive(XDR))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(C)]
pub struct SCPEnvelope {
    pub statement: SCPStatement,
    pub signature: Signature,
}

pub type SCPEnvelopes = VecM::<SCPEnvelope>;

// supports things like: A,B,C,(D,E,F),(G,H,(I,J,K,L))
// only allows 2 levels of nesting
#[cfg_attr(feature = "xdrs", derive(ReadXDR, WriteXDR))]
#[cfg_attr(feature = "spec", derive(XDR))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(C)]
pub struct SCPQuorumSet {
    pub threshold: SCPCount,
    pub validators: VecM<SCPNodeID>,
    pub inner_sets: VecM<SCPQuorumSet>,
}

pub type SCPOptQuorumSet = Option::<SCPQuorumSet>;


//
// XDR internals below this point
//

impl SCPStatement_pledges {
    pub fn stmt_type(&self) -> i32 {
        match self {
            
            _ => unsafe { *(self as *const Self as *const i32) },
        }
    }
}


