import {
  AccountMeta,
  Keypair,
  PublicKey,
  TransactionInstruction,
} from "@solana/web3.js";
import { STABLECOIN_PROGRAM_ID } from "..";

export interface RemoveFromBlacklistInstructionAccounts {
  authority: PublicKey;
  config: PublicKey;
  roleConfig: PublicKey;
  blacklistEntry: PublicKey;
}

export function createRemoveFromBlacklistInstruction(
  accounts: RemoveFromBlacklistInstructionAccounts,
  programId: PublicKey = STABLECOIN_PROGRAM_ID,
): TransactionInstruction {
  const keys: AccountMeta[] = [
    { pubkey: accounts.authority, isSigner: true, isWritable: true },
    { pubkey: accounts.config, isSigner: false, isWritable: false },
    { pubkey: accounts.roleConfig, isSigner: false, isWritable: false },
    { pubkey: accounts.blacklistEntry, isSigner: false, isWritable: true },
  ];
  const data = Buffer.from("2f69140aa5a8cbdb", "hex");

  return new TransactionInstruction({ keys, programId, data });
}
