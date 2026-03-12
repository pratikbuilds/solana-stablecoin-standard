import {
  AnchorProvider,
  Program,
  Wallet,
  type Idl,
} from "@coral-xyz/anchor";
import { Connection, PublicKey, clusterApiUrl, type Keypair } from "@solana/web3.js";

import type { Stablecoin } from "../../../target/types/stablecoin";
import type { TransferHook } from "../../../target/types/transfer_hook";
import {
  loadProgramIds,
  loadStablecoinIdl,
  loadTransferHookIdl,
} from "../config";

export const CONFIRM_COMMITMENT = "confirmed" as const;

export interface DevnetPrograms {
  connection: Connection;
  provider: AnchorProvider;
  stablecoinProgram: Program<Stablecoin>;
  transferHookProgram: Program<TransferHook>;
  stablecoinProgramId: PublicKey;
  transferHookProgramId: PublicKey;
}

export function devnetConnection(): Connection {
  return new Connection(
    process.env.SOLANA_RPC_URL ?? clusterApiUrl("devnet"),
    CONFIRM_COMMITMENT,
  );
}

export function createProvider(payer: Keypair): AnchorProvider {
  return new AnchorProvider(
    devnetConnection(),
    new Wallet(payer),
    {
      commitment: CONFIRM_COMMITMENT,
      preflightCommitment: CONFIRM_COMMITMENT,
    },
  );
}

export function loadPrograms(payer: Keypair): DevnetPrograms {
  const provider = createProvider(payer);
  const ids = loadProgramIds();
  const stablecoinProgramId = new PublicKey(ids.stablecoinProgramId);
  const transferHookProgramId = new PublicKey(ids.transferHookProgramId);
  const stablecoinIdl = loadStablecoinIdl() as Idl;
  const transferHookIdl = loadTransferHookIdl() as Idl;

  if (stablecoinIdl.address !== ids.stablecoinProgramId) {
    throw new Error(
      `Stablecoin IDL address ${stablecoinIdl.address} does not match deployed fixture ${ids.stablecoinProgramId}`,
    );
  }
  if (transferHookIdl.address !== ids.transferHookProgramId) {
    throw new Error(
      `Transfer-hook IDL address ${transferHookIdl.address} does not match deployed fixture ${ids.transferHookProgramId}`,
    );
  }

  return {
    connection: provider.connection,
    provider,
    stablecoinProgram: new Program<Stablecoin>(stablecoinIdl, provider),
    transferHookProgram: new Program<TransferHook>(transferHookIdl, provider),
    stablecoinProgramId,
    transferHookProgramId,
  };
}
