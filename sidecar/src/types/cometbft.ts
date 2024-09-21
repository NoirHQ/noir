export interface ResultTxSearch {
	txs: ResultTx[];
	total_count: string;
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
	gas_wanted: string;
	gas_used: string;
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
