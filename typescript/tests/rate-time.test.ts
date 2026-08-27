import { describe, expect, it } from 'vitest';

import {
  createValueRateSnapshot,
  evaluateValueRateFreshness,
  multiplyRationalHalfEven,
} from '../src/rate.js';
import { formatRfc3339Millis, parseRfc3339Millis } from '../src/time.js';

describe('value rates and time', () => {
  it('rounds rational ties to the nearest even minor unit', () => {
    expect(multiplyRationalHalfEven('1', '1', '2').amountMinor).toBe('0');
    expect(multiplyRationalHalfEven('3', '1', '2').amountMinor).toBe('2');
    expect(multiplyRationalHalfEven('5', '1', '2').amountMinor).toBe('2');
    expect(multiplyRationalHalfEven('7', '1', '2').amountMinor).toBe('4');
  });

  it('creates a generic snapshot with deterministic expiry', () => {
    expect(
      createValueRateSnapshot({
        snapshotId: 'rate-1',
        baseAsset: 'credits',
        quoteAsset: 'points',
        numerator: '5',
        denominator: '2',
        observedAt: '2026-01-01T00:00:00.000Z',
        recordedAt: '2026-01-01T00:00:01.000Z',
        effectiveAt: '2026-01-01T00:00:02.000Z',
        maxStalenessSeconds: 60,
      }).expiresAt,
    ).toBe('2026-01-01T00:01:02.000Z');
    expect(() =>
      createValueRateSnapshot({
        snapshotId: 'rate-out-of-range',
        baseAsset: 'credits',
        quoteAsset: 'points',
        numerator: '1',
        denominator: '1',
        observedAt: '2026-01-01T00:00:00.000Z',
        recordedAt: '2026-01-01T00:00:00.000Z',
        effectiveAt: '2026-01-01T00:00:00.000Z',
        maxStalenessSeconds: Number.MAX_SAFE_INTEGER,
      }),
    ).toThrow(/range/i);
  });

  it('evaluates refresh boundaries exactly', () => {
    expect(
      evaluateValueRateFreshness({
        snapshotId: 'rate-1',
        capturedAt: '2026-01-01T00:00:00.000Z',
        evaluatedAt: '2026-01-01T00:00:59.999Z',
        refreshIntervalSeconds: 60,
      }).status,
    ).toBe('fresh');
    expect(
      evaluateValueRateFreshness({
        snapshotId: 'rate-1',
        capturedAt: '2026-01-01T00:00:00.000Z',
        evaluatedAt: '2026-01-01T00:01:00.000Z',
        refreshIntervalSeconds: 60,
      }).status,
    ).toBe('stale');
  });

  it('parses offsets and rejects impossible calendar dates', () => {
    const millis = parseRfc3339Millis('2026-01-01T09:00:00+09:00');
    expect(formatRfc3339Millis(millis)).toBe('2026-01-01T00:00:00.000Z');
    expect(() => parseRfc3339Millis('2026-02-31T00:00:00Z')).toThrow('RFC 3339');
  });
});
