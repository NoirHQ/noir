#![allow(unexpected_cfgs)]
#![allow(clippy::too_many_arguments)]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(RUSTC_WITH_SPECIALIZATION, feature(min_specialization))]
#![deny(clippy::arithmetic_side_effects)]
#![deny(clippy::indexing_slicing)]

#[macro_use]
extern crate eager;

#[cfg(feature = "std")]
#[macro_use]
extern crate solana_metrics;

pub use solana_rbpf;
pub mod dummy;
pub mod invoke_context;
pub mod loaded_programs;
pub mod log_collector;
pub mod mem_pool;
pub mod stable_log;
pub mod sysvar_cache;
pub mod timings;
