import {
  Connection,
  Keypair,
  TransactionInstruction,
  TransactionMessage,
  VersionedTransaction,
} from "@solana/web3.js";
import type { Wallet } from "./types";

export async function buildAndSignTransaction(
  connection: Connection,
  wallet: Wallet | null,
  instructions: TransactionInstruction[],
  signAndSend: boolean,
  extraSigners?: Keypair[]
): Promise<VersionedTransaction | string> {
  if (!wallet) {
    throw new Error("Wallet required");
  }
  const { blockhash, lastValidBlockHeight } =
    await connection.getLatestBlockhash();
  const message = new TransactionMessage({
    payerKey: wallet.publicKey,
    recentBlockhash: blockhash,
    instructions,
  }).compileToV0Message();
  const tx = new VersionedTransaction(message);
  if (signAndSend) {
    if (extraSigners && extraSigners.length > 0) {
      tx.sign(extraSigners);
    }
    const signed = await wallet.signTransaction(tx);
    const sig = await connection.sendRawTransaction(signed.serialize(), {
      skipPreflight: true,
      maxRetries: 3,
    });
    await connection.confirmTransaction(
      { signature: sig, blockhash, lastValidBlockHeight },
      "confirmed"
    );
    return sig;
  }
  return tx;
}
