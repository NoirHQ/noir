export interface ResultStatus {
	node_info: StatusDefaultNodeInfo;
	sync_info: SyncInfo;
	validator_info: ValidatorInfo;
}

export interface DefaultNodeInfoOther {
	tx_index: string;
	rpc_address: string;
}

export interface ProtocolVersion {
	p2p: string;
	block: string;
	app: string;
}

export interface StatusDefaultNodeInfo {
	protocol_version: ProtocolVersion;
	id: string;
	listen_addr: string;
	network: string;
	version: string;
	channels: string;
	moniker: string;
	other: DefaultNodeInfoOther;
}

export interface SyncInfo {
	latest_block_hash: string;
	latest_app_hash: string;
	latest_block_height: string;
	latest_block_time: string;
	catching_up: boolean;
}

export interface ValidatorInfo {
	address: string;
	pub_key: PubKey;
	voting_power: string;
}

export interface PubKey {
	type: string;
	value: string;
}
