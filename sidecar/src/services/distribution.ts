import { QueryDelegationTotalRewardsResponse } from "cosmjs-types/cosmos/distribution/v1beta1/query.js";
import { ApiService } from "./service";

export class DistributionService implements ApiService {
	/* eslint-disable @typescript-eslint/no-unused-vars */
	public rewards(
		delegatorAddress: string
	): QueryDelegationTotalRewardsResponse {
		return {
			rewards: [],
			total: [],
		};
	}
}
