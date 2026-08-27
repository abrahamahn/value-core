import { parseAmountMinor } from "./amount.js";

export const MAX_STATEMENT_PAGE_SIZE = 500;

export interface ValueStatementPostingFact {
  readonly postingId: string;
  readonly transactionId: string;
  readonly accountId: string;
  readonly accountSequence: string;
  readonly postingSequence: number;
  readonly asset: string;
  readonly amountMinor: string;
  readonly balanceBeforeMinor: string;
  readonly balanceAfterMinor: string;
  readonly occurredAt: string;
  readonly recordedAt: string;
  readonly sourceNamespace: string;
  readonly sourceType: string;
  readonly sourceId: string;
}

export interface ValueStatementCursor {
  readonly accountSequence: string;
  readonly postingId: string;
}

export interface ValueStatementPage {
  readonly accountId: string;
  readonly asset: string;
  readonly previousAccountSequence: string;
  readonly openingBalanceMinor: string;
  readonly closingAccountSequence: string;
  readonly closingBalanceMinor: string;
  readonly entries: readonly ValueStatementPostingFact[];
  readonly hasMore: boolean;
  readonly nextCursor?: ValueStatementCursor;
}

function requireIdentity(value: string, field: string): void {
  if (value.trim().length === 0) throw new Error(`${field} is required`);
}

function parseSequence(value: string, allowZero: boolean): bigint {
  const sequence = parseAmountMinor(value);
  if (allowZero ? sequence < 0n : sequence <= 0n) {
    throw new Error("Value statement account sequence is invalid");
  }
  return sequence;
}

function requireTimestamp(value: string, field: string): void {
  const parsed = new Date(value);
  if (!Number.isFinite(parsed.getTime()) || parsed.toISOString() !== value) {
    throw new Error(`${field} must be a canonical timestamp`);
  }
}

export function buildValueStatementPage(input: {
  readonly accountId: string;
  readonly asset: string;
  readonly previousAccountSequence: string;
  readonly openingBalanceMinor: string;
  readonly limit: number;
  readonly postings: readonly ValueStatementPostingFact[];
}): ValueStatementPage {
  requireIdentity(input.accountId, "Value statement account");
  requireIdentity(input.asset, "Value statement asset");
  if (
    !Number.isSafeInteger(input.limit) ||
    input.limit < 1 ||
    input.limit > MAX_STATEMENT_PAGE_SIZE
  ) {
    throw new Error("Value statement page size is invalid");
  }

  const previousSequence = parseSequence(input.previousAccountSequence, true);
  let expectedSequence = previousSequence + 1n;
  let runningBalance = parseAmountMinor(input.openingBalanceMinor);
  let pageClosingSequence = previousSequence;
  let pageClosingBalance = runningBalance;
  const postingIds = new Set<string>();
  const accountSequences = new Set<string>();
  const ordered = [...input.postings].sort((left, right) => {
    const leftSequence = parseSequence(left.accountSequence, false);
    const rightSequence = parseSequence(right.accountSequence, false);
    return leftSequence < rightSequence
      ? -1
      : leftSequence > rightSequence
        ? 1
        : 0;
  });

  for (const [index, posting] of ordered.entries()) {
    requireIdentity(posting.postingId, "Value statement posting");
    requireIdentity(posting.transactionId, "Value statement transaction");
    requireIdentity(
      posting.sourceNamespace,
      "Value statement source namespace",
    );
    requireIdentity(posting.sourceType, "Value statement source type");
    requireIdentity(posting.sourceId, "Value statement source");
    requireTimestamp(posting.occurredAt, "Value statement occurrence time");
    requireTimestamp(posting.recordedAt, "Value statement record time");
    if (posting.accountId !== input.accountId) {
      throw new Error("Value statement contains a foreign account");
    }
    if (posting.asset !== input.asset) {
      throw new Error("Value statement contains a cross-asset posting");
    }
    if (
      !Number.isSafeInteger(posting.postingSequence) ||
      posting.postingSequence < 1
    ) {
      throw new Error("Value statement posting sequence is invalid");
    }
    const accountSequence = parseSequence(posting.accountSequence, false);
    if (
      postingIds.has(posting.postingId) ||
      accountSequences.has(posting.accountSequence)
    ) {
      throw new Error(
        "Value statement contains a duplicate posting identity or sequence",
      );
    }
    postingIds.add(posting.postingId);
    accountSequences.add(posting.accountSequence);
    if (accountSequence !== expectedSequence) {
      throw new Error("Value statement account sequence is not gap-free");
    }

    const amount = parseAmountMinor(posting.amountMinor);
    if (amount === 0n)
      throw new Error("Value statement posting must be non-zero");
    const balanceBefore = parseAmountMinor(posting.balanceBeforeMinor);
    const balanceAfter = parseAmountMinor(posting.balanceAfterMinor);
    if (balanceBefore !== runningBalance) {
      throw new Error("Value statement balance-before continuity mismatch");
    }
    if (balanceAfter !== balanceBefore + amount) {
      throw new Error("Value statement balance-after continuity mismatch");
    }

    runningBalance = balanceAfter;
    expectedSequence += 1n;
    if (index < input.limit) {
      pageClosingSequence = accountSequence;
      pageClosingBalance = balanceAfter;
    }
  }

  const entries = Object.freeze(ordered.slice(0, input.limit));
  const hasMore = ordered.length > entries.length;
  const lastEntry = entries.at(-1);
  return Object.freeze({
    accountId: input.accountId,
    asset: input.asset,
    previousAccountSequence: previousSequence.toString(),
    openingBalanceMinor: parseAmountMinor(input.openingBalanceMinor).toString(),
    closingAccountSequence: pageClosingSequence.toString(),
    closingBalanceMinor: pageClosingBalance.toString(),
    entries,
    hasMore,
    ...(hasMore && lastEntry !== undefined
      ? {
          nextCursor: Object.freeze({
            accountSequence: lastEntry.accountSequence,
            postingId: lastEntry.postingId,
          }),
        }
      : {}),
  });
}
