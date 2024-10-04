import { GetNodeInfoResponse } from "cosmjs-types/cosmos/base/tendermint/v1beta1/query.js";
import { ApiService } from "./service";
import { IConfig } from "config";
import Long from "long";
import { ChainService } from "./chain";

export class NodeInfoService implements ApiService {
	config: IConfig;
	chainService: ChainService;

	constructor(config: IConfig, chainService: ChainService) {
		this.config = config;
		this.chainService = chainService;
	}

	public async nodeInfo(): Promise<GetNodeInfoResponse> {
		console.debug('nodeInfo');

		const chainApi = await this.chainService.getChainApi();
		const { chain_id, name, bech32_prefix, version } = (await chainApi.rpc['cosmos']['chainInfo']()).toJSON();
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
