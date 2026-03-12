import {
  AccountMeta,
  Keypair,
  PublicKey,
  TransactionInstruction,
} from "@solana/web3.js";
import { STABLECOIN_PROGRAM_ID } from "..";

export interface UnpauseInstructionAccounts {
  authority: PublicKey;
  config: PublicKey;
  roleConfig: PublicKey;
}

export function createUnpauseInstruction(
  accounts: UnpauseInstructionAccounts,
  programId: PublicKey = STABLECOIN_PROGRAM_ID,
): TransactionInstruction {
  const keys: AccountMeta[] = [
    { pubkey: accounts.authority, isSigner: true, isWritable: false },
    { pubkey: accounts.config, isSigner: false, isWritable: true },
    { pubkey: accounts.roleConfig, isSigner: false, isWritable: false },
  ];
  const data = Buffer.from("a99004260a8dbcff", "hex");

  return new TransactionInstruction({ keys, programId, data });
}
