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




// clang-format on

#[cfg_attr(feature = "xdrs", derive(ReadXDR, WriteXDR))]
#[cfg_attr(feature = "spec", derive(XDR))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(C)]
pub struct SCPConfig {
    pub max_timeout_ms: u64,
    pub initial_ballot_timeout_ms: u64,
    pub increment_ballot_timeout_ms: u64,
    pub initial_nomination_timeout_ms: u64,
    pub increment_nomination_timeout_ms: u64,
    pub quorum_set: SCPQuorumSet,
}


//
// XDR internals below this point
//


