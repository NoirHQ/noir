export class AccountResponse {
	account: BaseAccount;

	constructor(account: BaseAccount) {
		this.account = account;
	}
}

export interface BaseAccount {
	'@type': string;
	address: string;
	pub_key: PublicKey | null;
	account_number: string;
	sequence: string;
}

export interface PublicKey {
	'@type': string;
	key: string;
}
