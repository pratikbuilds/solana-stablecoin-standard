import {
  AccountMeta,
  Keypair,
  PublicKey,
  TransactionInstruction,
} from "@solana/web3.js";
import { STABLECOIN_PROGRAM_ID } from "..";
import {
  fixCodecSize,
  getBytesCodec,
  getStructCodec,
  transformCodec,
} from "@solana/codecs";

export interface TransferAuthorityInstructionAccounts {
  authority: PublicKey;
  config: PublicKey;
  roleConfig: PublicKey;
}

export interface TransferAuthorityInstructionArgs {
  newAuthority: PublicKey;
}

const TransferAuthorityInstructionDataCodec = getStructCodec([
  [
    "newAuthority",
    transformCodec(
      fixCodecSize(getBytesCodec(), 32),
      (value: PublicKey) => value.toBytes(),
      (value) => new PublicKey(value),
    ),
  ],
]);

export function createTransferAuthorityInstruction(
  accounts: TransferAuthorityInstructionAccounts,
  args: TransferAuthorityInstructionArgs,
  programId: PublicKey = STABLECOIN_PROGRAM_ID,
): TransactionInstruction {
  const keys: AccountMeta[] = [
    { pubkey: accounts.authority, isSigner: true, isWritable: false },
    { pubkey: accounts.config, isSigner: false, isWritable: true },
    { pubkey: accounts.roleConfig, isSigner: false, isWritable: true },
  ];
  const instructionData = Buffer.from(
    TransferAuthorityInstructionDataCodec.encode(args),
  );
  const discriminator = Buffer.from("30a94c48e5b437a1", "hex");
  const data = Buffer.concat([discriminator, instructionData]);

  return new TransactionInstruction({ keys, programId, data });
}
