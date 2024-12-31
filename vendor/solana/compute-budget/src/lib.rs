//! Solana compute budget types and default configurations.
#![allow(unexpected_cfgs)]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(RUSTC_WITH_SPECIALIZATION, feature(min_specialization))]

pub mod compute_budget;
pub mod compute_budget_processor;
pub mod prioritization_fee;
