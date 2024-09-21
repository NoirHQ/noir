import { ApiPromise } from "@polkadot/api";
import { AccountResponse } from "../types";
import { fromBech32 } from "@cosmjs/encoding";
import { Codec } from "@polkadot/types/types/index.js";
import { blake2b } from "ethereum-cryptography/blake2b.js";
import { ApiService } from "./service";
import Dummy from "../constants/dummy";

export interface IAccountService extends ApiService {
	accounts(address: string): Promise<AccountResponse>;
	origin(address: string): Promise<Codec>;
	interim(address: string): string;
}

export class NoirAccountService implements IAccountService {
	chainApi: ApiPromise;

	constructor(chainApi: ApiPromise) {
		this.chainApi = chainApi;
	}

	public async accounts(address: string): Promise<AccountResponse> {
		let sequence = '0';

		const originRaw = await this.origin(address);
		const origin = originRaw.toString();
		const account = await this.chainApi.query["system"]["account"](origin);
		if (account) {
			const { nonce } = account.toJSON() as any;
			sequence = nonce.toString();
		}
		return new AccountResponse({
			"@type": "/cosmos.auth.v1beta1.BaseAccount",
			address,
			pub_key: {
				"@type": "/cosmos.crypto.secp256k1.PubKey",
				key: Dummy.Secp256k1PublicKey,
			},
			account_number: "0",
			sequence,
		});
	}

	public async origin(address: string): Promise<any> {
		const { data } = fromBech32(address);
		return this.chainApi.query['addressMap']['index'](
			Buffer.concat([Buffer.from([0x00]), data])
		);
	}

	public interim(address: string): string {
		const { data } = fromBech32(address);
		const addressRaw = Buffer.from(
			blake2b(Buffer.concat([Buffer.from("cosm:", "utf8"), data]), 32),
		);
		return `0x${addressRaw.toString("hex")}`;
	}
}
