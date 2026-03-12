import {
  AccountMeta,
  Keypair,
  PublicKey,
  TransactionInstruction,
} from "@solana/web3.js";
import { STABLECOIN_PROGRAM_ID } from "..";
import { findConfigPda } from "../pdas/config";
import { findRoleConfigPda } from "../pdas/roleConfig";
import { getStructCodec, getU64Codec } from "@solana/codecs";

export interface BurnInstructionAccounts {
  authority: PublicKey;
  config?: PublicKey;
  roleConfig?: PublicKey;
  mint: PublicKey;
  from: PublicKey;
  tokenProgram: PublicKey;
}

export interface BurnInstructionArgs {
  amount: bigint;
}

const BurnInstructionDataCodec = getStructCodec([["amount", getU64Codec()]]);

export function createBurnInstruction(
  accounts: BurnInstructionAccounts,
  args: BurnInstructionArgs,
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
    { pubkey: config, isSigner: false, isWritable: true },
    { pubkey: roleConfig, isSigner: false, isWritable: false },
    { pubkey: accounts.mint, isSigner: false, isWritable: true },
    { pubkey: accounts.from, isSigner: false, isWritable: true },
    { pubkey: accounts.tokenProgram, isSigner: false, isWritable: false },
  ];
  const instructionData = Buffer.from(BurnInstructionDataCodec.encode(args));
  const discriminator = Buffer.from("746e1d386bdb2a5d", "hex");
  const data = Buffer.concat([discriminator, instructionData]);

  return new TransactionInstruction({ keys, programId, data });
}
