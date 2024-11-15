//! Contains a single utility function for deserializing from [bincode].
//!
//! [bincode]: https://docs.rs/bincode

use {crate::instruction::InstructionError, bincode::config};

/// Deserialize with a limit based the maximum amount of data a program can expect to get.
/// This function should be used in place of direct deserialization to help prevent OOM errors
//pub fn limited_deserialize<T>(instruction_data: &[u8], limit: u64) -> Result<T, InstructionError>
//where
//    T: serde::de::DeserializeOwned,
//{
pub fn limited_deserialize<T, const N: usize>(
    instruction_data: &[u8],
) -> Result<T, InstructionError>
where
    T: serde::de::DeserializeOwned,
{
    bincode::serde::decode_borrowed_from_slice(instruction_data, config::legacy().with_limit::<N>())
        .map_err(|_| InstructionError::InvalidInstructionData)
}

#[cfg(test)]
pub mod tests {
    use {super::*, solana_program::system_instruction::SystemInstruction};

    #[test]
    fn test_limited_deserialize_advance_nonce_account() {
        let item = SystemInstruction::AdvanceNonceAccount;
        let serialized = crate::bincode::serialize(&item).unwrap();

        assert_eq!(
            serialized.len(),
            4,
            "`SanitizedMessage::get_durable_nonce()` may need a change"
        );

        assert_eq!(
            limited_deserialize::<SystemInstruction, 4>(&serialized).as_ref(),
            Ok(&item)
        );
        assert!(limited_deserialize::<SystemInstruction, 3>(&serialized).is_err());
    }
}
