export interface ResultTxSearch {
  txs: ResultTx[];
  total_count: number;
}

export interface ResultTx {
  hash: string;
  height: string;
  index: number;
  tx_result: ResponseDeliverTx;
  tx: string;
}

export interface ResponseDeliverTx {
  code: number;
  data: string;
  log: string;
  info: string;
  gas_wanted: number;
  gas_used: number;
  events: Event[];
  codespace: string;
}

export interface Event {
  type: string;
  attributes: EventAttribute[];
}

export interface EventAttribute {
  key: string;
  value: string;
  index: boolean;
}
