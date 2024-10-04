import fastify, {
	FastifyInstance,
} from "fastify";
import { ApiPromise, WsProvider } from "@polkadot/api";
import {
	AbciService,
	ApiServices,
	BalanceService,
	DistributionService,
	NodeInfoService,
	StakingService,
	StatusService,
	TxService,
	AccountService,
} from "./services";
import path from "path";
import fastifyStatic from "@fastify/static";
import FastifyWebsocket from "@fastify/websocket";
import { JSONRPCServer } from "json-rpc-2.0";
import { IConfig } from "config";
import { Database, open } from "lmdb";
import rpc from "./constants/rpc";
import { Header } from "@polkadot/types/interfaces";
import { types } from "./constants/types";
import {
	BalanceHandler,
	AccountHandler,
	DistributionHandler,
	NodeInfoHandler,
	TxHandler,
	StakingHandler
} from "./handlers";
import { addMethods } from "./rpc";

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
		console.debug('Start to initialize app');

		await this.initDatabase();
		await this.initApiServices();
		await this.initApiServer();
		await this.initJsonRpcServer();
		await this.initSubscribeEvents();
	}

	async start() {
		const port = this.config.get<number>('server.port');
		console.debug(`Start to listen on port: ${port}`);

		await this.server.listen({ port });
	}

	async initDatabase() {
		const path = this.config.get<string>('db.path');
		console.debug(`Start to initialize database. path: ${path}`);

		this.db = open({
			path,
			compression: true,
		});
	}

	async initApiServices() {
		const endpoint = this.config.get<string>('chain.endpoint');
		console.debug(`Try connecting to chain RPC. endpoint: ${endpoint}`);

		this.chainApi = await ApiPromise.create({
			provider: new WsProvider(endpoint),
			types,
			rpc
		});
		if (this.chainApi.isConnected) {
			console.debug('Chain RPC connected');
		} else {
			console.error('Failed to connect with chain RPC');
		}

		const accountService = new AccountService(this.chainApi);
		const txService = new TxService(this.db, this.chainApi);
		const balanceService = new BalanceService(
			this.config,
			this.chainApi,
			accountService
		);
		const abciService = new AbciService(this.chainApi, accountService, balanceService, txService);
		const distributionService = new DistributionService();
		const nodeInfoService = new NodeInfoService(this.config, this.chainApi);
		const stakingService = new StakingService();
		const statusService = new StatusService(this.config, this.chainApi);

		this.services.set('abci', abciService);
		this.services.set('account', accountService);
		this.services.set('balance', balanceService);
		this.services.set('distribution', distributionService);
		this.services.set('nodeInfo', nodeInfoService);
		this.services.set('staking', stakingService);
		this.services.set('status', statusService);
		this.services.set('tx', txService);
	}

	async initApiServer() {
		const logger = this.config.get<boolean>('server.logger');
		this.server = fastify({ logger });
		const __dirname = path.resolve();
		this.server.register(fastifyStatic, {
			root: path.join(__dirname, 'public'),
		});
		await this.server.register(FastifyWebsocket);

		const balanceHandler = new BalanceHandler(this.services.get<BalanceService>('balance'));
		const accountHandler = new AccountHandler(this.services.get<AccountService>('account'));
		const distributionHandler = new DistributionHandler(this.services.get<DistributionService>('distribution'));
		const nodeInfoHandler = new NodeInfoHandler(this.services.get<NodeInfoService>('nodeInfo'));
		const txHandler = new TxHandler(this.services.get<TxService>('tx'));
		const stakingHandler = new StakingHandler(this.services.get<StakingService>('staking'));

		this.server.get('/cosmos/bank/v1beta1/balances/:address', balanceHandler.handleGetBalance);
		this.server.get('/cosmos/auth/v1beta1/accounts/:address', accountHandler.handleGetAccount);
		this.server.get('/cosmos/distribution/v1beta1/delegators/:delegatorAddress/rewards', distributionHandler.handleGetDistribution);
		this.server.get('/cosmos/base/tendermint/v1beta1/node_info', nodeInfoHandler.handleGetNodeInfo);
		this.server.post('/cosmos/tx/v1beta1/txs', txHandler.handlePostTxs);
		this.server.post('/cosmos/tx/v1beta1/simulate', txHandler.handlePostSimulate);
		this.server.get('/cosmos/staking/v1beta1/delegations/:delegatorAddr', stakingHandler.handleGetStaking);
		this.server.get('/cosmos/staking/v1beta1/delegators/:delegatorAddr/unbonding_delegations', stakingHandler.handleGetUnbondingDelegations);
	}

	async initJsonRpcServer() {
		this.jsonrpc = new JSONRPCServer();
		addMethods(this);
	}

	async initSubscribeEvents() {
		await this.chainApi.rpc.chain.subscribeNewHeads(
			async (header: Header) => {
				const signedBlock = await this.chainApi.rpc.chain.getBlock(header.hash);

				signedBlock.block.extrinsics.forEach(
					async ({ method: { args, method, section } }, index) => {
						if (section === 'cosmos' && method === 'transact') {
							const txBytes = args.toString().split(',')[0];
							await this.services
								.get<TxService>('tx')
								.saveTransactResult(txBytes, index, header);
						}
					}
				);
			}
		);
	}
}
