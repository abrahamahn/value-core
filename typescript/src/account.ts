import { parseAmountMinor } from './amount.js';

export interface AccountHistoryPosting {
  readonly sequence: number;
  readonly balanceBeforeMinor: string;
  readonly amountMinor: string;
  readonly balanceAfterMinor: string;
}

export interface FoldedAccountHistory {
  readonly finalBalanceMinor: string;
  readonly lastSequence: number;
}

export function foldAccountHistory(input: {
  readonly openingBalanceMinor: string;
  readonly postings: readonly AccountHistoryPosting[];
  readonly allowNegative?: boolean;
}): FoldedAccountHistory {
  let balance = parseAmountMinor(input.openingBalanceMinor);
  if (balance < 0n && input.allowNegative !== true) {
    throw new Error('Account opening balance cannot be negative');
  }
  let sequence = 0;
  for (const posting of input.postings) {
    if (!Number.isSafeInteger(posting.sequence) || posting.sequence !== sequence + 1) {
      throw new Error('Account sequence contains a gap or duplicate');
    }
    if (parseAmountMinor(posting.balanceBeforeMinor) !== balance) {
      throw new Error('Account balance-before continuity mismatch');
    }
    const next = balance + parseAmountMinor(posting.amountMinor);
    const statedNext = parseAmountMinor(posting.balanceAfterMinor);
    if (statedNext !== next) throw new Error('Account balance-after continuity mismatch');
    if (next < 0n && input.allowNegative !== true) {
      throw new Error('Account cannot become negative');
    }
    balance = next;
    sequence = posting.sequence;
  }
  return Object.freeze({
    finalBalanceMinor: balance.toString(),
    lastSequence: sequence,
  });
}
