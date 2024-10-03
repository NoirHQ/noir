import { GetNodeInfoResponse } from "cosmjs-types/cosmos/base/tendermint/v1beta1/query.js";
import { ApiService } from "./service";
import { IConfig } from "config";
import Long from "long";
import { ApiPromise } from "@polkadot/api";

export class NodeInfoService implements ApiService {
	config: IConfig;
	chainApi: ApiPromise;

	constructor(config: IConfig, chainApi: ApiPromise) {
		this.config = config;
		this.chainApi = chainApi;
	}

	public async nodeInfo(): Promise<GetNodeInfoResponse> {
		const { chain_id, name, bech32_prefix, version } = (await this.chainApi.rpc['cosmos']['chainInfo']()).toJSON();
		const endpoint = this.config.get<string>('server.endpoint');

		return {
			defaultNodeInfo: {
				protocolVersion: {
					p2p: Long.ZERO,
					block: Long.ZERO,
					app: Long.ZERO,
				},
				defaultNodeId: '0000000000000000000000000000000000000000',
				listenAddr: endpoint,
				network: chain_id,
				version,
				channels: new Uint8Array(Buffer.allocUnsafe(8)),
				moniker: bech32_prefix,
				other: {
					txIndex: 'off',
					rpcAddress: '',
				},
			},
			applicationVersion: {
				name,
				appName: name,
				version,
				gitCommit: '0000000000000000000000000000000000000000',
				buildTags: '',
				goVersion: '0',
				buildDeps: [],
				cosmosSdkVersion: '0',
			},
		};
	}
}
