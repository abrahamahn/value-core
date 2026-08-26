import { describe, expect, it } from 'vitest';

import { buildValueConversionPlan } from '../src/conversion.js';
import { buildValueStatementPage } from '../src/statement.js';

describe('conversion and statements', () => {
  it('builds independently balanced transactions for each converted asset', () => {
    expect(
      buildValueConversionPlan({
        quoteId: 'quote-1',
        sourceAsset: 'credits',
        destinationAsset: 'points',
        sourceAmountMinor: '10',
        destinationAmountMinor: '25',
        rateNumerator: '5',
        rateDenominator: '2',
        rounding: 'floor',
      }).transactions,
    ).toHaveLength(2);
  });

  it('builds a statement only from gap-free, balance-continuous facts', () => {
    expect(
      buildValueStatementPage({
        accountId: 'account-1',
        asset: 'credits',
        previousAccountSequence: '0',
        openingBalanceMinor: '10',
        limit: 10,
        postings: [
          {
            postingId: 'posting-1',
            transactionId: 'transaction-1',
            accountId: 'account-1',
            accountSequence: '1',
            postingSequence: 1,
            asset: 'credits',
            amountMinor: '5',
            balanceBeforeMinor: '10',
            balanceAfterMinor: '15',
            occurredAt: '2026-01-01T00:00:00.000Z',
            recordedAt: '2026-01-01T00:00:00.000Z',
            sourceNamespace: 'marketplace',
            sourceType: 'sale',
            sourceId: 'sale-1',
          },
        ],
      }).closingBalanceMinor,
    ).toBe('15');
  });
});
