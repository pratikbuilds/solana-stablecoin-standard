import * as anchor from "@coral-xyz/anchor";
import {
  TOKEN_2022_PROGRAM_ID,
  createAssociatedTokenAccountIdempotent,
  getAccount,
  transferChecked,
  transferCheckedWithTransferHook,
} from "@solana/spl-token";
import { Keypair, PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";

import { loadPrograms } from "./cluster";
import {
  blacklistPda,
  configPda,
  eventAuthorityPda,
  extraAccountMetaListPda,
  minterQuotaPda,
  rolesPda,
} from "./pdas";
import { sendInstructions } from "./transactions";
import { fundAuthority, loadPayer } from "./wallet";

export interface PresetContext {
  preset: "SSS-1" | "SSS-2";
  programs: ReturnType<typeof loadPrograms>;
  payer: Keypair;
  authority: Keypair;
  mint: Keypair;
  userA: Keypair;
  userB: Keypair;
  configPda: PublicKey;
  roleConfigPda: PublicKey;
  minterQuotaPda: PublicKey;
  blacklistPda: PublicKey;
  extraAccountMetaListPda: PublicKey;
  treasuryAta: PublicKey;
  userAAta: PublicKey;
  userBAta: PublicKey;
  decimals: number;
}

const DEFAULT_DECIMALS = 6;
const DEFAULT_MINTER_QUOTA = new anchor.BN("1000000000000");

function stablecoinEventAccounts(programId: PublicKey) {
  return {
    eventAuthority: eventAuthorityPda(programId),
    program: programId,
  };
}

async function initializePreset(
  preset: "SSS-1" | "SSS-2",
): Promise<PresetContext> {
  const payer = loadPayer();
  const programs = loadPrograms(payer);
  const authority = Keypair.generate();
  const mint = Keypair.generate();
  const userA = Keypair.generate();
  const userB = Keypair.generate();

  await fundAuthority(programs.connection, payer, authority);

  const config = configPda(programs.stablecoinProgramId, mint.publicKey);
  const roleConfig = rolesPda(programs.stablecoinProgramId, mint.publicKey);
  const quotaPda = minterQuotaPda(
    programs.stablecoinProgramId,
    mint.publicKey,
    authority.publicKey,
  );
  const userBBlacklist = blacklistPda(
    programs.stablecoinProgramId,
    mint.publicKey,
    userB.publicKey,
  );
  const extraMeta = extraAccountMetaListPda(
    programs.transferHookProgramId,
    mint.publicKey,
  );

  const params = {
    name: preset === "SSS-1" ? "Simple USD" : "Regulated USD",
    symbol: preset === "SSS-1" ? "SUSD" : "RUSD",
    uri:
      preset === "SSS-1"
        ? "https://example.com/sss1.json"
        : "https://example.com/sss2.json",
    decimals: DEFAULT_DECIMALS,
    enablePermanentDelegate: preset === "SSS-2",
    enableTransferHook: preset === "SSS-2",
    defaultAccountFrozen: preset === "SSS-2",
  };

  const initializeBuilder = programs.stablecoinProgram.methods
    .initialize(params)
    .accountsPartial({
      authority: authority.publicKey,
      mint: mint.publicKey,
      config,
      roleConfig,
      ...stablecoinEventAccounts(programs.stablecoinProgramId),
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
      rent: SYSVAR_RENT_PUBKEY,
      ...(preset === "SSS-2"
        ? {
            extraAccountMetaList: extraMeta,
            transferHookProgram: programs.transferHookProgramId,
          }
        : {
            extraAccountMetaList: null,
            transferHookProgram: null,
          }),
    })
    .signers([authority, mint]);

  await initializeBuilder.rpc();

  const treasuryAta = await createAssociatedTokenAccountIdempotent(
    programs.connection,
    payer,
    mint.publicKey,
    authority.publicKey,
    undefined,
    TOKEN_2022_PROGRAM_ID,
  );
  const userAAta = await createAssociatedTokenAccountIdempotent(
    programs.connection,
    payer,
    mint.publicKey,
    userA.publicKey,
    undefined,
    TOKEN_2022_PROGRAM_ID,
  );
  const userBAta = await createAssociatedTokenAccountIdempotent(
    programs.connection,
    payer,
    mint.publicKey,
    userB.publicKey,
    undefined,
    TOKEN_2022_PROGRAM_ID,
  );

  await programs.stablecoinProgram.methods
    .updateMinter({
      minter: authority.publicKey,
      quota: DEFAULT_MINTER_QUOTA,
      active: true,
    })
    .accountsPartial({
      authority: authority.publicKey,
      config,
      roleConfig,
      mint: mint.publicKey,
      minter: authority.publicKey,
      minterQuota: quotaPda,
      ...stablecoinEventAccounts(programs.stablecoinProgramId),
      systemProgram: SystemProgram.programId,
    })
    .signers([authority])
    .rpc();

  const context: PresetContext = {
    preset,
    programs,
    payer,
    authority,
    mint,
    userA,
    userB,
    configPda: config,
    roleConfigPda: roleConfig,
    minterQuotaPda: quotaPda,
    blacklistPda: userBBlacklist,
    extraAccountMetaListPda: extraMeta,
    treasuryAta,
    userAAta,
    userBAta,
    decimals: DEFAULT_DECIMALS,
  };

  if (preset === "SSS-2") {
    await thawAccount(context, treasuryAta);
    await thawAccount(context, userAAta);
    await thawAccount(context, userBAta);
  }

  return context;
}

export async function createSss1Preset(): Promise<PresetContext> {
  return initializePreset("SSS-1");
}

export async function createSss2Preset(): Promise<PresetContext> {
  return initializePreset("SSS-2");
}

export async function mintToUser(
  ctx: PresetContext,
  amount: bigint,
  destination: "userA" | "userB" = "userA",
): Promise<string> {
  return ctx.programs.stablecoinProgram.methods
    .mint(new anchor.BN(amount.toString()))
    .accountsPartial({
      authority: ctx.authority.publicKey,
      config: ctx.configPda,
      minterQuota: ctx.minterQuotaPda,
      mint: ctx.mint.publicKey,
      to: destination === "userA" ? ctx.userAAta : ctx.userBAta,
      ...stablecoinEventAccounts(ctx.programs.stablecoinProgramId),
      tokenProgram: TOKEN_2022_PROGRAM_ID,
    })
    .signers([ctx.authority])
    .rpc();
}

export async function transferBetweenUsers(
  ctx: PresetContext,
  amount: bigint,
): Promise<string> {
  if (ctx.preset === "SSS-2") {
    return transferCheckedWithTransferHook(
      ctx.programs.connection,
      ctx.payer,
      ctx.userAAta,
      ctx.mint.publicKey,
      ctx.userBAta,
      ctx.userA,
      amount,
      ctx.decimals,
      [],
      {
        commitment: "confirmed",
        preflightCommitment: "confirmed",
      },
      TOKEN_2022_PROGRAM_ID,
    );
  }

  return transferChecked(
    ctx.programs.connection,
    ctx.payer,
    ctx.userAAta,
    ctx.mint.publicKey,
    ctx.userBAta,
    ctx.userA,
    amount,
    ctx.decimals,
    [],
    {
      commitment: "confirmed",
      preflightCommitment: "confirmed",
    },
    TOKEN_2022_PROGRAM_ID,
  );
}

export async function freezeAccount(
  ctx: PresetContext,
  account: PublicKey,
): Promise<string> {
  return ctx.programs.stablecoinProgram.methods
    .freezeAccount()
    .accountsPartial({
      authority: ctx.authority.publicKey,
      config: ctx.configPda,
      roleConfig: ctx.roleConfigPda,
      mint: ctx.mint.publicKey,
      account,
      ...stablecoinEventAccounts(ctx.programs.stablecoinProgramId),
      tokenProgram: TOKEN_2022_PROGRAM_ID,
    })
    .signers([ctx.authority])
    .rpc();
}

export async function thawAccount(
  ctx: PresetContext,
  account: PublicKey,
): Promise<string> {
  return ctx.programs.stablecoinProgram.methods
    .thawAccount()
    .accountsPartial({
      authority: ctx.authority.publicKey,
      config: ctx.configPda,
      roleConfig: ctx.roleConfigPda,
      mint: ctx.mint.publicKey,
      account,
      ...stablecoinEventAccounts(ctx.programs.stablecoinProgramId),
      tokenProgram: TOKEN_2022_PROGRAM_ID,
    })
    .signers([ctx.authority])
    .rpc();
}

export async function blacklistUser(
  ctx: PresetContext,
  reason = "compliance review",
): Promise<string> {
  return ctx.programs.stablecoinProgram.methods
    .addToBlacklist(reason)
    .accountsPartial({
      authority: ctx.authority.publicKey,
      config: ctx.configPda,
      roleConfig: ctx.roleConfigPda,
      wallet: ctx.userB.publicKey,
      blacklistEntry: ctx.blacklistPda,
      ...stablecoinEventAccounts(ctx.programs.stablecoinProgramId),
      systemProgram: SystemProgram.programId,
    })
    .signers([ctx.authority])
    .rpc();
}

export async function seizeFromBlacklistedAccount(
  ctx: PresetContext,
  amount: bigint,
): Promise<string> {
  return ctx.programs.stablecoinProgram.methods
    .seize(new anchor.BN(amount.toString()))
    .accountsPartial({
      authority: ctx.authority.publicKey,
      config: ctx.configPda,
      roleConfig: ctx.roleConfigPda,
      mint: ctx.mint.publicKey,
      from: ctx.userBAta,
      to: ctx.treasuryAta,
      blacklistEntry: ctx.blacklistPda,
      stablecoinProgram: ctx.programs.stablecoinProgramId,
      transferHookProgram: ctx.programs.transferHookProgramId,
      extraAccountMetaList: ctx.extraAccountMetaListPda,
      destinationBlacklist: blacklistPda(
        ctx.programs.stablecoinProgramId,
        ctx.mint.publicKey,
        ctx.authority.publicKey,
      ),
      ...stablecoinEventAccounts(ctx.programs.stablecoinProgramId),
      tokenProgram: TOKEN_2022_PROGRAM_ID,
    })
    .signers([ctx.authority])
    .rpc();
}

export async function pauseMint(ctx: PresetContext): Promise<string> {
  return ctx.programs.stablecoinProgram.methods
    .pause()
    .accountsPartial({
      authority: ctx.authority.publicKey,
      config: ctx.configPda,
      roleConfig: ctx.roleConfigPda,
      ...stablecoinEventAccounts(ctx.programs.stablecoinProgramId),
    })
    .signers([ctx.authority])
    .rpc();
}

export async function readTokenAmount(
  ctx: PresetContext,
  account: PublicKey,
): Promise<bigint> {
  const tokenAccount = await getAccount(
    ctx.programs.connection,
    account,
    "confirmed",
    TOKEN_2022_PROGRAM_ID,
  );
  return tokenAccount.amount;
}

export async function topUpUserForDirectTransfers(
  ctx: PresetContext,
  user: Keypair,
): Promise<void> {
  await sendInstructions(ctx.programs.connection, ctx.payer, [
    SystemProgram.transfer({
      fromPubkey: ctx.payer.publicKey,
      toPubkey: user.publicKey,
      lamports: 20_000_000,
    }),
  ]);
}
