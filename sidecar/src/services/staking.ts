import Long from "long";
import { ApiService } from "./service";
import {
	QueryDelegatorDelegationsResponse,
	QueryDelegatorUnbondingDelegationsResponse,
} from "cosmjs-types/cosmos/staking/v1beta1/query.js";

export class StakingService implements ApiService {
	/* eslint-disable @typescript-eslint/no-unused-vars */
	public delegations(delegatorAddr: string): QueryDelegatorDelegationsResponse {
		return {
			delegationResponses: [],
			pagination: {
				nextKey: new Uint8Array(),
				total: Long.ZERO,
			},
		};
	}

	/* eslint-disable @typescript-eslint/no-unused-vars */
	public unbondingDelegations(
		delegatorAddr: string
	): QueryDelegatorUnbondingDelegationsResponse {
		return {
			unbondingResponses: [],
			pagination: {
				nextKey: new Uint8Array(),
				total: Long.ZERO,
			},
		};
	}
}
