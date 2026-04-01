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
use ::serde_arrays;



#[derive(Serialize, Deserialize)]
#[derive(ReadXDR, WriteXDR)]

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(C)]
pub struct SCP_uint128_xdr {
    pub highbits: u64,
    pub lowbits: u64,
}

#[derive(Serialize, Deserialize)]
#[derive(ReadXDR, WriteXDR)]

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(C)]
pub struct SCP_int128_xdr {
    pub highbits: i64,
    pub lowbits: u64,
}

// ed25519 sig len is 512 bits
// 
// ed25519 key len is 256 bits
#[derive(Serialize, Deserialize)]
#[derive(ReadXDR, WriteXDR)]

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(Copy)]
#[repr(transparent)]
#[serde(transparent)]
pub struct SCPEd25519Signature {
    #[serde(with = "serde_arrays::hex")]
    inner: [u8; 64 as usize],
}
impl From<[u8; 64 as usize]> for SCPEd25519Signature {
    fn from(_inner: [u8; 64 as usize]) -> Self {
        Self { inner: _inner }
    }
}
impl SCPEd25519Signature {
    pub fn into_inner(self) -> [u8; 64 as usize] {
        self.inner
    }
    pub const fn new(_inner: [u8; 64 as usize]) -> Self {
        Self { inner: _inner }
    }
}

impl Deref for SCPEd25519Signature {
    type Target = [u8; 64 as usize];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl DerefMut for SCPEd25519Signature {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
impl AsRef<[u8]> for SCPEd25519Signature {
    fn as_ref(&self) -> &[u8] {
        self.deref().as_slice()
    }
}
impl<'a> From<&'a [u8; 64]> for &'a SCPEd25519Signature {
    fn from(arr: &[u8; 64]) -> Self {
        let ptr = arr as *const [u8; 64] as *const SCPEd25519Signature;
        unsafe { &*ptr }
    }
}
impl<'r> ReadXDR<'r> for &'r SCPEd25519Signature {
  fn read_xdr<R: xdrs::Reader<'r>>(r: &mut R) -> xdrs::ReadResult<Self> {
      let arr: &[u8; 64] = xdrs::read_xdr(r)?;
      let val = Self::from(arr);
      Ok(val)
  }
}



// 
// ed25519 secret key len is 64 bytes
#[derive(Serialize, Deserialize)]
#[derive(ReadXDR, WriteXDR)]

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(Default, Copy)]
#[repr(transparent)]
#[serde(transparent)]
pub struct SCPEd25519PublicKey {
    #[serde(with = "serde_arrays::hex")]
    inner: [u8; 32 as usize],
}
impl From<[u8; 32 as usize]> for SCPEd25519PublicKey {
    fn from(_inner: [u8; 32 as usize]) -> Self {
        Self { inner: _inner }
    }
}
impl SCPEd25519PublicKey {
    pub fn into_inner(self) -> [u8; 32 as usize] {
        self.inner
    }
    pub const fn new(_inner: [u8; 32 as usize]) -> Self {
        Self { inner: _inner }
    }
}

impl Deref for SCPEd25519PublicKey {
    type Target = [u8; 32 as usize];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl DerefMut for SCPEd25519PublicKey {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
impl AsRef<[u8]> for SCPEd25519PublicKey {
    fn as_ref(&self) -> &[u8] {
        self.deref().as_slice()
    }
}
impl<'a> From<&'a [u8; 32]> for &'a SCPEd25519PublicKey {
    fn from(arr: &[u8; 32]) -> Self {
        let ptr = arr as *const [u8; 32] as *const SCPEd25519PublicKey;
        unsafe { &*ptr }
    }
}
impl<'r> ReadXDR<'r> for &'r SCPEd25519PublicKey {
  fn read_xdr<R: xdrs::Reader<'r>>(r: &mut R) -> xdrs::ReadResult<Self> {
      let arr: &[u8; 32] = xdrs::read_xdr(r)?;
      let val = Self::from(arr);
      Ok(val)
  }
}



// 
#[derive(Serialize, Deserialize)]
#[derive(ReadXDR, WriteXDR)]

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(Copy)]
#[repr(transparent)]
#[serde(transparent)]
pub struct SCPEd25519SecretKey {
    #[serde(with = "serde_arrays::hex")]
    inner: [u8; 64 as usize],
}
impl From<[u8; 64 as usize]> for SCPEd25519SecretKey {
    fn from(_inner: [u8; 64 as usize]) -> Self {
        Self { inner: _inner }
    }
}
impl SCPEd25519SecretKey {
    pub fn into_inner(self) -> [u8; 64 as usize] {
        self.inner
    }
    pub const fn new(_inner: [u8; 64 as usize]) -> Self {
        Self { inner: _inner }
    }
}

impl Deref for SCPEd25519SecretKey {
    type Target = [u8; 64 as usize];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl DerefMut for SCPEd25519SecretKey {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
impl AsRef<[u8]> for SCPEd25519SecretKey {
    fn as_ref(&self) -> &[u8] {
        self.deref().as_slice()
    }
}
impl<'a> From<&'a [u8; 64]> for &'a SCPEd25519SecretKey {
    fn from(arr: &[u8; 64]) -> Self {
        let ptr = arr as *const [u8; 64] as *const SCPEd25519SecretKey;
        unsafe { &*ptr }
    }
}
impl<'r> ReadXDR<'r> for &'r SCPEd25519SecretKey {
  fn read_xdr<R: xdrs::Reader<'r>>(r: &mut R) -> xdrs::ReadResult<Self> {
      let arr: &[u8; 64] = xdrs::read_xdr(r)?;
      let val = Self::from(arr);
      Ok(val)
  }
}



#[derive(Serialize, Deserialize)]
#[serde(transparent)]
#[derive(ReadXDR, WriteXDR)]

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(Copy)]
#[repr(transparent)]
pub struct SCPCryptographicScheme(pub i32);

impl SCPCryptographicScheme {
    pub const Ed25519: Self = Self(0);
}
impl From<i32> for SCPCryptographicScheme {
    fn from(v: i32) -> Self {
        Self(v)
    }
}
#[cfg(feature = "std")]
impl core::fmt::Display for SCPCryptographicScheme {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0 {
            x if x == Self::Ed25519.0 => {
                write!(f, "SCPCryptographicScheme::Ed25519")
            },
            _ => {
                write!(f, "SCPCryptographicScheme({})", self.0)
            }
        }
    }
}


#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(Serialize, Deserialize)]
#[derive(ReadXDR, WriteXDR)]

#[repr(i32)]
#[non_exhaustive]
pub enum SCPSignature {
    // case Ed25519:
    //   SCPEd25519Signature ed25519;
    Ed25519 { ed25519: SCPEd25519Signature } = SCPCryptographicScheme::Ed25519.0 as i32,
    
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(Serialize, Deserialize)]
#[derive(ReadXDR, WriteXDR)]

#[repr(i32)]
#[non_exhaustive]
pub enum SCPPublicKey {
    // case Ed25519:
    //   SCPEd25519PublicKey ed25519;
    Ed25519 { ed25519: SCPEd25519PublicKey } = SCPCryptographicScheme::Ed25519.0 as i32,
    
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(Serialize, Deserialize)]
#[derive(ReadXDR, WriteXDR)]

#[repr(i32)]
#[non_exhaustive]
pub enum SCPSecretKey {
    // case Ed25519:
    //   SCPEd25519SecretKey ed25519;
    Ed25519 { ed25519: SCPEd25519SecretKey } = SCPCryptographicScheme::Ed25519.0 as i32,
    
}

// 256 bit hash
// 
#[derive(Serialize, Deserialize)]
#[derive(ReadXDR, WriteXDR)]

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(Default, Copy)]
#[repr(transparent)]
#[serde(transparent)]
pub struct SCPSHA256Hash {
    #[serde(with = "serde_arrays::hex")]
    inner: [u8; 32 as usize],
}
impl From<[u8; 32 as usize]> for SCPSHA256Hash {
    fn from(_inner: [u8; 32 as usize]) -> Self {
        Self { inner: _inner }
    }
}
impl SCPSHA256Hash {
    pub fn into_inner(self) -> [u8; 32 as usize] {
        self.inner
    }
    pub const fn new(_inner: [u8; 32 as usize]) -> Self {
        Self { inner: _inner }
    }
}

impl Deref for SCPSHA256Hash {
    type Target = [u8; 32 as usize];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl DerefMut for SCPSHA256Hash {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
impl AsRef<[u8]> for SCPSHA256Hash {
    fn as_ref(&self) -> &[u8] {
        self.deref().as_slice()
    }
}
impl<'a> From<&'a [u8; 32]> for &'a SCPSHA256Hash {
    fn from(arr: &[u8; 32]) -> Self {
        let ptr = arr as *const [u8; 32] as *const SCPSHA256Hash;
        unsafe { &*ptr }
    }
}
impl<'r> ReadXDR<'r> for &'r SCPSHA256Hash {
  fn read_xdr<R: xdrs::Reader<'r>>(r: &mut R) -> xdrs::ReadResult<Self> {
      let arr: &[u8; 32] = xdrs::read_xdr(r)?;
      let val = Self::from(arr);
      Ok(val)
  }
}




//
// XDR internals below this point
//

impl SCPSignature {
    pub fn scheme(&self) -> SCPCryptographicScheme {
        match self {
            
            _ => (unsafe { *(self as *const Self as *const i32) }).into(),
        }
    }
}

impl SCPPublicKey {
    pub fn scheme(&self) -> SCPCryptographicScheme {
        match self {
            
            _ => (unsafe { *(self as *const Self as *const i32) }).into(),
        }
    }
}

impl SCPSecretKey {
    pub fn scheme(&self) -> SCPCryptographicScheme {
        match self {
            
            _ => (unsafe { *(self as *const Self as *const i32) }).into(),
        }
    }
}


