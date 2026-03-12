import {
  Transaction,
  sendAndConfirmTransaction,
  type Connection,
  type Keypair,
  type Signer,
  type TransactionInstruction,
} from "@solana/web3.js";

import { CONFIRM_COMMITMENT } from "./cluster";

export async function sendInstructions(
  connection: Connection,
  payer: Keypair,
  instructions: TransactionInstruction[],
  signers: Signer[] = [],
): Promise<string> {
  const transaction = new Transaction().add(...instructions);
  return sendAndConfirmTransaction(connection, transaction, [payer, ...signers], {
    commitment: CONFIRM_COMMITMENT,
    preflightCommitment: CONFIRM_COMMITMENT,
  });
}
