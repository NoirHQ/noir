import { ApiPromise } from "@polkadot/api";
import { AccountService } from "./account";
import { ApiService } from "./service";
import { IConfig } from "config";
import { QueryAllBalancesResponse, QueryBalanceResponse } from "cosmjs-types/cosmos/bank/v1beta1/query.js";
import Long from "long";
import { AccountInfo } from "@polkadot/types/interfaces";
import { encodeTo } from "../utils";

export class BalanceService implements ApiService {
	config: IConfig;
	chainApi: ApiPromise;
	accountService: AccountService;

	constructor(
		config: IConfig,
		chainApi: ApiPromise,
		accountService: AccountService
	) {
		this.config = config;
		this.chainApi = chainApi;
		this.accountService = accountService;
	}

	public async balance(address: string, denom: string, blockHash?: string): Promise<QueryBalanceResponse> {
		console.debug('balance');

		const originRaw = await this.accountService.origin(address);
		let amount = '0';
		let origin = originRaw.toString();
		if (!origin) {
			origin = this.accountService.interim(address);
		}

		const nativeDenom = this.config.get<string>('chain.denom');
		if (nativeDenom === denom) {
			const account = await (await (blockHash ? this.chainApi.at(blockHash) : this.chainApi)).query['system']['account'](origin);
			if (account) {
				const { data } = account.toJSON() as unknown as AccountInfo;
				amount = BigInt(data.free.toString()).toString();
				return {
					balance: { denom, amount }
				}
			} else {
				return {
					balance: { denom, amount: '0' }
				}
			}
		} else {
			const assetId = await (await (blockHash ? this.chainApi.at(blockHash) : this.chainApi)).query.assetMap.index(denom);
			const asset = await (await (blockHash ? this.chainApi.at(blockHash) : this.chainApi)).query.assets.account(assetId.toString(), origin);
			if (!asset.isEmpty) {
				const amount = BigInt(asset.toJSON()['balance']).toString();
				console.debug(`denom: ${denom}, amount: ${amount}`);
				return {
					balance: { denom, amount }
				}
			} else {
				return {
					balance: { denom, amount: '0' }
				}
			}
		}
	}

	public async balances(address: string, blockHash?: string): Promise<QueryAllBalancesResponse> {
		console.debug('balances');

		const originRaw = await this.accountService.origin(address);
		let amount = '0';
		let origin = originRaw.toString();
		if (!origin) {
			origin = this.accountService.interim(address);
		}
		const account = await (await (blockHash ? this.chainApi.at(blockHash) : this.chainApi)).query['system']['account'](origin);
		if (account) {
			const { data } = account.toJSON() as unknown as AccountInfo;
			amount = BigInt(data.free.toString()).toString();
		}
		const denom = this.config.get<string>('chain.denom');
		const nativeBalance = { denom, amount };

		const assets = [];
		const metadata = await (await (blockHash ? this.chainApi.at(blockHash) : this.chainApi)).query.assets.metadata.entries();
		for (const [{ args: [assetId] }] of metadata) {
			const asset = await (await (blockHash ? this.chainApi.at(blockHash) : this.chainApi)).query.assets.account(assetId.toString(), origin);

			if (!asset.isEmpty) {
				const assetDenom = await (await (blockHash ? this.chainApi.at(blockHash) : this.chainApi)).query.assetMap.map(assetId);

				if (!assetDenom.isEmpty) {
					const denomSet = assetDenom.toJSON();
					const denom = encodeTo(denomSet[0].toString(), 'hex', 'utf8');
					const amount = BigInt(asset.toJSON()['balance']).toString();

					console.debug(`denom: ${denom}, amount: ${amount}`);

					assets.push({ denom, amount });
				}
			}
		}

		const balances = [
			nativeBalance,
			...assets,
		];

		console.debug(`balances: ${JSON.stringify(balances)}`);

		return {
			balances,
			pagination: {
				nextKey: new Uint8Array(),
				total: Long.ZERO,
			},
		};
	}
}
