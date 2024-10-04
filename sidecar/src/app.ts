import fastify, {
	FastifyInstance,
} from "fastify";
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
	ChainService,
} from "./services";
import FastifyWebsocket from "@fastify/websocket";
import { JSONRPCServer } from "json-rpc-2.0";
import { IConfig } from "config";
import { Database, open } from "lmdb";
import { Header } from "@polkadot/types/interfaces";
import {
	BalanceHandler,
	AccountHandler,
	DistributionHandler,
	NodeInfoHandler,
	TxHandler,
	StakingHandler,
	WebsocketHandler
} from "./handlers";
import {
	StatusRpcHandler,
	AbciRpcHandler,
	TxRpcHandler
} from "./rpc";

export class App {
	config: IConfig;
	db: Database;
	server: FastifyInstance;
	services: ApiServices;
	jsonrpc: JSONRPCServer;

	constructor(config: IConfig) {
		this.config = config;
		this.services = new ApiServices();
	}

	async initialize() {
		console.debug('Start to initialize app');

		await this.initDatabase();
		await this.initApiServices();
		await this.initJsonRpcServer();
		await this.initApiServer();
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
		console.debug('Initialize API Services');

		const endpoint = this.config.get<string>('chain.endpoint');
		const chainService = new ChainService(endpoint);

		const accountService = new AccountService(chainService);
		const txService = new TxService(this.db, chainService);
		const balanceService = new BalanceService(
			this.config,
			chainService,
			accountService
		);
		const abciService = new AbciService(chainService, accountService, balanceService, txService);
		const distributionService = new DistributionService();
		const nodeInfoService = new NodeInfoService(this.config, chainService);
		const stakingService = new StakingService();
		const statusService = new StatusService(this.config, chainService);

		this.services.set('chain', chainService);
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
		console.debug('Initialize API Server');

		const logger = this.config.get<boolean>('server.logger');
		this.server = fastify({ logger });
		await this.server.register(FastifyWebsocket);

		this.server.addHook('preHandler', (req, res, done) => {
			res.header('Access-Control-Allow-Origin', '*');
			res.header('Access-Control-Allow-Methods', '*');
			res.header('Access-Control-Allow-Headers', '*');

			done();
		});

		const balanceHandler = new BalanceHandler(this.services.get<BalanceService>('balance'));
		const accountHandler = new AccountHandler(this.services.get<AccountService>('account'));
		const distributionHandler = new DistributionHandler(this.services.get<DistributionService>('distribution'));
		const nodeInfoHandler = new NodeInfoHandler(this.services.get<NodeInfoService>('nodeInfo'));
		const txHandler = new TxHandler(this.services.get<TxService>('tx'));
		const stakingHandler = new StakingHandler(this.services.get<StakingService>('staking'));
		const wsHandler = new WebsocketHandler(this.jsonrpc);

		this.server.get('/cosmos/bank/v1beta1/balances/:address', balanceHandler.handleGetBalance);
		this.server.get('/cosmos/auth/v1beta1/accounts/:address', accountHandler.handleGetAccount);
		this.server.get('/cosmos/distribution/v1beta1/delegators/:delegatorAddress/rewards', distributionHandler.handleGetDistribution);
		this.server.get('/cosmos/base/tendermint/v1beta1/node_info', nodeInfoHandler.handleGetNodeInfo);
		this.server.post('/cosmos/tx/v1beta1/txs', txHandler.handlePostTxs);
		this.server.post('/cosmos/tx/v1beta1/simulate', txHandler.handlePostSimulate);
		this.server.get('/cosmos/staking/v1beta1/delegations/:delegatorAddr', stakingHandler.handleGetStaking);
		this.server.get('/cosmos/staking/v1beta1/delegators/:delegatorAddr/unbonding_delegations', stakingHandler.handleGetUnbondingDelegations);
		this.server.get('/websocket', { websocket: true }, wsHandler.handlerMessage);
	}

	async initJsonRpcServer() {
		this.jsonrpc = new JSONRPCServer();

		const statusHandler = new StatusRpcHandler(this.services.get<StatusService>('status'));
		const abciHandler = new AbciRpcHandler(this.services.get<AbciService>('abci'));
		const txHandler = new TxRpcHandler(this.services.get<TxService>('tx'));

		this.jsonrpc.addMethod('status', statusHandler.status);
		this.jsonrpc.addMethod('abci_query', abciHandler.abciQuery);
		this.jsonrpc.addMethod('broadcast_tx_sync', txHandler.broadcastTxSync);
		this.jsonrpc.addMethod('tx_search', txHandler.txSearch);
	}

	async initSubscribeEvents() {
		const chainApi = await this.services.get<ChainService>('chain').getChainApi();
		await chainApi.rpc.chain.subscribeNewHeads(
			async (header: Header) => {
				const signedBlock = await chainApi.rpc.chain.getBlock(header.hash);

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
