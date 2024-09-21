import { QueryDelegationTotalRewardsResponse } from "cosmjs-types/cosmos/distribution/v1beta1/query.js";
import { ApiService } from "./service";

export class DistributionService implements ApiService {
	public rewards(
		delegatorAddress: string
	): QueryDelegationTotalRewardsResponse {
		return {
			rewards: [],
			total: [],
		};
	}
}
