import { describe, expect, it } from "vitest";

import {
  evaluateValueArithmetic,
  multiplyRationalFloor,
  parseAmountMinor,
} from "../src/amount.js";

describe("exact value arithmetic", () => {
  it("accepts only canonical signed 64-bit minor units", () => {
    expect(parseAmountMinor("9223372036854775807")).toBe(
      9_223_372_036_854_775_807n,
    );
    expect(parseAmountMinor("-9223372036854775808")).toBe(
      -9_223_372_036_854_775_808n,
    );
    for (const invalid of ["-0", "+1", "01", "9223372036854775808"]) {
      expect(() => parseAmountMinor(invalid)).toThrow();
    }
  });

  it("detects overflow and floors negative rational results exactly", () => {
    expect(() =>
      evaluateValueArithmetic({
        operation: "add",
        left: "9223372036854775807",
        right: "1",
      }),
    ).toThrow("overflow");
    expect(multiplyRationalFloor("-3", "1", "2")).toEqual({
      amountMinor: "-2",
      remainderNumerator: "1",
    });
  });
});
