import { PublicKey } from "@solana/web3.js";
import { buildAndSignTransaction } from "./transaction";
import type { Stablecoin, SeizeArgs } from "./stablecoin";

export class Compliance {
  constructor(private readonly stablecoin: Stablecoin) {}

  async freeze(account: PublicKey): Promise<string> {
    const ix = this.stablecoin.getFreezeAccountInstruction(account);
    const sig = await buildAndSignTransaction(
      this.stablecoin.client.connection,
      this.stablecoin.client.wallet,
      [ix],
      true
    );
    if (typeof sig !== "string") throw new Error("Expected signature");
    return sig;
  }

  async thaw(account: PublicKey): Promise<string> {
    const ix = this.stablecoin.getThawAccountInstruction(account);
    const sig = await buildAndSignTransaction(
      this.stablecoin.client.connection,
      this.stablecoin.client.wallet,
      [ix],
      true
    );
    if (typeof sig !== "string") throw new Error("Expected signature");
    return sig;
  }

  async blacklistAdd(wallet: PublicKey, reason: string): Promise<string> {
    const ix = this.stablecoin.getAddToBlacklistInstruction(wallet, reason);
    const sig = await buildAndSignTransaction(
      this.stablecoin.client.connection,
      this.stablecoin.client.wallet,
      [ix],
      true
    );
    if (typeof sig !== "string") throw new Error("Expected signature");
    return sig;
  }

  async blacklistRemove(wallet: PublicKey): Promise<string> {
    const ix = this.stablecoin.getRemoveFromBlacklistInstruction(wallet);
    const sig = await buildAndSignTransaction(
      this.stablecoin.client.connection,
      this.stablecoin.client.wallet,
      [ix],
      true
    );
    if (typeof sig !== "string") throw new Error("Expected signature");
    return sig;
  }

  async seize(args: SeizeArgs): Promise<string> {
    const ix = this.stablecoin.getSeizeInstruction(args);
    const sig = await buildAndSignTransaction(
      this.stablecoin.client.connection,
      this.stablecoin.client.wallet,
      [ix],
      true
    );
    if (typeof sig !== "string") throw new Error("Expected signature");
    return sig;
  }
}
