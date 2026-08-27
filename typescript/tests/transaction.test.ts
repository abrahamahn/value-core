import { describe, expect, it } from "vitest";

import {
  applyBalancedTransaction,
  createPostingManifestDigest,
  createTransactionReversal,
  validateBalancedTransaction,
  validatePostingManifest,
} from "../src/transaction.js";

const transfer = Object.freeze([
  Object.freeze({ accountId: "source", asset: "credits", amountMinor: "-25" }),
  Object.freeze({
    accountId: "destination",
    asset: "credits",
    amountMinor: "25",
  }),
]);

describe("balanced value transactions", () => {
  it("applies a transfer atomically without mutating the input balances", () => {
    const balances = Object.freeze([
      Object.freeze({
        accountId: "source",
        asset: "credits",
        balanceMinor: "100",
      }),
      Object.freeze({
        accountId: "destination",
        asset: "credits",
        balanceMinor: "5",
      }),
    ]);
    expect(applyBalancedTransaction({ balances, postings: transfer })).toEqual([
      { accountId: "source", asset: "credits", balanceMinor: "75" },
      { accountId: "destination", asset: "credits", balanceMinor: "30" },
    ]);
    expect(balances[0]?.balanceMinor).toBe("100");
  });

  it("rejects insufficient value without returning a partial result", () => {
    expect(() =>
      applyBalancedTransaction({
        balances: [
          { accountId: "source", asset: "credits", balanceMinor: "20" },
          { accountId: "destination", asset: "credits", balanceMinor: "0" },
        ],
        postings: transfer,
      }),
    ).toThrow("insufficient value");
  });

  it("rejects an invalid negative opening balance even when postings would heal it", () => {
    expect(() =>
      applyBalancedTransaction({
        balances: [
          { accountId: "source", asset: "credits", balanceMinor: "-5" },
          { accountId: "destination", asset: "credits", balanceMinor: "10" },
        ],
        postings: [
          { accountId: "source", asset: "credits", amountMinor: "5" },
          { accountId: "destination", asset: "credits", amountMinor: "-5" },
        ],
      }),
    ).toThrow("cannot start with negative value");
  });

  it("conserves each asset independently in a multi-posting transaction", () => {
    expect(
      validateBalancedTransaction([
        { accountId: "a", asset: "credits", amountMinor: "-7" },
        { accountId: "b", asset: "credits", amountMinor: "5" },
        { accountId: "fee", asset: "credits", amountMinor: "2" },
        { accountId: "x", asset: "points", amountMinor: "-3" },
        { accountId: "y", asset: "points", amountMinor: "3" },
      ]),
    ).toEqual({ totalMinor: "0" });
    expect(() =>
      validateBalancedTransaction([
        { accountId: "a", asset: "credits", amountMinor: "-7" },
        { accountId: "b", asset: "credits", amountMinor: "6" },
      ]),
    ).toThrow("non-zero");
  });

  it("creates an exact reversal and a closed, verifiable manifest", async () => {
    expect(createTransactionReversal(transfer)).toEqual([
      { accountId: "source", asset: "credits", amountMinor: "25" },
      { accountId: "destination", asset: "credits", amountMinor: "-25" },
    ]);
    const digest = await createPostingManifestDigest(transfer);
    await expect(
      validatePostingManifest({
        declaredCount: 2,
        postings: transfer,
        closed: true,
        digest,
      }),
    ).resolves.toMatchObject({ digest });
  });

  it("rejects incomplete and tampered posting manifests", async () => {
    const digest = await createPostingManifestDigest(transfer);
    await expect(
      validatePostingManifest({
        declaredCount: 2,
        postings: transfer,
        closed: false,
        digest,
      }),
    ).rejects.toThrow("closed before commit");
    await expect(
      validatePostingManifest({
        declaredCount: 3,
        postings: transfer,
        closed: true,
        digest,
      }),
    ).rejects.toThrow("count does not match");
    await expect(
      validatePostingManifest({
        declaredCount: 2,
        postings: transfer,
        closed: true,
        digest: "0".repeat(64),
      }),
    ).rejects.toThrow("digest mismatch");
  });
});
