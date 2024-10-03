import { ApiPromise } from "@polkadot/api";
import { ResultStatus } from "../types";
import { ApiService } from "./service";
import { IConfig } from "config";
import Long from "long";
import { Block } from "@polkadot/types/interfaces";

export class StatusService implements ApiService {
	config: IConfig;
	chainApi: ApiPromise;

	constructor(config: IConfig, chainApi: ApiPromise) {
		this.config = config;
		this.chainApi = chainApi;
	}

	public async status(): Promise<ResultStatus> {
		const hash = (await this.chainApi.rpc.chain.getFinalizedHead()).toString();
		const { block } = (await this.chainApi.rpc.chain.getBlock(hash)).toJSON();
		const blockNumber = (block as unknown as Block).header.number.toString();
		const timestamp = (
			await (await this.chainApi.at(hash)).query.timestamp.now()
		).toString();
		const blockTime = new Date(parseInt(timestamp)).toISOString();
		const blockHash = hash.startsWith('0x') ? hash.slice(2) : hash;

		const { chain_id, bech32_prefix, version } = (await this.chainApi.rpc['cosmos']['chainInfo']()).toJSON();

		const endpoint = this.config.get<string>('server.endpoint');

		return {
			node_info: {
				protocol_version: {
					p2p: Long.ZERO.toString(),
					block: Long.ZERO.toString(),
					app: Long.ZERO.toString(),
				},
				id: '0000000000000000000000000000000000000000',
				listen_addr: endpoint,
				network: chain_id,
				version,
				channels: '0000000000000000',
				moniker: bech32_prefix,
				other: {
					tx_index: 'off',
					rpc_address: endpoint,
				},
			},
			sync_info: {
				latest_block_hash: blockHash.toUpperCase(),
				latest_app_hash:
					'0000000000000000000000000000000000000000000000000000000000000000',
				latest_block_height: blockNumber,
				latest_block_time: blockTime,
				catching_up: false,
			},
			validator_info: {
				address: '0000000000000000000000000000000000000000',
				pub_key: {
					type: 'tendermint/PubKeyEd25519',
					value: 'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=',
				},
				voting_power: '0',
			},
		};
	}
}
