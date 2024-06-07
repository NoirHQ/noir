import { ApiPromise } from "@pinot/api";
import { ResultStatus } from "../types/index.js";
import { ApiService } from "./service.js";
import { IConfig } from "config";
import Long from "long";

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
    const blockNumber = (block as any).header.number;
    const timestamp = (
      await (await this.chainApi.at(hash)).query.timestamp.now()
    ).toString();
    const blockTime = new Date(parseInt(timestamp)).toISOString();
    let blockHash = hash;
    if (blockHash.startsWith("0x")) {
      blockHash = blockHash.substring(2);
    }

    const endpoint = this.config.get<string>("server.endpoint");
    const network = this.config.get<string>("chain.network");
    const version = this.config.get<string>("chain.version");
    const moniker = this.config.get<string>("chain.moniker");
    return {
      node_info: {
        protocol_version: {
          p2p: Long.ZERO.toString(),
          block: Long.ZERO.toString(),
          app: Long.ZERO.toString(),
        },
        id: "0000000000000000000000000000000000000000",
        listen_addr: endpoint,
        network,
        version,
        channels: "0000000000000000",
        moniker,
        other: {
          tx_index: "off",
          rpc_address: endpoint,
        },
      },
      sync_info: {
        latest_block_hash: blockHash.toUpperCase(),
        latest_app_hash:
          "0000000000000000000000000000000000000000000000000000000000000000",
        latest_block_height: blockNumber,
        latest_block_time: blockTime,
        catching_up: false,
      },
      validator_info: {
        address: "0000000000000000000000000000000000000000",
        pub_key: {
          type: "tendermint/PubKeyEd25519",
          value: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
        },
        voting_power: "0",
      },
    };
  }
}
