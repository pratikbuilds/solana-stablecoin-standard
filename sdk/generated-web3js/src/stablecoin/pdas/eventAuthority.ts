import { PublicKey } from "@solana/web3.js";
import { STABLECOIN_PROGRAM_ID } from "..";

export function findEventAuthorityPda(
  programId: PublicKey = STABLECOIN_PROGRAM_ID,
): [PublicKey, number] {
  const seedsBuffer: Buffer[] = [Buffer.from("__event_authority", "utf8")];
  return PublicKey.findProgramAddressSync(seedsBuffer, programId);
}
