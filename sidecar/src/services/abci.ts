import {
	QueryAccountRequest,
	QueryAccountResponse,
} from "cosmjs-types/cosmos/auth/v1beta1/query.js";
import { ApiService } from "./service";
import { AccountService } from "./account";
import { PubKey } from "cosmjs-types/cosmos/crypto/secp256k1/keys.js";
import { BaseAccount } from "cosmjs-types/cosmos/auth/v1beta1/auth.js";
import Long from "long";
import { ApiPromise } from "@polkadot/api";
import { ABCIQueryResponse } from "cosmjs-types/cosmos/base/tendermint/v1beta1/query.js";
import { SimulateRequest, SimulateResponse } from "cosmjs-types/cosmos/tx/v1beta1/service.js";
import { TxService } from "./tx";
import { QuerySmartContractStateRequest, QuerySmartContractStateResponse } from 'cosmjs-types/cosmwasm/wasm/v1/query.js'
import { convertToCodespace } from "../constants/codespace";
import { encodeTo } from "../utils";

export class AbciService implements ApiService {
	chainApi: ApiPromise;
	accountService: AccountService;
	txService: TxService;

	constructor(chainApi: ApiPromise, accountService: AccountService, txService: TxService) {
		this.chainApi = chainApi;
		this.accountService = accountService;
		this.txService = txService;
	}

	async query(path: string, data: string): Promise<ABCIQueryResponse> {
		console.debug(`query(${path}, ${data})`);

		if (path === '/cosmos.auth.v1beta1.Query/Account') {
			const address = QueryAccountRequest.decode(
				Buffer.from(data, 'hex')
			).address;

			const height = await this.chainApi.query.system.number();
			const blockHash = await this.chainApi.rpc.chain.getBlockHash(height.toString());

			const { account } = await this.accountService.accounts(address, blockHash.toString());
			const pubkey: PubKey = {
				key: Buffer.from(account.pub_key.key, 'base64'),
			};
			const baseAccount: BaseAccount = {
				address: account.address,
				pubKey: {
					typeUrl: '/cosmos.crypto.secp256k1.PubKey',
					value: PubKey.encode(pubkey).finish(),
				},
				accountNumber: Long.fromNumber(parseInt(account.account_number)),
				sequence: Long.fromNumber(parseInt(account.sequence)),
			};

			const queryAccountResponse: QueryAccountResponse = {
				account: {
					typeUrl: '/cosmos.auth.v1beta1.BaseAccount',
					value: BaseAccount.encode(baseAccount).finish(),
				},
			};
			const value = QueryAccountResponse.encode(queryAccountResponse).finish();
			return {
				code: 0,
				log: '',
				info: '',
				index: Long.ZERO,
				key: undefined,
				value,
				proofOps: undefined,
				height: Long.fromString(height.toString()),
				codespace: '',
			};
		} else if (path === '/cosmos.tx.v1beta1.Service/Simulate') {
			const height = await this.chainApi.query.system.number();
			const blockHash = await this.chainApi.rpc.chain.getBlockHash(height.toString());
			try {
				const req = SimulateRequest.decode(Buffer.from(data, 'hex'));
				const res = await this.txService.simulate(Buffer.from(req.txBytes).toString('base64'), blockHash.toString());
				const resBytes = SimulateResponse.encode(res).finish();
				return {
					code: 0,
					log: '',
					info: '',
					index: Long.ZERO,
					key: undefined,
					value: resBytes,
					proofOps: undefined,
					height: Long.fromString(height.toString()),
					codespace: '',
				};
			} catch (e: unknown) {
				const message = e.toString();
				const codespace = message.slice(message.indexOf('codespace:') + 'codespace:'.length, message.indexOf('code:')).trim();
				const code = message.slice(message.indexOf('code:') + 'code:'.length, message.indexOf('}')).trim();
				return {
					code: parseInt(code),
					log: message,
					info: '',
					index: Long.ZERO,
					key: undefined,
					value: new Uint8Array(),
					proofOps: undefined,
					height: Long.fromString(height.toString()),
					codespace: convertToCodespace(parseInt(codespace)),
				};
			}
		} else if (path === '/cosmwasm.wasm.v1.Query/SmartContractState') {
			const { address, queryData } = QuerySmartContractStateRequest.decode(Uint8Array.from(Buffer.from(data, 'hex')));
			const gas = 10000000000;
			const msg = {
				wasm: {
					smart: {
						contract_addr: address, msg: Buffer.from(queryData).toString('base64')
					}
				}
			};

			const height = await this.chainApi.query.system.number();
			const blockHash = await this.chainApi.rpc.chain.getBlockHash(height.toString());

			const response = await this.chainApi.rpc['cosmwasm']['query'](address, gas, `0x${encodeTo(JSON.stringify(msg), 'utf8', 'hex')}`, blockHash.toString());
			const stateResponse = QuerySmartContractStateResponse.fromPartial({ data: Uint8Array.from(Buffer.from(response, 'hex')) });

			return {
				code: 0,
				log: '',
				info: '',
				index: Long.ZERO,
				key: undefined,
				value: QuerySmartContractStateResponse.encode(stateResponse).finish(),
				proofOps: undefined,
				height: Long.fromString(height.toString()),
				codespace: '',
			};
		} else {
			throw new Error('Unexpected path');
		}
	}
}
