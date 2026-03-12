import {
  createSss2Preset,
  mintToUser,
  transferBetweenUsers,
} from "./helpers/presets";

describe("Devnet stress", function () {
  this.timeout(10 * 60 * 1000);

  it("runs repeated example operations with explicit bounds", async () => {
    const iterations = Number(process.env.DEVNET_STRESS_ITERATIONS ?? "2");

    for (let i = 0; i < iterations; i += 1) {
      const ctx = await createSss2Preset();
      await mintToUser(ctx, 100_000n);
      await transferBetweenUsers(ctx, 10_000n);
    }
  });
});
