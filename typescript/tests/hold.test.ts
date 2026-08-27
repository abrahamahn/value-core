import { describe, expect, it } from "vitest";

import {
  createValueHold,
  releaseValueHold,
  settleValueHold,
} from "../src/hold.js";

describe("value holds", () => {
  it("places, partially settles, and releases the remainder of an available hold", () => {
    const hold = createValueHold({
      holdId: "hold-1",
      accountId: "buyer",
      asset: "credits",
      amountMinor: "40",
      availableBalanceMinor: "100",
    });
    expect(
      settleValueHold({
        hold,
        destinationAccountId: "seller",
        amountMinor: "30",
      }),
    ).toEqual({
      hold: { ...hold, state: "settled" },
      settledAmountMinor: "30",
      releasedAmountMinor: "10",
      postings: [
        { accountId: "buyer", asset: "credits", amountMinor: "-30" },
        { accountId: "seller", asset: "credits", amountMinor: "30" },
      ],
    });
  });

  it("rejects over-reservation and invalid lifecycle transitions", () => {
    expect(() =>
      createValueHold({
        holdId: "hold-1",
        accountId: "buyer",
        asset: "credits",
        amountMinor: "101",
        availableBalanceMinor: "100",
      }),
    ).toThrow("Insufficient");
    const released = releaseValueHold(
      createValueHold({
        holdId: "hold-2",
        accountId: "buyer",
        asset: "credits",
        amountMinor: "10",
        availableBalanceMinor: "100",
      }),
    );
    expect(() => releaseValueHold(released)).toThrow("Only an open hold");
    expect(() =>
      releaseValueHold({
        holdId: "forged",
        accountId: "buyer",
        asset: "credits",
        amountMinor: "0",
        state: "open",
      }),
    ).toThrow("must be positive");
  });
});
