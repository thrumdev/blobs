#![cfg_attr(not(feature = "native"), no_std)]

extern crate alloc;

#[cfg(feature = "native")]
pub mod service;
pub mod spec;
pub mod types;
pub mod verifier;
