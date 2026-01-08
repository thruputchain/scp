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


#[cfg(feature = "serde")]
use ::serde::{Serialize, Deserialize};
#[cfg(feature = "serde")]
use ::serde_arrays;



#[cfg_attr(feature = "xdrs", derive(ReadXDR, WriteXDR))]
#[cfg_attr(feature = "spec", derive(XDR))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(C)]
pub struct uint128_xdr {
    pub highbits: u64,
    pub lowbits: u64,
}

#[cfg_attr(feature = "xdrs", derive(ReadXDR, WriteXDR))]
#[cfg_attr(feature = "spec", derive(XDR))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(C)]
pub struct int128_xdr {
    pub highbits: i64,
    pub lowbits: u64,
}

// ed25519 sig len is 512 bits
// 
// ed25519 key len is 256 bits
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "xdrs", derive(ReadXDR, WriteXDR))]
#[cfg_attr(feature = "spec", derive(XDR))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(Copy)]
#[repr(transparent)]
pub struct Ed25519Signature {
    #[cfg_attr(feature = "serde", serde(with = "serde_arrays"))]
    inner: [u8; 64 as usize],
}
impl From<[u8; 64 as usize]> for Ed25519Signature {
    fn from(_inner: [u8; 64 as usize]) -> Self {
        Self { inner: _inner }
    }
}
impl Ed25519Signature {
    pub fn into_inner(self) -> [u8; 64 as usize] {
        self.inner
    }
    pub const fn new(_inner: [u8; 64 as usize]) -> Self {
        Self { inner: _inner }
    }
}

impl Deref for Ed25519Signature {
    type Target = [u8; 64 as usize];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl DerefMut for Ed25519Signature {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
impl AsRef<[u8]> for Ed25519Signature {
    fn as_ref(&self) -> &[u8] {
        self.deref().as_slice()
    }
}
impl<'a> From<&'a [u8; 64]> for &'a Ed25519Signature {
    fn from(arr: &[u8; 64]) -> Self {
        let ptr = arr as *const [u8; 64] as *const Ed25519Signature;
        unsafe { &*ptr }
    }
}
#[cfg(feature = "xdrs")]
impl<'r> ReadXDR<'r> for &'r Ed25519Signature {
    fn read_xdr<R: xdrs::Reader<'r>>(r: &mut R) -> xdrs::ReadResult<Self> {
        let arr: &[u8; 64] = xdrs::read_xdr(r)?;
        let val = Self::from(arr);
        Ok(val)
    }
}


// 
// ed25519 secret key len is 64 bytes
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "xdrs", derive(ReadXDR, WriteXDR))]
#[cfg_attr(feature = "spec", derive(XDR))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(Default, Copy)]
#[repr(transparent)]
pub struct Ed25519PublicKey {
    #[cfg_attr(feature = "serde", serde(with = "serde_arrays"))]
    inner: [u8; 32 as usize],
}
impl From<[u8; 32 as usize]> for Ed25519PublicKey {
    fn from(_inner: [u8; 32 as usize]) -> Self {
        Self { inner: _inner }
    }
}
impl Ed25519PublicKey {
    pub fn into_inner(self) -> [u8; 32 as usize] {
        self.inner
    }
    pub const fn new(_inner: [u8; 32 as usize]) -> Self {
        Self { inner: _inner }
    }
}

impl Deref for Ed25519PublicKey {
    type Target = [u8; 32 as usize];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl DerefMut for Ed25519PublicKey {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
impl AsRef<[u8]> for Ed25519PublicKey {
    fn as_ref(&self) -> &[u8] {
        self.deref().as_slice()
    }
}
impl<'a> From<&'a [u8; 32]> for &'a Ed25519PublicKey {
    fn from(arr: &[u8; 32]) -> Self {
        let ptr = arr as *const [u8; 32] as *const Ed25519PublicKey;
        unsafe { &*ptr }
    }
}
#[cfg(feature = "xdrs")]
impl<'r> ReadXDR<'r> for &'r Ed25519PublicKey {
    fn read_xdr<R: xdrs::Reader<'r>>(r: &mut R) -> xdrs::ReadResult<Self> {
        let arr: &[u8; 32] = xdrs::read_xdr(r)?;
        let val = Self::from(arr);
        Ok(val)
    }
}


// 
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "xdrs", derive(ReadXDR, WriteXDR))]
#[cfg_attr(feature = "spec", derive(XDR))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(Copy)]
#[repr(transparent)]
pub struct Ed25519SecretKey {
    #[cfg_attr(feature = "serde", serde(with = "serde_arrays"))]
    inner: [u8; 64 as usize],
}
impl From<[u8; 64 as usize]> for Ed25519SecretKey {
    fn from(_inner: [u8; 64 as usize]) -> Self {
        Self { inner: _inner }
    }
}
impl Ed25519SecretKey {
    pub fn into_inner(self) -> [u8; 64 as usize] {
        self.inner
    }
    pub const fn new(_inner: [u8; 64 as usize]) -> Self {
        Self { inner: _inner }
    }
}

impl Deref for Ed25519SecretKey {
    type Target = [u8; 64 as usize];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl DerefMut for Ed25519SecretKey {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
impl AsRef<[u8]> for Ed25519SecretKey {
    fn as_ref(&self) -> &[u8] {
        self.deref().as_slice()
    }
}
impl<'a> From<&'a [u8; 64]> for &'a Ed25519SecretKey {
    fn from(arr: &[u8; 64]) -> Self {
        let ptr = arr as *const [u8; 64] as *const Ed25519SecretKey;
        unsafe { &*ptr }
    }
}
#[cfg(feature = "xdrs")]
impl<'r> ReadXDR<'r> for &'r Ed25519SecretKey {
    fn read_xdr<R: xdrs::Reader<'r>>(r: &mut R) -> xdrs::ReadResult<Self> {
        let arr: &[u8; 64] = xdrs::read_xdr(r)?;
        let val = Self::from(arr);
        Ok(val)
    }
}


#[cfg_attr(feature = "xdrs", derive(ReadXDR, WriteXDR))]
#[cfg_attr(feature = "spec", derive(XDR))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(Copy)]
#[repr(i32)]
pub enum CryptographicScheme {
    Ed25519 = 0 as i32,
}
pub use CryptographicScheme::*;

impl TryFrom<i32> for CryptographicScheme {
    type Error = ();
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Ed25519),
            _ => Err(()),
        }
    }
}


#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "xdrs", derive(ReadXDR, WriteXDR))]
#[cfg_attr(feature = "spec", derive(XDR))]
#[repr(i32)]
pub enum Signature {
    // case Ed25519:
    //   Ed25519Signature ed25519;
    Ed25519 { ed25519: Ed25519Signature } = CryptographicScheme::Ed25519 as i32,
    
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "xdrs", derive(ReadXDR, WriteXDR))]
#[cfg_attr(feature = "spec", derive(XDR))]
#[repr(i32)]
pub enum PublicKey {
    // case Ed25519:
    //   Ed25519PublicKey ed25519;
    Ed25519 { ed25519: Ed25519PublicKey } = CryptographicScheme::Ed25519 as i32,
    
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "xdrs", derive(ReadXDR, WriteXDR))]
#[cfg_attr(feature = "spec", derive(XDR))]
#[repr(i32)]
pub enum SecretKey {
    // case Ed25519:
    //   Ed25519SecretKey ed25519;
    Ed25519 { ed25519: Ed25519SecretKey } = CryptographicScheme::Ed25519 as i32,
    
}

// 256 bit hash
// 
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "xdrs", derive(ReadXDR, WriteXDR))]
#[cfg_attr(feature = "spec", derive(XDR))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(Default, Copy)]
#[repr(transparent)]
pub struct SHA256Hash {
    #[cfg_attr(feature = "serde", serde(with = "serde_arrays"))]
    inner: [u8; 32 as usize],
}
impl From<[u8; 32 as usize]> for SHA256Hash {
    fn from(_inner: [u8; 32 as usize]) -> Self {
        Self { inner: _inner }
    }
}
impl SHA256Hash {
    pub fn into_inner(self) -> [u8; 32 as usize] {
        self.inner
    }
    pub const fn new(_inner: [u8; 32 as usize]) -> Self {
        Self { inner: _inner }
    }
}

impl Deref for SHA256Hash {
    type Target = [u8; 32 as usize];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl DerefMut for SHA256Hash {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
impl AsRef<[u8]> for SHA256Hash {
    fn as_ref(&self) -> &[u8] {
        self.deref().as_slice()
    }
}
impl<'a> From<&'a [u8; 32]> for &'a SHA256Hash {
    fn from(arr: &[u8; 32]) -> Self {
        let ptr = arr as *const [u8; 32] as *const SHA256Hash;
        unsafe { &*ptr }
    }
}
#[cfg(feature = "xdrs")]
impl<'r> ReadXDR<'r> for &'r SHA256Hash {
    fn read_xdr<R: xdrs::Reader<'r>>(r: &mut R) -> xdrs::ReadResult<Self> {
        let arr: &[u8; 32] = xdrs::read_xdr(r)?;
        let val = Self::from(arr);
        Ok(val)
    }
}



//
// XDR internals below this point
//

impl Signature {
    pub fn scheme(&self) -> i32 {
        match self {
            
            _ => unsafe { *(self as *const Self as *const i32) },
        }
    }
}

impl PublicKey {
    pub fn scheme(&self) -> i32 {
        match self {
            
            _ => unsafe { *(self as *const Self as *const i32) },
        }
    }
}

impl SecretKey {
    pub fn scheme(&self) -> i32 {
        match self {
            
            _ => unsafe { *(self as *const Self as *const i32) },
        }
    }
}


