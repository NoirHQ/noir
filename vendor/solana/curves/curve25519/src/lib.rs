#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::arithmetic_side_effects, clippy::op_ref)]
//! Syscall operations for curve25519

extern crate alloc;

pub mod curve_syscall_traits;
pub mod edwards;
pub mod errors;
pub mod ristretto;
pub mod scalar;
