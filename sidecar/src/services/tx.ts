import { ApiPromise } from "@pinot/api";
import { ResultTx, ResultTxSearch } from "../types/index.js";
import { ApiService } from "./service.js";
import { Database } from "lmdb";
import { Tx } from "cosmjs-types/cosmos/tx/v1beta1/tx.js";
import Weights from "../constants/weights.js";
import {
  BroadcastTxResponse,
  SimulateResponse,
} from "cosmjs-types/cosmos/tx/v1beta1/service.js";
import Long from "long";

type TransactResult = { code: number; gasUsed: number };

export class TxService implements ApiService {
  chainApi: ApiPromise;
  db: Database;

  constructor(db: Database, chainApi: ApiPromise) {
    this.chainApi = chainApi;
    this.db = db;
  }

  public async broadcastTx(txBytes: string): Promise<BroadcastTxResponse> {
    const hexTx = `0x${Buffer.from(txBytes, "base64").toString("hex")}`;
    const res = await this.chainApi.rpc["cosm"]["broadcastTx"](hexTx);
    let txhash = res.toString();
    if (txhash.startsWith("0x")) {
      txhash = txhash.substring(2);
    }

    await this.db.put(`tx::origin::${txhash.toLowerCase()}`, txBytes);
    return {
      txResponse: {
        height: Long.ZERO,
        txhash: txhash.toUpperCase(),
        codespace: "",
        code: 0,
        data: "",
        rawLog: "",
        logs: [],
        info: "",
        gasWanted: Long.ZERO,
        gasUsed: Long.ZERO,
        tx: {
          typeUrl: "",
          value: new Uint8Array(),
        },
        timestamp: "",
        events: [],
      },
    };
  }

  public searchTx(hash: string): ResultTxSearch {
    const resultTx = this.db.get(`tx::result::${hash.toLowerCase()}`);
    const txs: ResultTx[] = [];
    if (resultTx) {
      txs.push(resultTx);
    }
    return {
      txs,
      total_count: txs.length,
    };
  }

  public async saveTransactResult(
    tx: any,
    extrinsicIndex: number,
    header: any
  ): Promise<void> {
    let txHash = tx.hash;
    if (txHash.startsWith("0x")) {
      txHash = txHash.substring(2);
    }
    const rawTx = this.db.get(`tx::origin::${txHash.toLowerCase()}`);
    const { code, gasUsed } = await this.checkResult(header, extrinsicIndex);
    const txResult: ResultTx = {
      hash: txHash.toUpperCase(),
      height: header.number.toString(),
      index: extrinsicIndex,
      tx_result: {
        code,
        data: "",
        log: "",
        info: "",
        gas_wanted: tx.authInfo.fee.gasLimit,
        gas_used: gasUsed,
        events: [],
        codespace: "",
      },
      tx: rawTx,
    };
    await this.db.put(`tx::result::${txHash.toLowerCase()}`, txResult);
  }

  async checkResult(
    header: any,
    extrinsicIndex: number
  ): Promise<TransactResult> {
	  const events = (await (
		  await this.chainApi.at(header.hash)
    ).query.system.events()) as any;
    const result = events
      .filter(({ event: { section, method }, phase }) => {
        const { applyExtrinsic } = JSON.parse(phase.toString());
        return (
          applyExtrinsic === extrinsicIndex &&
          (`${section}::${method}` === "cosmos::Executed" ||
            `${section}::${method}` === "system::ExtrinsicFailed")
        );
      })
      .map(({ event: { data, section, method } }) => {
        if (`${section}::${method}` === "cosmos::Executed") {
          const result = JSON.parse(data.toString());
          const code = result[0];
          const { refTime } = result[1];
          return { code, gasUsed: refTime };
        } else {
          const { error } = JSON.parse(data.toString())[0]["module"];
          const { refTime } = JSON.parse(data.toString())[1]["weight"];
          let code = error;
          if (code.startsWith("0x")) {
            code = code.substring(2);
          }
          code = Buffer.from(code, "hex").readUint32LE();
          return { code, gasUsed: refTime };
        }
      });
    return result[0];
  }

  public simulateTx(txBytes: string): SimulateResponse {
    const {
      body,
      authInfo: {
        fee: { gasLimit },
      },
    } = Tx.decode(Buffer.from(txBytes, "base64"));
    const { messages } = body;

    let gasUsed = 0;
    // Multi message type is not supported yet.
    if (messages[0].typeUrl === "/cosmos.bank.v1beta1.MsgSend") {
      gasUsed += Weights.MsgSend;
    }

    return {
      gasInfo: {
        gasWanted: gasLimit,
        gasUsed: Long.fromNumber(gasUsed),
      },
      result: {
        data: new Uint8Array(),
        log: "",
        events: [],
        msgResponses: [],
      },
    };
  }
}
