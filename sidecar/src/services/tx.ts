import { ResultTx } from "../types";
import { ApiService } from "./service";
import { Database } from "lmdb";
import {
	BroadcastTxResponse,
	SimulateResponse,
} from "cosmjs-types/cosmos/tx/v1beta1/service.js";
import Long from "long";
import { createHash } from "crypto";
import { convertToCodespace } from "../constants/codespace";
import { encodeTo, sleep } from "../utils";
import { Event as CosmosEvent } from "cosmjs-types/tendermint/abci/types";
import { Header } from "@polkadot/types/interfaces";
import { ChainService } from "./chain";

export type TransactResult = { codespace: string, code: number, gasWanted: number, gasUsed: number, events: CosmosEvent[] };

export class TxService implements ApiService {
	chainService: ChainService;
	db: Database;

	constructor(db: Database, chainService: ChainService) {
		this.chainService = chainService;
		this.db = db;
	}

	public async broadcastTx(txBytes: string): Promise<BroadcastTxResponse> {
		console.debug('broadcastTx');

		const chainApi = await this.chainService.getChainApi();
		let txHash = (await chainApi.rpc['cosmos']['broadcastTx'](`0x${encodeTo(txBytes, 'base64', 'hex')}`)).toString();
		txHash = txHash.startsWith('0x') ? txHash.slice(2) : txHash;
		console.debug(`txHash: ${txHash}`);

		const height = await chainApi.query.system.number();

		return {
			txResponse: {
				height: Long.fromString(height.toString()),
				txhash: txHash.toUpperCase(),
				codespace: '',
				code: 0,
				data: '',
				rawLog: '',
				logs: [],
				info: '',
				gasWanted: Long.fromNumber(0),
				gasUsed: Long.fromNumber(0),
				tx: {
					typeUrl: '',
					value: new Uint8Array(),
				},
				timestamp: '',
				events: [],
			},
		};
	}

	public searchTx(hash: string): ResultTx[] {
		console.debug('searchTx');

		hash = hash.startsWith('0x') ? hash.slice(2) : hash;
		console.debug(`txHash: ${hash}`);

		const resultTx = this.db.get(`tx::result::${hash.toLowerCase()}`);
		const txs: ResultTx[] = [];
		if (resultTx) {
			txs.push(resultTx);
		}
		return txs;
	}

	public async saveTransactResult(
		txBytes: string,
		extrinsicIndex: number,
		header: Header
	): Promise<void> {
		console.debug('saveTransactResult');

		txBytes = txBytes.startsWith('0x') ? txBytes.slice(2) : txBytes;

		const txHash = createHash('sha256').update(Buffer.from(txBytes, 'hex')).digest('hex');
		console.debug(`txHash: ${txHash}`)

		const { codespace, code, gasWanted, gasUsed, events } = await this.checkResult(header, extrinsicIndex);
		const result: ResultTx = {
			hash: `${txHash.toUpperCase()}`,
			height: header.number.toString(),
			index: extrinsicIndex,
			tx_result: {
				code,
				data: '',
				log: '',
				info: '',
				gas_wanted: gasWanted.toString(),
				gas_used: gasUsed.toString(),
				events,
				codespace,
			},
			tx: encodeTo(txBytes, 'hex', 'base64'),
		};
		await this.db.put(`tx::result::${txHash.toLowerCase()}`, result);
	}

	async checkResult(
		header: Header,
		extrinsicIndex: number
	): Promise<TransactResult> {
		const chainApi = await this.chainService.getChainApi();

		/* eslint-disable @typescript-eslint/no-explicit-any */
		const events = (await (
			await chainApi.at(header.hash)
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
					const [gas_wanted, gas_used, events] = JSON.parse(data.toString());
					console.debug(`gasWanted: ${gas_wanted}`);
					console.debug(`gasUsed: ${gas_used}`);

					const cosmosEvents = this.encodeEvents(events, 'hex', 'utf8');
					console.debug(`cosmosEvents: ${JSON.stringify(cosmosEvents)}`)

					return { codespace: '', code: 0, gasWanted: gas_wanted, gasUsed: gas_used, events: cosmosEvents };
				} else {
					const [{ module: { error } }, info] = JSON.parse(data.toString());
					const errors = Uint8Array.from(Buffer.from(error.startsWith('0x') ? error.slice(2) : error, 'hex'));
					const weight = info.weight.refTime;

					return { codespace: convertToCodespace(errors[1]), code: errors[2], gasWanted: 0, gasUsed: weight, events: [] };
				}
			});

		return result[0];
	}

	public async simulate(txBytes: string, blockHash?: string): Promise<SimulateResponse> {
		console.debug('simulate');

		const chainApi = await this.chainService.getChainApi();
		const txRaw = `0x${encodeTo(txBytes, 'base64', 'hex')}`;
		const { gas_info, events } = (await chainApi.rpc['cosmos']['simulate'](txRaw, blockHash)).toJSON();
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
				type: encodeTo(event.type ? event.type : event['r#type'], from, to),
				attributes: this.encodeAttributes(event.attributes, from, to),
			}
		});
	}

	public encodeAttributes(attributes, from: BufferEncoding, to: BufferEncoding) {
		return attributes.map(({ key, value }) => {
			const eventKey = encodeTo(key, from, to);
			const eventValue = encodeTo(value, from, to);

			return {
				key: eventKey,
				value: eventValue,
			}
		});
	}
}
