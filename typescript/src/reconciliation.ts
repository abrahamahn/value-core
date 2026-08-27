import { parseAmountMinor } from "./amount.js";

export interface ReconciliationBalance {
  readonly accountId: string;
  readonly asset: string;
  readonly amountMinor: string;
}

export interface ReconciliationDifference {
  readonly accountId: string;
  readonly asset: string;
  readonly expectedMinor: string;
  readonly actualMinor: string;
  readonly differenceMinor: string;
}

export interface ReconciliationResult {
  readonly closed: boolean;
  readonly differences: readonly ReconciliationDifference[];
}

function indexBalances(
  balances: readonly ReconciliationBalance[],
): ReadonlyMap<string, ReconciliationBalance> {
  const indexed = new Map<string, ReconciliationBalance>();
  for (const balance of balances) {
    if (
      balance.accountId.trim().length === 0 ||
      balance.asset.trim().length === 0
    ) {
      throw new Error("Reconciliation account identity and asset are required");
    }
    parseAmountMinor(balance.amountMinor);
    const key = `${balance.asset}\u0000${balance.accountId}`;
    if (indexed.has(key))
      throw new Error("Reconciliation balance identity is duplicated");
    indexed.set(key, balance);
  }
  return indexed;
}

export function reconcileBalances(input: {
  readonly expected: readonly ReconciliationBalance[];
  readonly actual: readonly ReconciliationBalance[];
}): ReconciliationResult {
  const expected = indexBalances(input.expected);
  const actual = indexBalances(input.actual);
  const keys = [...new Set([...expected.keys(), ...actual.keys()])].sort();
  const differences: ReconciliationDifference[] = [];
  for (const key of keys) {
    const expectedBalance = expected.get(key);
    const actualBalance = actual.get(key);
    const identity = expectedBalance ?? actualBalance;
    if (identity === undefined) continue;
    const expectedMinor = parseAmountMinor(expectedBalance?.amountMinor ?? "0");
    const actualMinor = parseAmountMinor(actualBalance?.amountMinor ?? "0");
    if (expectedMinor !== actualMinor) {
      differences.push(
        Object.freeze({
          accountId: identity.accountId,
          asset: identity.asset,
          expectedMinor: expectedMinor.toString(),
          actualMinor: actualMinor.toString(),
          differenceMinor: (actualMinor - expectedMinor).toString(),
        }),
      );
    }
  }
  return Object.freeze({
    closed: differences.length === 0,
    differences: Object.freeze(differences),
  });
}
