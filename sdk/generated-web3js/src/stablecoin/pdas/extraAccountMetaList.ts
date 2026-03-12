import { PublicKey } from "@solana/web3.js";
import { STABLECOIN_PROGRAM_ID } from "..";

export interface ExtraAccountMetaListPdaSeeds {
  mint: PublicKey;
}

export function findExtraAccountMetaListPda(
  seeds: ExtraAccountMetaListPdaSeeds,
  programId: PublicKey = STABLECOIN_PROGRAM_ID,
): [PublicKey, number] {
  const seedsBuffer: Buffer[] = [
    Buffer.from("extra-account-metas", "utf8"),
    seeds.mint.toBuffer(),
  ];
  return PublicKey.findProgramAddressSync(seedsBuffer, programId);
}
