import fastify, {
  FastifyInstance,
  FastifyReply,
  FastifyRequest,
} from "fastify";
import { ApiPromise, WsProvider } from "@pinot/api";
import {
  AbciService,
  ApiServices,
  BalanceService,
  DistributionService,
  NodeInfoService,
  StakingService,
  StatusService,
  TxService,
  IAccountService,
  NoirAccountService,
  HorizonAccountService,
} from "./services/index.js";
import {
  AccountResponse,
  BroadcastTxSyncResponse,
  ResultTxSearch,
} from "./types/index.js";
import path from "path";
import fastifyStatic from "@fastify/static";
import FastifyWebsocket, { SocketStream } from "@fastify/websocket";
import { JSONRPCServer } from "json-rpc-2.0";
import { IConfig } from "config";
import { Database, open } from "lmdb";
import quertstring from "node:querystring";
import rpc from "./constants/rpc.js";
import { QueryAccountRequest } from "cosmjs-types/cosmos/auth/v1beta1/query.js";
import {
  ABCIQueryResponse,
  GetNodeInfoResponse,
} from "cosmjs-types/cosmos/base/tendermint/v1beta1/query.js";
import {
  QueryAllBalancesRequest,
  QueryAllBalancesResponse,
} from "cosmjs-types/cosmos/bank/v1beta1/query.js";
import { toSnakeCase } from "./utils/index.js";
import {
  QueryDelegationTotalRewardsRequest,
  QueryDelegationTotalRewardsResponse,
} from "cosmjs-types/cosmos/distribution/v1beta1/query.js";
import {
  QueryDelegatorDelegationsRequest,
  QueryDelegatorDelegationsResponse,
  QueryDelegatorUnbondingDelegationsRequest,
  QueryDelegatorUnbondingDelegationsResponse,
} from "cosmjs-types/cosmos/staking/v1beta1/query.js";
import {
  BroadcastTxResponse,
  SimulateResponse,
} from "cosmjs-types/cosmos/tx/v1beta1/service.js";

export class App {
  config: IConfig;
  db: Database;
  server: FastifyInstance;
  services: ApiServices;
  jsonrpc: JSONRPCServer;
  chainApi: ApiPromise;

  constructor(config: IConfig) {
    this.config = config;
    this.services = new ApiServices();
  }

  async initialize() {
    await this.initDatabase();
    await this.initApiServices();
    await this.initApiServer();
    await this.initJsonRpcServer();
    await this.initSubscribeEvents();
  }

  async start() {
    const port = this.config.get<number>("server.port");
    await this.server.listen({ port });
  }

  async initDatabase() {
    const path = this.config.get<string>("db.path");
    this.db = open({
      path,
      compression: true,
    });
  }

  async initApiServices() {
    const endpoint = this.config.get<string>("chain.endpoint");
    const provider = new WsProvider(endpoint);
    this.chainApi = await ApiPromise.create({
      provider,
      rpc,
    });
    let accountService: IAccountService;
    if (this.config.get<string>("chain.node") === "horizon") {
      accountService = new HorizonAccountService(this.chainApi);
    } else if (this.config.get<string>("chain.node") === "noir") {
      accountService = new NoirAccountService(this.chainApi);
    } else {
      throw new Error("unsupported node type");
    }
    const abciService = new AbciService(this.chainApi, accountService);
    const balanceService = new BalanceService(
      this.config,
      this.chainApi,
      accountService
    );
    const distributionService = new DistributionService();
    const nodeInfoService = new NodeInfoService(this.config);
    const stakingService = new StakingService();
    const statusService = new StatusService(this.config, this.chainApi);
    const txService = new TxService(this.db, this.chainApi);

    this.services.set("abci", abciService);
    this.services.set("account", accountService);
    this.services.set("balance", balanceService);
    this.services.set("distribution", distributionService);
    this.services.set("nodeInfo", nodeInfoService);
    this.services.set("staking", stakingService);
    this.services.set("status", statusService);
    this.services.set("tx", txService);
  }

  async initApiServer() {
    const logger = this.config.get<boolean>("server.logger");
    this.server = fastify({ logger });
    const __dirname = path.resolve();
    this.server.register(fastifyStatic, {
      root: path.join(__dirname, "public"),
    });
    await this.server.register(FastifyWebsocket);

    this.server.get("/", (request: FastifyRequest, reply: FastifyReply) => {
      reply.sendFile("index.html");
    });

    this.server.get(
      "/cosmos/bank/v1beta1/balances/:address",
      async (
        request: FastifyRequest<{
          Params: QueryAllBalancesRequest;
        }>
      ): Promise<unknown> => {
        const { address } = request.params;
        const response = QueryAllBalancesResponse.toJSON(
          await this.services.get<BalanceService>("balance").balances(address)
        );
        return toSnakeCase(response);
      }
    );

    this.server.get(
      "/cosmos/auth/v1beta1/accounts/:address",
      async (
        request: FastifyRequest<{
          Params: QueryAccountRequest;
        }>
      ): Promise<AccountResponse> => {
        const { address } = request.params;
        return await this.services
          .get<IAccountService>("account")
          .accounts(address);
      }
    );

    this.server.get(
      "/cosmos/base/tendermint/v1beta1/node_info",
      (): GetNodeInfoResponse => {
        const response = GetNodeInfoResponse.toJSON(
          this.services.get<NodeInfoService>("nodeInfo").nodeInfo()
        );
        return toSnakeCase(response);
      }
    );

    this.server.post(
      "/cosmos/tx/v1beta1/txs",
      async (
        request: FastifyRequest<{
          Body: {
            tx_bytes: string;
            mode: number;
          };
        }>
      ): Promise<unknown> => {
        const { tx_bytes } = request.body;
        const response = BroadcastTxResponse.toJSON(
          await this.services.get<TxService>("tx").broadcastTx(tx_bytes)
        );
        return toSnakeCase(response);
      }
    );

    this.server.get(
      "/cosmos/staking/v1beta1/delegations/:delegatorAddr",
      (
        request: FastifyRequest<{
          Params: QueryDelegatorDelegationsRequest;
        }>
      ): unknown => {
        const { delegatorAddr } = request.params;
        const response = QueryDelegatorDelegationsResponse.toJSON(
          this.services
            .get<StakingService>("staking")
            .deligations(delegatorAddr)
        );
        return toSnakeCase(response);
      }
    );

    this.server.get(
      "/cosmos/distribution/v1beta1/delegators/:delegatorAddress/rewards",
      (
        request: FastifyRequest<{
          Params: QueryDelegationTotalRewardsRequest;
        }>
      ): unknown => {
        const { delegatorAddress } = request.params;
        const response = QueryDelegationTotalRewardsResponse.toJSON(
          this.services
            .get<DistributionService>("distribution")
            .rewards(delegatorAddress)
        );
        return toSnakeCase(response);
      }
    );

    this.server.get(
      "/cosmos/staking/v1beta1/delegators/:delegatorAddr/unbonding_delegations",
      (
        request: FastifyRequest<{
          Params: QueryDelegatorUnbondingDelegationsRequest;
        }>
      ): unknown => {
        const { delegatorAddr } = request.params;
        const response = QueryDelegatorUnbondingDelegationsResponse.toJSON(
          this.services
            .get<StakingService>("staking")
            .unbondingDeligations(delegatorAddr)
        );
        return toSnakeCase(response);
      }
    );

    this.server.post(
      "/cosmos/tx/v1beta1/simulate",
      (
        request: FastifyRequest<{
          Body: { tx_bytes: string };
        }>
      ): unknown => {
        const { tx_bytes } = request.body;
        const response = SimulateResponse.toJSON(
          this.services.get<TxService>("tx").simulateTx(tx_bytes)
        );
        return toSnakeCase(response);
      }
    );
  }

  async initJsonRpcServer() {
    this.jsonrpc = new JSONRPCServer();

    this.jsonrpc.addMethod("status", async (): Promise<unknown> => {
      return toSnakeCase(
        await this.services.get<StatusService>("status").status()
      );
    });

    this.jsonrpc.addMethod(
      "abci_query",
      async ({ path, data }): Promise<unknown> => {
        const result = await this.services
          .get<AbciService>("abci")
          .query(path, data);
        const response = ABCIQueryResponse.toJSON(result);
        return {
          response,
        };
      }
    );

    this.jsonrpc.addMethod(
      "broadcast_tx_sync",
      async ({ tx }): Promise<BroadcastTxSyncResponse> => {
        const result = await this.services.get<TxService>("tx").broadcastTx(tx);
        const { code, txhash, data, rawLog, codespace } = result.txResponse;
        return {
          code,
          data,
          log: rawLog,
          codespace,
          hash: txhash,
        };
      }
    );

    this.jsonrpc.addMethod("tx_search", ({ query }): ResultTxSearch => {
      const args = quertstring.parse(query);
      let hash = args["tx.hash"] as string;
      if (hash.includes("'")) {
        hash = hash.replace(/'/gi, "");
      }
      return this.services.get<TxService>("tx").searchTx(hash);
    });

    this.server.get(
      "/websocket",
      { websocket: true },
      (connection: SocketStream) => {
        connection.socket.on("message", async (message) => {
          const request = JSON.parse(message.toString());
          const response = await this.jsonrpc.receive(request);
          if (response) {
            connection.socket.send(
              Buffer.from(JSON.stringify(response), "utf8")
            );
          }
        });
      }
    );
  }

  async initSubscribeEvents() {
    await this.chainApi.rpc.chain.subscribeNewHeads(
      async (header) => {
        const signedBlock = await this.chainApi.rpc.chain.getBlock(header.hash);

        signedBlock.block.extrinsics.forEach(
          async ({ method: { args, method, section } }, index) => {
            if (section === "cosmos" && method === "transact") {
              const tx = JSON.parse(args.toString());
              await this.services
                .get<TxService>("tx")
                .saveTransactResult(tx, index, header);
            }
          }
        );
      }
    );
  }
}
