import {
  AccountMeta,
  Keypair,
  PublicKey,
  TransactionInstruction,
} from "@solana/web3.js";
import { STABLECOIN_PROGRAM_ID } from "..";
import { findEventAuthorityPda } from "../pdas/eventAuthority";

export interface RemoveFromBlacklistInstructionAccounts {
  authority: PublicKey;
  config: PublicKey;
  roleConfig: PublicKey;
  blacklistEntry: PublicKey;
  eventAuthority?: PublicKey;
  program: PublicKey;
}

export function createRemoveFromBlacklistInstruction(
  accounts: RemoveFromBlacklistInstructionAccounts,
  programId: PublicKey = STABLECOIN_PROGRAM_ID,
): TransactionInstruction {
  let eventAuthority = accounts.eventAuthority;
  if (!eventAuthority) {
    const [derived] = findEventAuthorityPda(programId);
    eventAuthority = derived;
  }
  const keys: AccountMeta[] = [
    { pubkey: accounts.authority, isSigner: true, isWritable: true },
    { pubkey: accounts.config, isSigner: false, isWritable: false },
    { pubkey: accounts.roleConfig, isSigner: false, isWritable: false },
    { pubkey: accounts.blacklistEntry, isSigner: false, isWritable: true },
    { pubkey: eventAuthority, isSigner: false, isWritable: false },
    { pubkey: accounts.program, isSigner: false, isWritable: false },
  ];
  const data = Buffer.from("2f69140aa5a8cbdb", "hex");

  return new TransactionInstruction({ keys, programId, data });
}
