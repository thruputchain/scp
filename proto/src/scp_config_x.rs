// -*-Rust-*-
// Generated Code: DO NOT CHECK IN

// Generated from xdrc commit: 73ac135b6b8a14396074dde8ea20f0e6b671175c

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
use ::xdrs::{ReadXDR, WriteXDR};



#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[derive()]
#[cfg_attr(feature = "xdrs", derive(ReadXDR, WriteXDR))]
#[repr(C)]
pub struct SCPConfig {
    pub max_timeout_ms: u64,
    pub initial_ballot_timeout_ms: u64,
    pub increment_ballot_timeout_ms: u64,
    pub initial_nomination_timeout_ms: u64,
    pub increment_nomination_timeout_ms: u64,
}


//
// XDR internals below this point
//


