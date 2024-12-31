#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use {alloc::boxed::Box, solana_program_runtime::declare_process_instruction};

pub const DEFAULT_COMPUTE_UNITS: u64 = 150;

declare_process_instruction!(Entrypoint, DEFAULT_COMPUTE_UNITS, |_invoke_context| {
    // Do nothing, compute budget instructions handled by the runtime
    Ok(())
});
