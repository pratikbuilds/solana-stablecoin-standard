export type ProgramName = "stablecoin" | "transfer-hook";

export interface ProgramArtifact {
  name: ProgramName;
  packagePath: string;
}

export const PROGRAM_ARTIFACTS: ProgramArtifact[] = [
  {
    name: "stablecoin",
    packagePath: "programs/stablecoin",
  },
  {
    name: "transfer-hook",
    packagePath: "programs/transfer-hook",
  },
];

