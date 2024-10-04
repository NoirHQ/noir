import { StatusService } from "../services";
import { toSnakeCase } from "../utils";

export class StatusRpcHandler {
    statusService: StatusService;

    constructor(statusService: StatusService) {
        this.statusService = statusService;
    }

    async status() {
        return toSnakeCase(
            await this.statusService.status()
        );
    }
}