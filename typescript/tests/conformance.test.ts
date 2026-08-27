import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import { multiplyRationalFloor } from '../src/amount.js';
import { multiplyRationalHalfEven } from '../src/rate.js';
import {
  applyBalancedTransaction,
  createPostingManifestDigest,
  createTransactionReversal,
  type AccountBalance,
  type CanonicalPostingInput,
} from '../src/transaction.js';

interface RationalFloorVector {
  readonly amountMinor: string;
  readonly numerator: string;
  readonly denominator: string;
  readonly expectedAmountMinor: string;
  readonly remainderNumerator: string;
}

interface HalfEvenVector {
  readonly amountMinor: string;
  readonly numerator: string;
  readonly denominator: string;
  readonly expected: string;
}

interface ValueRulesFixture {
  readonly profile: string;
  readonly rationalFloor: readonly RationalFloorVector[];
  readonly halfEven: readonly HalfEvenVector[];
  readonly transaction: {
    readonly balances: readonly AccountBalance[];
    readonly postings: readonly CanonicalPostingInput[];
    readonly expectedBalances: readonly AccountBalance[];
    readonly reversal: readonly CanonicalPostingInput[];
    readonly digest: string;
  };
}

const fixture = JSON.parse(
  readFileSync(new URL('../../rust/fixtures/value-rules-v1.json', import.meta.url), 'utf8'),
) as ValueRulesFixture;

describe('cross-language value rule conformance', () => {
  it('matches exact floor and half-even arithmetic vectors', () => {
    expect(fixture.profile).toBe('value-core-rules-v1');
    for (const vector of fixture.rationalFloor) {
      expect(
        multiplyRationalFloor(vector.amountMinor, vector.numerator, vector.denominator),
      ).toEqual({
        amountMinor: vector.expectedAmountMinor,
        remainderNumerator: vector.remainderNumerator,
      });
    }
    for (const vector of fixture.halfEven) {
      expect(
        multiplyRationalHalfEven(vector.amountMinor, vector.numerator, vector.denominator),
      ).toEqual({ amountMinor: vector.expected });
    }
  });

  it('matches atomic application, reversal, and manifest evidence', async () => {
    const vector = fixture.transaction;
    expect(
      applyBalancedTransaction({ balances: vector.balances, postings: vector.postings }),
    ).toEqual(vector.expectedBalances);
    expect(createTransactionReversal(vector.postings)).toEqual(vector.reversal);
    await expect(createPostingManifestDigest(vector.postings)).resolves.toBe(vector.digest);
  });
});
