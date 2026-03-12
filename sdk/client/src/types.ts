import type {
  Connection,
  PublicKey,
  Transaction,
  VersionedTransaction,
} from "@solana/web3.js";

export interface Wallet {
  publicKey: PublicKey;
  signTransaction<T extends Transaction | VersionedTransaction>(
    tx: T
  ): Promise<T>;
  signAllTransactions<T extends Transaction | VersionedTransaction>(
    txs: T[]
  ): Promise<T[]>;
}

/** Minimal client interface for Stablecoin to avoid circular imports. */
export interface StablecoinClientLike {
  connection: Connection;
  wallet: Wallet | null;
  stablecoinProgramId: PublicKey;
  transferHookProgramId?: PublicKey;
}
