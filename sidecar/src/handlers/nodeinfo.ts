import { NodeInfoService } from "../services";
import { GetNodeInfoResponse } from "cosmjs-types/cosmos/base/tendermint/v1beta1/query";
import { toSnakeCase } from "../utils";

export class NodeInfoHandler {
	nodeInfoService: NodeInfoService;

	constructor(nodeInfoService: NodeInfoService) {
		this.nodeInfoService = nodeInfoService;
	}

	handleGetNodeInfo = async (): Promise<GetNodeInfoResponse> => {
		const response = GetNodeInfoResponse.toJSON(
			await this.nodeInfoService.nodeInfo()
		);
		return toSnakeCase(response);
	}
}
