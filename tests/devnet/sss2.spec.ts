import { assertSss2FlowResult } from "./helpers/assertions";
import {
  blacklistUser,
  createSss2Preset,
  freezeAccount,
  mintToUser,
  seizeFromBlacklistedAccount,
  transferBetweenUsers,
} from "./helpers/presets";

describe("SSS-2 devnet flow", function () {
  this.timeout(240_000);

  it("runs mint -> transfer -> blacklist -> seize", async () => {
    const ctx = await createSss2Preset();
    const mintedAmount = 1_000_000n;
    const transferredAmount = 250_000n;

    await mintToUser(ctx, mintedAmount);
    await transferBetweenUsers(ctx, transferredAmount);
    await blacklistUser(ctx);
    await freezeAccount(ctx, ctx.userBAta);
    await seizeFromBlacklistedAccount(ctx, transferredAmount);
    await assertSss2FlowResult(ctx, transferredAmount);
  });
});
