import { TxService } from "../services";
import { BroadcastTxSyncResponse, ResultTxSearch } from "../types";
import querystring from "querystring";

export class TxRpcHandler {
    txService: TxService;

    constructor(txService: TxService) {
        this.txService = txService;
    }

    broadcastTxSync = async ({ tx }): Promise<BroadcastTxSyncResponse> => {
        const result = await this.txService.broadcastTx(tx);
        const { code, txhash, data, rawLog, codespace } = result.txResponse;
        return {
            code,
            data,
            log: rawLog,
            codespace,
            hash: txhash,
        };
    }

    txSearch = ({ query }): ResultTxSearch => {
        const args = querystring.parse(query);
        let hash = args['tx.hash'] as string;
        if (hash.includes("'")) {
            hash = hash.replace(/'/gi, '');
        }
        const txs = this.txService.searchTx(hash);
        txs.forEach(tx => {
            tx.tx_result.events.forEach(event => {
                event.attributes = this.txService.encodeAttributes(event.attributes, 'utf8', 'base64');
            });
        });

        return {
            txs,
            total_count: txs.length.toString(),
        };
    }
}