import { PublicKey } from "@solana/web3.js";

export const TRANSFERHOOK_PROGRAM_ID = new PublicKey(
  "6mjTtZjRFK8FWA24f2KNEfMVcAvpYLWcpMzLvKiVXyd2",
);

export * from "./instructions/initializeExtraAccountMetaList";
export * from "./instructions/transferHook";
