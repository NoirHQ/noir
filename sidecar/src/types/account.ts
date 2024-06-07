import Dummy from "../constants/dummy.js";

export class AccountResponse {
  account: BaseAccount;

  constructor(account: BaseAccount) {
    this.account = account;
  }

  public isInterim(): boolean {
    return this.account.pub_key.key === Dummy.Secp256k1PublicKey;
  }
}

export interface BaseAccount {
  "@type": string;
  address: string;
  pub_key: PublicKey | null;
  account_number: string;
  sequence: string;
}

export interface PublicKey {
  "@type": string;
  key: string;
}
