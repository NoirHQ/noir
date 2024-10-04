import { rpc } from "../constants/rpc";
import { types } from "../constants/types";
import { sleep } from "../utils";
import { ApiService } from "./service";
import { ApiPromise, WsProvider } from "@polkadot/api";

export class ChainService implements ApiService {
    endpoint: string;
    chainApi: ApiPromise;

    constructor(endpoint: string) {
        this.endpoint = endpoint;

        ApiPromise.create({
            provider: new WsProvider(this.endpoint),
            types,
            rpc
        }).then((chainApi: ApiPromise) => this.chainApi = chainApi);
    }

    public async getChainApi(): Promise<ApiPromise> {
        while (!this.chainApi || !this.chainApi.isConnected) {
            console.debug(`Try connecting to chain RPC. endpoint: ${this.endpoint}`);

            this.chainApi = await ApiPromise.create({
                provider: new WsProvider(this.endpoint),
                types,
                rpc
            });

            await sleep(1000);
        }
        console.debug('Chain RPC connected');

        return this.chainApi;
    }
}
