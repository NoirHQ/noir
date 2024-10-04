import { ABCIQueryResponse } from "cosmjs-types/cosmos/base/tendermint/v1beta1/query";
import { App } from "../app";
import { AbciService, StatusService, TxService } from "../services";
import { toSnakeCase } from "../utils";
import { BroadcastTxSyncResponse, ResultTxSearch } from "../types";
import querystring from "node:querystring";
import { SocketStream } from "@fastify/websocket";

export function addMethods(context: App) {
	context.jsonrpc.addMethod('status', async (): Promise<unknown> => {
		return toSnakeCase(
			await context.services.get<StatusService>('status').status()
		);
	});

	context.jsonrpc.addMethod(
		'abci_query',
		async ({ path, data }): Promise<unknown> => {
			const result = await context.services
				.get<AbciService>('abci')
				.query(path, data);
			const response = ABCIQueryResponse.toJSON(result);
			return {
				response,
			};
		}
	);

	context.jsonrpc.addMethod(
		'broadcast_tx_sync',
		async ({ tx }): Promise<BroadcastTxSyncResponse> => {
			const result = await context.services.get<TxService>('tx').broadcastTx(tx);
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

	context.jsonrpc.addMethod('tx_search', ({ query }): ResultTxSearch => {
		const args = querystring.parse(query);
		let hash = args['tx.hash'] as string;
		if (hash.includes("'")) {
			hash = hash.replace(/'/gi, '');
		}
		const txs = context.services.get<TxService>('tx').searchTx(hash);
		txs.forEach(tx => {
			tx.tx_result.events.forEach(event => {
				event.attributes = context.services.get<TxService>('tx').encodeAttributes(event.attributes, 'utf8', 'base64');
			});
		});

		return {
			txs,
			total_count: txs.length.toString(),
		};
	});

	context.server.get(
		'/websocket',
		{ websocket: true },
		(connection: SocketStream) => {
			connection.socket.on('message', async (message) => {
				const request = JSON.parse(message.toString());
				const response = await this.jsonrpc.receive(request);
				if (response) {
					connection.socket.send(
						Buffer.from(JSON.stringify(response), 'utf8')
					);
				}
			});
		}
	);
}
