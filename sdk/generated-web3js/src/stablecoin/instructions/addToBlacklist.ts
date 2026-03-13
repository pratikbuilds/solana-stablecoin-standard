import {
  AccountMeta,
  Keypair,
  PublicKey,
  TransactionInstruction,
} from "@solana/web3.js";
import { STABLECOIN_PROGRAM_ID } from "..";
import {
  addCodecSizePrefix,
  getStructCodec,
  getU32Codec,
  getUtf8Codec,
} from "@solana/codecs";
import { findEventAuthorityPda } from "../pdas/eventAuthority";

export interface AddToBlacklistInstructionAccounts {
  authority: PublicKey;
  config: PublicKey;
  roleConfig: PublicKey;
  wallet: PublicKey;
  blacklistEntry: PublicKey;
  systemProgram: PublicKey;
  eventAuthority?: PublicKey;
  program: PublicKey;
}

export interface AddToBlacklistInstructionArgs {
  reason: string;
}

const AddToBlacklistInstructionDataCodec = getStructCodec([
  ["reason", addCodecSizePrefix(getUtf8Codec(), getU32Codec())],
]);

export function createAddToBlacklistInstruction(
  accounts: AddToBlacklistInstructionAccounts,
  args: AddToBlacklistInstructionArgs,
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
    { pubkey: accounts.wallet, isSigner: false, isWritable: false },
    { pubkey: accounts.blacklistEntry, isSigner: false, isWritable: true },
    { pubkey: accounts.systemProgram, isSigner: false, isWritable: false },
    { pubkey: eventAuthority, isSigner: false, isWritable: false },
    { pubkey: accounts.program, isSigner: false, isWritable: false },
  ];
  const instructionData = Buffer.from(
    AddToBlacklistInstructionDataCodec.encode(args),
  );
  const discriminator = Buffer.from("5a7362e7ad7775b0", "hex");
  const data = Buffer.concat([discriminator, instructionData]);

  return new TransactionInstruction({ keys, programId, data });
}
