#![cfg_attr(not(feature = "std"), no_std)]
#![allow(incomplete_features)]
#![cfg_attr(RUSTC_WITH_SPECIALIZATION, feature(specialization))]

#[cfg(not(target_os = "solana"))]
pub mod processor;
