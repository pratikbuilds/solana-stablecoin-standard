import { PROGRAM_ARTIFACTS } from "@stbr/sss-generated";

export interface WorkspaceStatus {
  generatedPrograms: string[];
  readyForCodama: boolean;
}

export class SolanaStablecoinClient {
  getWorkspaceStatus(): WorkspaceStatus {
    return {
      generatedPrograms: PROGRAM_ARTIFACTS.map((artifact) => artifact.name),
      readyForCodama: true,
    };
  }
}

