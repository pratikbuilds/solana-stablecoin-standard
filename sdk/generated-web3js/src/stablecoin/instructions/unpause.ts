import {
  AccountMeta,
  Keypair,
  PublicKey,
  TransactionInstruction,
} from "@solana/web3.js";
import { STABLECOIN_PROGRAM_ID } from "..";
import { findEventAuthorityPda } from "../pdas/eventAuthority";

export interface UnpauseInstructionAccounts {
  authority: PublicKey;
  config: PublicKey;
  roleConfig: PublicKey;
  eventAuthority?: PublicKey;
  program: PublicKey;
}

export function createUnpauseInstruction(
  accounts: UnpauseInstructionAccounts,
  programId: PublicKey = STABLECOIN_PROGRAM_ID,
): TransactionInstruction {
  let eventAuthority = accounts.eventAuthority;
  if (!eventAuthority) {
    const [derived] = findEventAuthorityPda(programId);
    eventAuthority = derived;
  }
  const keys: AccountMeta[] = [
    { pubkey: accounts.authority, isSigner: true, isWritable: false },
    { pubkey: accounts.config, isSigner: false, isWritable: true },
    { pubkey: accounts.roleConfig, isSigner: false, isWritable: false },
    { pubkey: eventAuthority, isSigner: false, isWritable: false },
    { pubkey: accounts.program, isSigner: false, isWritable: false },
  ];
  const data = Buffer.from("a99004260a8dbcff", "hex");

  return new TransactionInstruction({ keys, programId, data });
}
