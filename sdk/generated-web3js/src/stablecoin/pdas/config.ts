import { PublicKey } from "@solana/web3.js";
import { STABLECOIN_PROGRAM_ID } from "..";

export interface ConfigPdaSeeds {
  mint: PublicKey;
}

export function findConfigPda(
  seeds: ConfigPdaSeeds,
  programId: PublicKey = STABLECOIN_PROGRAM_ID,
): [PublicKey, number] {
  const seedsBuffer: Buffer[] = [
    Buffer.from("config", "utf8"),
    seeds.mint.toBuffer(),
  ];
  return PublicKey.findProgramAddressSync(seedsBuffer, programId);
}
