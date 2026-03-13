import {
  AccountMeta,
  Keypair,
  PublicKey,
  TransactionInstruction,
} from "@solana/web3.js";
import { STABLECOIN_PROGRAM_ID } from "..";
import { findEventAuthorityPda } from "../pdas/eventAuthority";
import {
  fixCodecSize,
  getBytesCodec,
  getOptionCodec,
  getStructCodec,
  transformCodec,
} from "@solana/codecs";

export interface UpdateRolesInstructionAccounts {
  authority: PublicKey;
  config: PublicKey;
  roleConfig: PublicKey;
  eventAuthority?: PublicKey;
  program: PublicKey;
}

export interface UpdateRolesInstructionArgs {
  pauser: PublicKey | null;
  burner: PublicKey | null;
  blacklister: PublicKey | null;
  seizer: PublicKey | null;
}

const UpdateRolesInstructionDataCodec = getStructCodec([
  [
    "pauser",
    getOptionCodec(
      transformCodec(
        fixCodecSize(getBytesCodec(), 32),
        (value: PublicKey) => value.toBytes(),
        (value) => new PublicKey(value),
      ),
    ),
  ],
  [
    "burner",
    getOptionCodec(
      transformCodec(
        fixCodecSize(getBytesCodec(), 32),
        (value: PublicKey) => value.toBytes(),
        (value) => new PublicKey(value),
      ),
    ),
  ],
  [
    "blacklister",
    getOptionCodec(
      transformCodec(
        fixCodecSize(getBytesCodec(), 32),
        (value: PublicKey) => value.toBytes(),
        (value) => new PublicKey(value),
      ),
    ),
  ],
  [
    "seizer",
    getOptionCodec(
      transformCodec(
        fixCodecSize(getBytesCodec(), 32),
        (value: PublicKey) => value.toBytes(),
        (value) => new PublicKey(value),
      ),
    ),
  ],
]);

export function createUpdateRolesInstruction(
  accounts: UpdateRolesInstructionAccounts,
  args: UpdateRolesInstructionArgs,
  programId: PublicKey = STABLECOIN_PROGRAM_ID,
): TransactionInstruction {
  let eventAuthority = accounts.eventAuthority;
  if (!eventAuthority) {
    const [derived] = findEventAuthorityPda(programId);
    eventAuthority = derived;
  }
  const keys: AccountMeta[] = [
    { pubkey: accounts.authority, isSigner: true, isWritable: false },
    { pubkey: accounts.config, isSigner: false, isWritable: false },
    { pubkey: accounts.roleConfig, isSigner: false, isWritable: true },
    { pubkey: eventAuthority, isSigner: false, isWritable: false },
    { pubkey: accounts.program, isSigner: false, isWritable: false },
  ];
  const instructionData = Buffer.from(
    UpdateRolesInstructionDataCodec.encode(args),
  );
  const discriminator = Buffer.from("dc98cde9b17bdb7d", "hex");
  const data = Buffer.concat([discriminator, instructionData]);

  return new TransactionInstruction({ keys, programId, data });
}
