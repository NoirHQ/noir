use crate::{
	prelude::*,
	runtimes::vm::CosmwasmVMError,
	types::{AccountIdOf, DefaultCosmwasmVM},
	Config, Pallet,
};

impl<T: Config> Pallet<T> {
	/// Check whether a contract export the mandatory IBC functions and is consequently IBC capable.
	pub(crate) fn do_check_ibc_capability(_module: &parity_wasm::elements::Module) -> bool {
		false
	}

	pub fn do_ibc_transfer(
		_vm: &mut DefaultCosmwasmVM<T>,
		_channel_id: String,
		_to_address: String,
		_amount: cosmwasm_std::Coin,
		_timeout: cosmwasm_std::IbcTimeout,
	) -> Result<(), CosmwasmVMError<T>> {
		Err(CosmwasmVMError::<T>::Unsupported)
	}

	pub(crate) fn do_ibc_send_packet(
		_vm: &mut DefaultCosmwasmVM<T>,
		_channel_id: String,
		_data: cosmwasm_std::Binary,
		_timeout: cosmwasm_std::IbcTimeout,
	) -> Result<(), CosmwasmVMError<T>> {
		Err(CosmwasmVMError::<T>::Unsupported)
	}

	pub(crate) fn do_ibc_close_channel(
		_vm: &mut DefaultCosmwasmVM<T>,
		_channel_id: String,
	) -> Result<(), CosmwasmVMError<T>> {
		Err(CosmwasmVMError::<T>::Unsupported)
	}

	pub(crate) fn do_compute_ibc_contract_port(address: AccountIdOf<T>) -> String {
		format!("wasm.{}", Pallet::<T>::account_to_cosmwasm_addr(address))
	}
}
