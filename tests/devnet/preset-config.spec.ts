import assert from "node:assert/strict";

import { loadProgramIds } from "./config";
import { assertSss1Flags, assertSss2Flags } from "./helpers/assertions";
import { createSss1Preset, createSss2Preset } from "./helpers/presets";

describe("devnet preset config", function () {
  this.timeout(180_000);

  it("loads deployed program ids", () => {
    const ids = loadProgramIds();
    assert.ok(ids.stablecoinProgramId);
    assert.ok(ids.transferHookProgramId);
  });

  it("initializes an SSS-1 preset with expected flags", async () => {
    const ctx = await createSss1Preset();
    await assertSss1Flags(ctx);
  });

  it("initializes an SSS-2 preset with expected flags", async () => {
    const ctx = await createSss2Preset();
    await assertSss2Flags(ctx);
  });
});
