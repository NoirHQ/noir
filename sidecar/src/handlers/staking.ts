import { FastifyRequest } from "fastify";
import {
	QueryDelegatorDelegationsRequest,
	QueryDelegatorDelegationsResponse,
	QueryDelegatorUnbondingDelegationsRequest,
	QueryDelegatorUnbondingDelegationsResponse
} from "cosmjs-types/cosmos/staking/v1beta1/query";
import { StakingService } from "../services";
import { toSnakeCase } from "../utils";

export class StakingHandler {
	stakingService: StakingService;

	constructor(stakingService: StakingService) {
		this.stakingService = stakingService;
	}

	handleGetStaking = (
		request: FastifyRequest<{
			Params: QueryDelegatorDelegationsRequest;
		}>
	): unknown => {
		const { delegatorAddr } = request.params;
		const response = QueryDelegatorDelegationsResponse.toJSON(
			this.stakingService.delegations(delegatorAddr)
		);
		return toSnakeCase(response);
	}

	handleGetUnbondingDelegations = (
		request: FastifyRequest<{
			Params: QueryDelegatorUnbondingDelegationsRequest;
		}>
	): unknown => {
		const { delegatorAddr } = request.params;
		const response = QueryDelegatorUnbondingDelegationsResponse.toJSON(
			this.stakingService.unbondingDelegations(delegatorAddr)
		);
		return toSnakeCase(response);
	}
}
