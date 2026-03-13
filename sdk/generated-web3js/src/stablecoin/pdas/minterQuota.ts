import { PublicKey } from "@solana/web3.js";
import { STABLECOIN_PROGRAM_ID } from "..";

export interface MinterQuotaPdaSeeds {
  mint: PublicKey;
  minter: PublicKey;
}

export function findMinterQuotaPda(
  seeds: MinterQuotaPdaSeeds,
  programId: PublicKey = STABLECOIN_PROGRAM_ID,
): [PublicKey, number] {
  const seedsBuffer: Buffer[] = [
    Buffer.from("minter", "utf8"),
    seeds.mint.toBuffer(),
    seeds.minter.toBuffer(),
  ];
  return PublicKey.findProgramAddressSync(seedsBuffer, programId);
}
