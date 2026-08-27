import { parseAmountMinor } from "./amount.js";
import {
  validateBalancedTransaction,
  type CanonicalPostingInput,
} from "./transaction.js";

export type HoldState = "open" | "released" | "settled";

export interface ValueHold {
  readonly holdId: string;
  readonly accountId: string;
  readonly asset: string;
  readonly amountMinor: string;
  readonly state: HoldState;
}

export interface HoldSettlement {
  readonly hold: ValueHold;
  readonly settledAmountMinor: string;
  readonly releasedAmountMinor: string;
  readonly postings: readonly CanonicalPostingInput[];
}

function requireIdentity(value: string, field: string): void {
  if (value.trim().length === 0) throw new Error(`${field} is required`);
}

function validateHold(hold: ValueHold): bigint {
  requireIdentity(hold.holdId, "Hold identity");
  requireIdentity(hold.accountId, "Hold account");
  requireIdentity(hold.asset, "Hold asset");
  const amount = parseAmountMinor(hold.amountMinor);
  if (amount <= 0n) throw new Error("Hold amount must be positive");
  return amount;
}

export function createValueHold(input: {
  readonly holdId: string;
  readonly accountId: string;
  readonly asset: string;
  readonly amountMinor: string;
  readonly availableBalanceMinor: string;
}): ValueHold {
  requireIdentity(input.holdId, "Hold identity");
  requireIdentity(input.accountId, "Hold account");
  requireIdentity(input.asset, "Hold asset");
  const amount = parseAmountMinor(input.amountMinor);
  const available = parseAmountMinor(input.availableBalanceMinor);
  if (amount <= 0n) throw new Error("Hold amount must be positive");
  if (available < amount)
    throw new Error("Insufficient available value for hold");
  return Object.freeze({
    holdId: input.holdId,
    accountId: input.accountId,
    asset: input.asset,
    amountMinor: amount.toString(),
    state: "open",
  });
}

export function releaseValueHold(hold: ValueHold): ValueHold {
  if (hold.state !== "open")
    throw new Error("Only an open hold can be released");
  validateHold(hold);
  return Object.freeze({ ...hold, state: "released" });
}

export function settleValueHold(input: {
  readonly hold: ValueHold;
  readonly destinationAccountId: string;
  readonly amountMinor?: string;
}): HoldSettlement {
  if (input.hold.state !== "open")
    throw new Error("Only an open hold can be settled");
  const held = validateHold(input.hold);
  requireIdentity(
    input.destinationAccountId,
    "Hold settlement destination account",
  );
  if (input.destinationAccountId === input.hold.accountId) {
    throw new Error("Hold settlement requires a distinct destination account");
  }
  const settled = parseAmountMinor(input.amountMinor ?? input.hold.amountMinor);
  if (settled <= 0n || settled > held) {
    throw new Error(
      "Hold settlement amount must be positive and cannot exceed the hold",
    );
  }
  const postings = Object.freeze([
    Object.freeze({
      accountId: input.hold.accountId,
      asset: input.hold.asset,
      amountMinor: (-settled).toString(),
    }),
    Object.freeze({
      accountId: input.destinationAccountId,
      asset: input.hold.asset,
      amountMinor: settled.toString(),
    }),
  ]);
  validateBalancedTransaction(postings);
  return Object.freeze({
    hold: Object.freeze({ ...input.hold, state: "settled" }),
    settledAmountMinor: settled.toString(),
    releasedAmountMinor: (held - settled).toString(),
    postings,
  });
}
