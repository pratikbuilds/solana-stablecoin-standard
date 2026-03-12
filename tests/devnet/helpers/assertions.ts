import assert from "node:assert/strict";

import {
  AccountState,
  getAccount,
  getDefaultAccountState,
  getMint,
  getMintCloseAuthority,
  getPermanentDelegate,
  getTransferHook,
  TOKEN_2022_PROGRAM_ID,
} from "@solana/spl-token";
import { PublicKey } from "@solana/web3.js";

import type { PresetContext } from "./presets";

export async function assertSss1Flags(ctx: PresetContext): Promise<void> {
  const [config, roleConfig, mint] = await Promise.all([
    ctx.programs.stablecoinProgram.account.stablecoinConfig.fetch(ctx.configPda),
    ctx.programs.stablecoinProgram.account.roleConfig.fetch(ctx.roleConfigPda),
    getMint(
      ctx.programs.connection,
      ctx.mint.publicKey,
      "confirmed",
      TOKEN_2022_PROGRAM_ID,
    ),
  ]);

  assert.equal(config.paused, false);
  assert.equal(config.enableTransferHook, false);
  assert.equal(config.enablePermanentDelegate, false);
  assert.equal(config.defaultAccountFrozen, false);
  assert.equal(roleConfig.blacklister.toBase58(), PublicKey.default.toBase58());
  assert.equal(roleConfig.seizer.toBase58(), PublicKey.default.toBase58());
  assert.equal(mint.mintAuthority?.toBase58(), ctx.configPda.toBase58());
  assert.equal(mint.freezeAuthority?.toBase58(), ctx.configPda.toBase58());
  assert.equal(getPermanentDelegate(mint), null);
  assert.equal(getTransferHook(mint), null);
  assert.equal(
    getMintCloseAuthority(mint)?.closeAuthority.toBase58(),
    ctx.configPda.toBase58(),
  );
}

export async function assertSss2Flags(ctx: PresetContext): Promise<void> {
  const [config, roleConfig, mint] = await Promise.all([
    ctx.programs.stablecoinProgram.account.stablecoinConfig.fetch(ctx.configPda),
    ctx.programs.stablecoinProgram.account.roleConfig.fetch(ctx.roleConfigPda),
    getMint(
      ctx.programs.connection,
      ctx.mint.publicKey,
      "confirmed",
      TOKEN_2022_PROGRAM_ID,
    ),
  ]);

  assert.equal(config.paused, false);
  assert.equal(config.enableTransferHook, true);
  assert.equal(config.enablePermanentDelegate, true);
  assert.equal(config.defaultAccountFrozen, true);
  assert.equal(roleConfig.blacklister.toBase58(), ctx.authority.publicKey.toBase58());
  assert.equal(roleConfig.seizer.toBase58(), ctx.authority.publicKey.toBase58());
  assert.equal(
    getPermanentDelegate(mint)?.delegate.toBase58(),
    ctx.configPda.toBase58(),
  );
  assert.equal(
    getTransferHook(mint)?.authority.toBase58(),
    ctx.configPda.toBase58(),
  );
  assert.equal(
    getTransferHook(mint)?.programId.toBase58(),
    ctx.programs.transferHookProgramId.toBase58(),
  );
  assert.equal(getDefaultAccountState(mint)?.state, AccountState.Frozen);
}

export async function assertSss1FlowResult(
  ctx: PresetContext,
  mintedAmount: bigint,
  transferredAmount: bigint,
): Promise<void> {
  const [config, userA, userB] = await Promise.all([
    ctx.programs.stablecoinProgram.account.stablecoinConfig.fetch(ctx.configPda),
    getAccount(ctx.programs.connection, ctx.userAAta, "confirmed", TOKEN_2022_PROGRAM_ID),
    getAccount(ctx.programs.connection, ctx.userBAta, "confirmed", TOKEN_2022_PROGRAM_ID),
  ]);

  assert.equal(config.totalMinted.toString(), mintedAmount.toString());
  assert.equal(userA.amount.toString(), (mintedAmount - transferredAmount).toString());
  assert.equal(userB.amount.toString(), transferredAmount.toString());
  assert.equal(userB.isFrozen, true);
}

export async function assertSss2FlowResult(
  ctx: PresetContext,
  seizedAmount: bigint,
): Promise<void> {
  const [blacklistEntry, treasury, target] = await Promise.all([
    ctx.programs.stablecoinProgram.account.blacklistEntry.fetch(ctx.blacklistPda),
    getAccount(
      ctx.programs.connection,
      ctx.treasuryAta,
      "confirmed",
      TOKEN_2022_PROGRAM_ID,
    ),
    getAccount(
      ctx.programs.connection,
      ctx.userBAta,
      "confirmed",
      TOKEN_2022_PROGRAM_ID,
    ),
  ]);

  assert.equal(blacklistEntry.wallet.toBase58(), ctx.userB.publicKey.toBase58());
  assert.equal(blacklistEntry.mint.toBase58(), ctx.mint.publicKey.toBase58());
  assert.equal(treasury.owner.toBase58(), ctx.authority.publicKey.toBase58());
  assert.equal(treasury.amount.toString(), seizedAmount.toString());
  assert.equal(target.isFrozen, true);
}
