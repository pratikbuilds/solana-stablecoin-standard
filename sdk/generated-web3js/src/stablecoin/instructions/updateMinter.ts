import {
  AccountMeta,
  Keypair,
  PublicKey,
  TransactionInstruction,
} from "@solana/web3.js";
import { STABLECOIN_PROGRAM_ID } from "..";
import { findConfigPda } from "../pdas/config";
import { findMinterQuotaPda } from "../pdas/minterQuota";
import { findRoleConfigPda } from "../pdas/roleConfig";
import {
  fixCodecSize,
  getBooleanCodec,
  getBytesCodec,
  getStructCodec,
  getU64Codec,
  transformCodec,
} from "@solana/codecs";

export interface UpdateMinterInstructionAccounts {
  authority: PublicKey;
  config?: PublicKey;
  roleConfig?: PublicKey;
  mint: PublicKey;
  minter: PublicKey;
  minterQuota?: PublicKey;
  systemProgram: PublicKey;
}

export interface UpdateMinterInstructionArgs {
  minter: PublicKey;
  quota: bigint;
  active: boolean;
}

const UpdateMinterInstructionDataCodec = getStructCodec([
  [
    "minter",
    transformCodec(
      fixCodecSize(getBytesCodec(), 32),
      (value: PublicKey) => value.toBytes(),
      (value) => new PublicKey(value),
    ),
  ],
  ["quota", getU64Codec()],
  ["active", getBooleanCodec()],
]);

export function createUpdateMinterInstruction(
  accounts: UpdateMinterInstructionAccounts,
  args: UpdateMinterInstructionArgs,
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
  let minterQuota = accounts.minterQuota;
  if (!minterQuota) {
    const [derived] = findMinterQuotaPda(
      {
        mint: accounts.mint,
        minter: accounts.minter,
      },
      programId,
    );
    minterQuota = derived;
  }
  const keys: AccountMeta[] = [
    { pubkey: accounts.authority, isSigner: true, isWritable: true },
    { pubkey: config, isSigner: false, isWritable: false },
    { pubkey: roleConfig, isSigner: false, isWritable: false },
    { pubkey: accounts.mint, isSigner: false, isWritable: false },
    { pubkey: accounts.minter, isSigner: false, isWritable: false },
    { pubkey: minterQuota, isSigner: false, isWritable: true },
    { pubkey: accounts.systemProgram, isSigner: false, isWritable: false },
  ];
  const instructionData = Buffer.from(
    UpdateMinterInstructionDataCodec.encode(args),
  );
  const discriminator = Buffer.from("a481a4584b1d5b26", "hex");
  const data = Buffer.concat([discriminator, instructionData]);

  return new TransactionInstruction({ keys, programId, data });
}
