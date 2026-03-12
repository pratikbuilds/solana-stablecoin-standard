import {
  AccountMeta,
  Keypair,
  PublicKey,
  TransactionInstruction,
} from "@solana/web3.js";
import { STABLECOIN_PROGRAM_ID } from "..";
import { findConfigPda } from "../pdas/config";
import { findRoleConfigPda } from "../pdas/roleConfig";

export interface ThawAccountInstructionAccounts {
  authority: PublicKey;
  config?: PublicKey;
  roleConfig?: PublicKey;
  mint: PublicKey;
  account: PublicKey;
  tokenProgram: PublicKey;
}

export function createThawAccountInstruction(
  accounts: ThawAccountInstructionAccounts,
  programId: PublicKey = STABLECOIN_PROGRAM_ID,
): TransactionInstruction {
  let config = accounts.config;
  if (!config) {
    const [derived] = findConfigPda(
      {
        mint: accounts.mint,
      },
      programId,
    );
    config = derived;
  }
  let roleConfig = accounts.roleConfig;
  if (!roleConfig) {
    const [derived] = findRoleConfigPda(
      {
        mint: accounts.mint,
      },
      programId,
    );
    roleConfig = derived;
  }
  const keys: AccountMeta[] = [
    { pubkey: accounts.authority, isSigner: true, isWritable: false },
    { pubkey: config, isSigner: false, isWritable: false },
    { pubkey: roleConfig, isSigner: false, isWritable: false },
    { pubkey: accounts.mint, isSigner: false, isWritable: true },
    { pubkey: accounts.account, isSigner: false, isWritable: true },
    { pubkey: accounts.tokenProgram, isSigner: false, isWritable: false },
  ];
  const data = Buffer.from("73984fd5d5a9b823", "hex");

  return new TransactionInstruction({ keys, programId, data });
}
