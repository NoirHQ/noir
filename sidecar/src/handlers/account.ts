import { FastifyRequest } from "fastify";
import { AccountService } from "../services";
import { AccountResponse } from "../types";
import { QueryAccountRequest } from "cosmjs-types/cosmos/auth/v1beta1/query";

export class AccountHandler {
	accountService: AccountService;

	constructor(accountService: AccountService) {
		this.accountService = accountService;
	}

	handleGetAccount = async (
		request: FastifyRequest<{
			Params: QueryAccountRequest;
		}>
	): Promise<AccountResponse> => {
		const { address } = request.params;
		return this.accountService.accounts(address);
	}
}
