import { GetNodeInfoResponse } from "cosmjs-types/cosmos/base/tendermint/v1beta1/query.js";
import { ApiService } from "./service.js";
import { IConfig } from "config";
import Long from "long";

export class NodeInfoService implements ApiService {
  config: IConfig;

  constructor(config: IConfig) {
    this.config = config;
  }

  public nodeInfo(): GetNodeInfoResponse {
    const endpoint = this.config.get<string>("server.endpoint");
    const network = this.config.get<string>("chain.network");
    const version = this.config.get<string>("chain.version");
    const moniker = this.config.get<string>("chain.moniker");
    const name = this.config.get<string>("chain.name");
    return {
      defaultNodeInfo: {
        protocolVersion: {
          p2p: Long.ZERO,
          block: Long.ZERO,
          app: Long.ZERO,
        },
        defaultNodeId: "0000000000000000000000000000000000000000",
        listenAddr: endpoint,
        network,
        version,
        channels: new Uint8Array(Buffer.allocUnsafe(8)),
        moniker,
        other: {
          txIndex: "off",
          rpcAddress: "",
        },
      },
      applicationVersion: {
        name,
        appName: name,
        version,
        gitCommit: "0000000000000000000000000000000000000000",
        buildTags: "",
        goVersion: "0",
        buildDeps: [],
        cosmosSdkVersion: "0",
      },
    };
  }
}
