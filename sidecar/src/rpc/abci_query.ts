import { ABCIQueryResponse } from "cosmjs-types/cosmos/base/tendermint/v1beta1/query";
import { AbciService } from "../services";

export class AbciRpcHandler {
    abciService: AbciService;

    constructor(abciService: AbciService) {
        this.abciService = abciService;
    }

    abciQuery = async ({ path, data }): Promise<unknown> => {
        const result = await this.abciService.query(path, data);
        const response = ABCIQueryResponse.toJSON(result);
        return {
            response,
        };
    }
}