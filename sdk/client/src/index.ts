import { PROGRAM_ARTIFACTS } from "./constants";

export { StablecoinClient } from "./client";
export type { CreateParams, StablecoinClientOptions } from "./client";
export { Presets, PRESET_CONFIGS, DEFAULT_MINTER_QUOTA } from "./presets";
export type { PresetConfig } from "./presets";
export { Stablecoin } from "./stablecoin";
export type {
  MintArgs,
  BurnArgs,
  SeizeArgs,
  TransferArgs,
  UpdateRolesArgs,
  UpdateMinterArgs,
} from "./stablecoin";
export { Compliance } from "./compliance";
export { buildAndSignTransaction } from "./transaction";
export type { Wallet, StablecoinClientLike } from "./types";
export { findBlacklistEntryPda } from "./pdas";

// Re-export generated instruction creators for advanced users
import { stablecoin as generatedStablecoin } from "@stbr/sss-generated-web3js";
export const createMintInstruction = generatedStablecoin.createMintInstruction;
export const createBurnInstruction = generatedStablecoin.createBurnInstruction;
export const createPauseInstruction = generatedStablecoin.createPauseInstruction;
export const createUnpauseInstruction = generatedStablecoin.createUnpauseInstruction;
export const createFreezeAccountInstruction =
  generatedStablecoin.createFreezeAccountInstruction;
export const createThawAccountInstruction =
  generatedStablecoin.createThawAccountInstruction;
export const createAddToBlacklistInstruction =
  generatedStablecoin.createAddToBlacklistInstruction;
export const createRemoveFromBlacklistInstruction =
  generatedStablecoin.createRemoveFromBlacklistInstruction;
export const createSeizeInstruction = generatedStablecoin.createSeizeInstruction;
export const createTransferAuthorityInstruction =
  generatedStablecoin.createTransferAuthorityInstruction;
export const createUpdateRolesInstruction =
  generatedStablecoin.createUpdateRolesInstruction;
export const createUpdateMinterInstruction =
  generatedStablecoin.createUpdateMinterInstruction;
export const createInitializeInstruction =
  generatedStablecoin.createInitializeInstruction;

/** CLI compatibility: workspace status for sss-token status command */
export interface WorkspaceStatus {
  generatedPrograms: string[];
  readyForCodama: boolean;
}

export class SolanaStablecoinClient {
  getWorkspaceStatus(): WorkspaceStatus {
    return {
      generatedPrograms: PROGRAM_ARTIFACTS.map((a) => a.name),
      readyForCodama: true,
    };
  }
}
