import { ApiPromise } from "@polkadot/api";
import { AccountService } from "./account";
import { ApiService } from "./service";
import { IConfig } from "config";
import { QueryAllBalancesResponse } from "cosmjs-types/cosmos/bank/v1beta1/query.js";
import Long from "long";
import { AccountInfo } from "@polkadot/types/interfaces";

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

	public async balances(address: string): Promise<QueryAllBalancesResponse> {
		console.debug('balances');

		const originRaw = await this.accountService.origin(address);
		let amount = '0';
		let origin = originRaw.toString();
		if (!origin) {
			origin = this.accountService.interim(address);
		}
		const account = await this.chainApi.query.system.account(origin);
		if (account) {
			const { data } = account.toJSON() as unknown as AccountInfo;
			amount = data.free.toString();
		}
		const denom = this.config.get<string>('chain.denom');
		const nativeBalance = { denom, amount };

		const assets = [];
		const metadata = await this.chainApi.query.assets.metadata.entries();
		for (const [{ args: [assetId] }, value] of metadata) {
			const asset = await this.chainApi.query.assets.account(assetId.toString(), origin)

			if (asset) {
				const denom = value.toHuman()['symbol'];
				const amount = asset.toJSON() ? BigInt(asset.toJSON()['balance']).toString() : '0';

				assets.push({ denom, amount });
			}
		}

		console.debug([
			nativeBalance,
			...assets,
		]);

		return {
			balances: [
				nativeBalance,
				...assets,
			],
			pagination: {
				nextKey: new Uint8Array(),
				total: Long.ZERO,
			},
		};
	}
}
