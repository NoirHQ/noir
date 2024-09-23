import { ApiPromise } from "@polkadot/api";
import { ResultTx } from "../types";
import { ApiService } from "./service";
import { Database } from "lmdb";
import {
	BroadcastTxResponse,
	SimulateResponse,
} from "cosmjs-types/cosmos/tx/v1beta1/service.js";
import Long from "long";
import { createHash } from "crypto";
import { Tx } from "cosmjs-types/cosmos/tx/v1beta1/tx.js";
import { convertToCodespace } from "../constants/codespace";

type TransactResult = { codespace: string, code: number; gasUsed: number, events: any[] };

export class TxService implements ApiService {
	chainApi: ApiPromise;
	db: Database;

	constructor(db: Database, chainApi: ApiPromise) {
		this.chainApi = chainApi;
		this.db = db;
	}

	public async broadcastTx(txBytes: string): Promise<BroadcastTxResponse> {
		console.debug(`txBytes: ${txBytes}`);

		const rawTx = `0x${Buffer.from(txBytes, 'base64').toString('hex')}`;

		let txHash = (await this.chainApi.rpc['cosmos']['broadcastTx'](rawTx)).toString();
		txHash = txHash.startsWith('0x') ? txHash.slice(2) : txHash;

		console.debug(`txHash: ${txHash.toLowerCase()}`);

		const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

		while (true) {
			const txs = this.searchTx(txHash);
			// console.debug(`txs: ${JSON.stringify(txs)}`);

			if (txs.length > 0) {
				const tx = txs.at(0);

				return {
					txResponse: {
						height: Long.fromString(tx.height),
						txhash: txHash.toUpperCase(),
						codespace: tx.tx_result.codespace,
						code: tx.tx_result.code,
						data: tx.tx_result.data,
						rawLog: '',
						logs: [],
						info: tx.tx_result.info,
						gasWanted: Long.fromString(tx.tx_result.gas_wanted),
						gasUsed: Long.fromString(tx.tx_result.gas_used),
						tx: {
							typeUrl: '',
							value: new Uint8Array(),
						},
						timestamp: '',
						events: tx.tx_result.events,
					},
				};
			} else {
				console.debug('Waiting for events...');

				await sleep(1000);
			}
		}
	}

	public searchTx(hash: string): ResultTx[] {
		if (hash.startsWith('0x')) {
			hash = hash.slice(2);
		}

		console.debug(`txHash: ${hash.toLowerCase()}`);

		const resultTx = this.db.get(`tx::result::${hash.toLowerCase()}`);
		const txs: ResultTx[] = [];
		if (resultTx) {
			txs.push(resultTx);
		}
		return txs;
	}

	public async saveTransactResult(
		txRaw: string,
		extrinsicIndex: number,
		header: any
	): Promise<void> {
		txRaw = txRaw.startsWith('0x') ? txRaw.slice(2) : txRaw;
		const txBytes = Buffer.from(txRaw, 'hex');
		const gasLimit = Tx.decode(txBytes).authInfo!.fee!.gasLimit;

		const txHash = createHash('sha256').update(Buffer.from(txRaw, 'hex')).digest('hex');

		const { codespace, code, gasUsed, events } = await this.checkResult(header, extrinsicIndex);
		const txResult: ResultTx = {
			hash: `${txHash.toUpperCase()}`,
			height: header.number.toString(),
			index: extrinsicIndex,
			tx_result: {
				code,
				data: '',
				log: '',
				info: '',
				gas_wanted: gasLimit.toString(),
				gas_used: gasUsed.toString(),
				events,
				codespace,
			},
			tx: txBytes.toString('base64'),
		};
		await this.db.put(`tx::result::${txHash.toLowerCase()}`, txResult);
	}

	async checkResult(
		header: any,
		extrinsicIndex: number
	): Promise<TransactResult> {
		const events = (await (
			await this.chainApi.at(header.hash)
		).query.system.events()) as any;
		const result = events
			.filter(({ event: { section, method }, phase }) => {
				const { applyExtrinsic } = JSON.parse(phase.toString());
				return (
					applyExtrinsic === extrinsicIndex &&
					(`${section}::${method}` === 'cosmos::Executed' ||
						`${section}::${method}` === 'system::ExtrinsicFailed')
				);
			})
			.map(({ event: { data, section, method } }) => {
				if (`${section}::${method}` === 'cosmos::Executed') {
					const [gas_wanted, gas_used, events] = JSON.parse(data);

					console.debug(`gasWanted: ${gas_wanted}`);
					console.debug(`gasUsed: ${gas_used}`);
					console.debug(`events: ${JSON.stringify(events)}`);

					const cosmosEvents = this.encodeEvents(events, 'hex', 'utf8');

					console.debug(`cosmosEvents: ${JSON.stringify(cosmosEvents)}`)

					return { codespace: '', code: 0, gasUsed: gas_used, events: cosmosEvents };
				} else {
					console.debug(JSON.parse(data));
					const [{ module: { index, error } }, info] = JSON.parse(data);

					const errors = Uint8Array.from(Buffer.from(error.startsWith('0x') ? error.slice(2) : error, 'hex'));
					const weight = info.weight.refTime;

					return { codespace: convertToCodespace(errors[1]), code: errors[2], gasUsed: weight, events: [] };
				}
			});
		return result[0];
	}

	convert(str: string, from: BufferEncoding, to: BufferEncoding): string {
		if (from === 'hex') {
			str = str.startsWith('0x') ? str.slice(2) : str;
		}
		return Buffer.from(str, from).toString(to);
	}

	public async simulate(txBytes: string, blockHash?: string): Promise<SimulateResponse> {
		const txRaw = `0x${this.convert(txBytes, 'base64', 'hex')}`;
		const { gas_info, events } = (await this.chainApi.rpc['cosmos']['simulate'](txRaw, blockHash)).toJSON();
		const cosmosEvents = this.encodeEvents(events, 'hex', 'utf8');

		console.debug(`gasInfo: ${JSON.stringify(gas_info)}`);
		console.debug(`events: ${JSON.stringify(cosmosEvents)}`);

		return {
			gasInfo: {
				gasWanted: Long.fromNumber(gas_info.gas_wanted),
				gasUsed: Long.fromNumber(gas_info.gas_used),
			},
			result: {
				data: new Uint8Array(),
				log: '',
				events: cosmosEvents,
				msgResponses: [],
			},
		};
	}

	public encodeEvents(events, from: BufferEncoding, to: BufferEncoding) {
		return events.map((event) => {
			return {
				type: this.convert(event.type ? event.type : event['r#type'], from, to),
				attributes: this.encodeAttributes(event.attributes, from, to),
			}
		});
	}

	public encodeAttributes(attributes, from: BufferEncoding, to: BufferEncoding) {
		return attributes.map(({ key, value }) => {
			const eventKey = this.convert(key, from, to);
			const eventValue = this.convert(value, from, to);

			return {
				key: eventKey,
				value: eventValue,
			}
		});
	}
}
