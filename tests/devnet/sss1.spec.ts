import { assertSss1FlowResult } from "./helpers/assertions";
import {
  createSss1Preset,
  freezeAccount,
  mintToUser,
  transferBetweenUsers,
} from "./helpers/presets";

describe("SSS-1 devnet flow", function () {
  this.timeout(180_000);

  it("runs mint -> transfer -> freeze", async () => {
    const ctx = await createSss1Preset();
    const mintedAmount = 1_000_000n;
    const transferredAmount = 250_000n;

    await mintToUser(ctx, mintedAmount);
    await transferBetweenUsers(ctx, transferredAmount);
    await freezeAccount(ctx, ctx.userBAta);
    await assertSss1FlowResult(ctx, mintedAmount, transferredAmount);
  });
});
