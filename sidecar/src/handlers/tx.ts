import { FastifyRequest } from "fastify";
import { toSnakeCase } from "../utils";
import { BroadcastTxResponse, SimulateResponse } from "cosmjs-types/cosmos/tx/v1beta1/service";
import { TxService } from "../services";

export class TxHandler {
	txService: TxService;

	constructor(txService: TxService) {
		this.txService = txService;
	}

	handlePostTxs = async (
		request: FastifyRequest<{
			Body: {
				tx_bytes: string;
				mode: number;
			};
		}>
	): Promise<unknown> => {
		const { tx_bytes } = request.body;
		const response = BroadcastTxResponse.toJSON(
			await this.txService.broadcastTx(tx_bytes)
		);
		return toSnakeCase(response);
	}

	handlePostSimulate = async (request: FastifyRequest<{
		Body: { tx_bytes: string };
	}>
	): Promise<unknown> => {
		const { tx_bytes } = request.body;
		const response = SimulateResponse.toJSON(
			await this.txService.simulate(tx_bytes)
		);
		return toSnakeCase(response);
	}
}
