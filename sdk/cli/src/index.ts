import { SolanaStablecoinClient } from "@stbr/sss-client";

export function runCli(argv: string[] = process.argv.slice(2)): number {
  const client = new SolanaStablecoinClient();

  if (argv[0] === "status") {
    console.log(JSON.stringify(client.getWorkspaceStatus(), null, 2));
    return 0;
  }

  console.log("sss-token workspace bootstrap is ready. Use `sss-token status` to inspect the scaffold.");
  return 0;
}

