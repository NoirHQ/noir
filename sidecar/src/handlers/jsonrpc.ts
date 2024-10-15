import { FastifyRequest } from "fastify";
import { JSONRPCRequest, JSONRPCResponse, JSONRPCServer } from "json-rpc-2.0";

export class JsonRpcHandler {
    jsonrpc: JSONRPCServer;

    constructor(jsonrpc: JSONRPCServer) {
        this.jsonrpc = jsonrpc;
    }

    handleRequest = async (request: FastifyRequest<{
        Body: JSONRPCRequest;
    }>): Promise<JSONRPCResponse | null> => {
        return await this.jsonrpc.receive(request.body);
    }
}
