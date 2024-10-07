export const types = {
	GasInfo: {
		gas_used: 'u64',
		gas_wanted: 'u64',
	},
	EventAttribute: {
		key: 'Vec<u8>',
		value: 'Vec<u8>',
	},
	CosmosEvent: {
		type: 'Vec<u8>',
		attributes: 'Vec<EventAttribute>',

	},
	SimulateResponse: {
		gas_info: 'GasInfo',
		events: 'Vec<CosmosEvent>',
	},
	ChainInfo: {
		chain_id: 'String',
		name: 'String',
		bech32_prefix: 'String',
		version: 'String',
	}
};
