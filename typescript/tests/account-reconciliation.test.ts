import { describe, expect, it } from 'vitest';

import { foldAccountHistory } from '../src/account.js';
import { reconcileBalances } from '../src/reconciliation.js';

describe('account history and reconciliation', () => {
  it('folds a gap-free account history', () => {
    expect(
      foldAccountHistory({
        openingBalanceMinor: '10',
        postings: [
          {
            sequence: 1,
            balanceBeforeMinor: '10',
            amountMinor: '5',
            balanceAfterMinor: '15',
          },
        ],
      }),
    ).toEqual({ finalBalanceMinor: '15', lastSequence: 1 });
    expect(() =>
      foldAccountHistory({
        openingBalanceMinor: '10',
        postings: [
          {
            sequence: 2,
            balanceBeforeMinor: '10',
            amountMinor: '5',
            balanceAfterMinor: '15',
          },
        ],
      }),
    ).toThrow('gap or duplicate');
  });

  it('reports exact deterministic reconciliation differences', () => {
    expect(
      reconcileBalances({
        expected: [{ accountId: 'a', asset: 'credits', amountMinor: '10' }],
        actual: [{ accountId: 'a', asset: 'credits', amountMinor: '8' }],
      }),
    ).toEqual({
      closed: false,
      differences: [
        {
          accountId: 'a',
          asset: 'credits',
          expectedMinor: '10',
          actualMinor: '8',
          differenceMinor: '-2',
        },
      ],
    });
  });
});
