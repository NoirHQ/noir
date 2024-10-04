import {
	QueryAllBalancesRequest,
	QueryAllBalancesResponse
} from "cosmjs-types/cosmos/bank/v1beta1/query";
import { FastifyRequest } from "fastify";
import { BalanceService } from "../services";
import { toSnakeCase } from "../utils";

export class BalanceHandler {
	balanceService: BalanceService;

	constructor(balanceService: BalanceService) {
		this.balanceService = balanceService;
	}

	handleGetBalance = async (
		request: FastifyRequest<{
			Params: QueryAllBalancesRequest;
		}>
	): Promise<unknown> => {
		const { address } = request.params;

		const response = QueryAllBalancesResponse.toJSON(
			await this.balanceService.balances(address)
		);
		return toSnakeCase(response);
	}
}
