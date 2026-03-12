import { PublicKey } from "@solana/web3.js";
import { STABLECOIN_PROGRAM_ID } from "..";

export interface RoleConfigPdaSeeds {
  mint: PublicKey;
}

export function findRoleConfigPda(
  seeds: RoleConfigPdaSeeds,
  programId: PublicKey = STABLECOIN_PROGRAM_ID,
): [PublicKey, number] {
  const seedsBuffer: Buffer[] = [
    Buffer.from("roles", "utf8"),
    seeds.mint.toBuffer(),
  ];
  return PublicKey.findProgramAddressSync(seedsBuffer, programId);
}
