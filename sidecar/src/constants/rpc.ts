export const rpc = {
	cosmos: {
		broadcastTx: {
			description: 'Broadcast cosmos transaction.',
			params: [
				{
					name: 'tx_bytes',
					type: 'Bytes',
				},
			],
			type: 'H256',
		},
		simulate: {
			description: 'Simulate cosmos transaction.',
			params: [
				{
					name: 'tx_bytes',
					type: 'Bytes',
				},
				{
					name: 'at',
					type: 'Option<Bytes>',
				},
			],
			type: 'SimulateResponse',
		},
		chainInfo: {
			description: 'Get Cosmos chain information.',
			params: [],
			type: 'ChainInfo',
		}
	},
	cosmwasm: {
		query: {
			description: 'Query Cosmwasm state.',
			params: [
				{
					name: 'contract',
					type: 'String',
				},
				{
					name: 'gas',
					type: 'u64',
				},
				{
					name: 'query_request',
					type: 'Bytes',
				},
				{
					name: 'at',
					type: 'Option<BlockHash>',
				},
			],
			type: 'Bytes',
		},
	}
};
