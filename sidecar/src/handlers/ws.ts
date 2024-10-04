import { JSONRPCServer } from "json-rpc-2.0";
import { SocketStream } from "@fastify/websocket";

export class WebsocketHandler {
    jsonrpc: JSONRPCServer;

    constructor(jsonrpc: JSONRPCServer) {
        this.jsonrpc = jsonrpc;
    }

    handlerMessage = (connection: SocketStream) => {
        connection.socket.on('message', async (message) => {
            const request = JSON.parse(message.toString());
            const response = await this.jsonrpc.receive(request);
            if (response) {
                connection.socket.send(
                    Buffer.from(JSON.stringify(response), 'utf8')
                );
            }
        });
    }
}
