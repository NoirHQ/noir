import { FastifyRequest } from "fastify";
import { DistributionService } from "../services";
import { toSnakeCase } from "../utils";
import {
	QueryDelegationTotalRewardsRequest,
	QueryDelegationTotalRewardsResponse
} from "cosmjs-types/cosmos/distribution/v1beta1/query";

export class DistributionHandler {
	distributionService: DistributionService;

	constructor(distributionService: DistributionService) {
		this.distributionService = distributionService;
	}

	handleGetDistribution = (
		request: FastifyRequest<{
			Params: QueryDelegationTotalRewardsRequest;
		}>
	): unknown => {
		const { delegatorAddress } = request.params;
		const response = QueryDelegationTotalRewardsResponse.toJSON(
			this.distributionService
				.rewards(delegatorAddress)
		);
		return toSnakeCase(response);
	}
}
