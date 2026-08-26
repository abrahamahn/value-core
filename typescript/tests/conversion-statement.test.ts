import { describe, expect, it } from 'vitest';

import {
  buildValueConversionPlan,
  planValueConversionCorrection,
  validateValueConversionQuote,
} from '../src/conversion.js';
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
    ).toEqual([
      {
        asset: 'credits',
        postings: [
          { asset: 'credits', amountMinor: '-10' },
          { asset: 'credits', amountMinor: '10' },
        ],
      },
      {
        asset: 'points',
        postings: [
          { asset: 'points', amountMinor: '-25' },
          { asset: 'points', amountMinor: '25' },
        ],
      },
    ]);
  });

  it('rejects a conversion whose pinned rate rounds to a zero destination posting', () => {
    expect(() =>
      buildValueConversionPlan({
        quoteId: 'quote-small',
        sourceAsset: 'credits',
        destinationAsset: 'points',
        sourceAmountMinor: '1',
        destinationAmountMinor: '0',
        rateNumerator: '1',
        rateDenominator: '2',
        rounding: 'floor',
      }),
    ).toThrow('destinationAmountMinor must be positive');
  });

  it('rejects impossible calendar dates when validating quote expiry', () => {
    expect(() =>
      validateValueConversionQuote({
        quote: {
          quoteId: 'quote-date',
          actorId: 'actor-1',
          rateSnapshotId: 'rate-1',
          expiresAt: '2026-02-31T00:00:00.000Z',
        },
        actorId: 'actor-1',
        rateSnapshotId: 'rate-1',
        evaluatedAt: '2026-02-01T00:00:00.000Z',
      }),
    ).toThrow('RFC 3339 instant');
  });

  it('rejects a correction for an impossible zero-value original conversion', () => {
    expect(() =>
      planValueConversionCorrection({
        original: {
          sourceAsset: 'credits',
          sourceAmountMinor: '1',
          destinationAsset: 'points',
          destinationAmountMinor: '0',
          rateSnapshotId: 'rate-1',
        },
        correctionKind: 'literal_reversal',
      }),
    ).toThrow('destinationAmountMinor must be positive');
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

  it('rejects statement facts for a foreign account or asset', () => {
    const posting = {
      postingId: 'posting-1',
      transactionId: 'transaction-1',
      accountId: 'other-account',
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
    };
    expect(() =>
      buildValueStatementPage({
        accountId: 'account-1',
        asset: 'credits',
        previousAccountSequence: '0',
        openingBalanceMinor: '10',
        limit: 10,
        postings: [posting],
      }),
    ).toThrow('foreign account');
    expect(() =>
      buildValueStatementPage({
        accountId: 'other-account',
        asset: 'points',
        previousAccountSequence: '0',
        openingBalanceMinor: '10',
        limit: 10,
        postings: [posting],
      }),
    ).toThrow('cross-asset');
  });
});
