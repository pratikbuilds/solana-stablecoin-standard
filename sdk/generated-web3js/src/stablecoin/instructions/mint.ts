import {
  AccountMeta,
  Keypair,
  PublicKey,
  TransactionInstruction,
} from "@solana/web3.js";
import { STABLECOIN_PROGRAM_ID } from "..";
import { findConfigPda } from "../pdas/config";
import { findEventAuthorityPda } from "../pdas/eventAuthority";
import { findMinterQuotaPda } from "../pdas/minterQuota";
import { getStructCodec, getU64Codec } from "@solana/codecs";

export interface MintInstructionAccounts {
  authority: PublicKey;
  config?: PublicKey;
  minterQuota?: PublicKey;
  mint: PublicKey;
  to: PublicKey;
  tokenProgram: PublicKey;
  eventAuthority?: PublicKey;
  program: PublicKey;
}

export interface MintInstructionArgs {
  amount: bigint;
}

const MintInstructionDataCodec = getStructCodec([["amount", getU64Codec()]]);

export function createMintInstruction(
  accounts: MintInstructionAccounts,
  args: MintInstructionArgs,
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
  let minterQuota = accounts.minterQuota;
  if (!minterQuota) {
    const [derived] = findMinterQuotaPda(
      {
        mint: accounts.mint,
        authority: accounts.authority,
      },
      programId,
    );
    minterQuota = derived;
  }
  let eventAuthority = accounts.eventAuthority;
  if (!eventAuthority) {
    const [derived] = findEventAuthorityPda(programId);
    eventAuthority = derived;
  }
  const keys: AccountMeta[] = [
    { pubkey: accounts.authority, isSigner: true, isWritable: false },
    { pubkey: config, isSigner: false, isWritable: true },
    { pubkey: minterQuota, isSigner: false, isWritable: true },
    { pubkey: accounts.mint, isSigner: false, isWritable: true },
    { pubkey: accounts.to, isSigner: false, isWritable: true },
    { pubkey: accounts.tokenProgram, isSigner: false, isWritable: false },
    { pubkey: eventAuthority, isSigner: false, isWritable: false },
    { pubkey: accounts.program, isSigner: false, isWritable: false },
  ];
  const instructionData = Buffer.from(MintInstructionDataCodec.encode(args));
  const discriminator = Buffer.from("3339e12fb69289a6", "hex");
  const data = Buffer.concat([discriminator, instructionData]);

  return new TransactionInstruction({ keys, programId, data });
}
