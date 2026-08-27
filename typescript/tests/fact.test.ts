import { describe, expect, it } from 'vitest';

import { orderValueFacts, validatePinnedDigestReferences } from '../src/fact.js';

describe('ordered and pinned value facts', () => {
  it('orders a gap-free fact sequence deterministically', () => {
    expect(
      orderValueFacts([
        { id: 'fact-2', sequence: 2, occurredAt: '2026-01-01T00:00:01.000Z' },
        { id: 'fact-1', sequence: 1, occurredAt: '2026-01-01T00:00:00.000Z' },
      ]).map((fact) => fact.id),
    ).toEqual(['fact-1', 'fact-2']);
  });

  it('rejects gaps and impossible timestamps', () => {
    expect(() =>
      orderValueFacts([{ id: 'fact-2', sequence: 2, occurredAt: '2026-01-01T00:00:00.000Z' }]),
    ).toThrow('gap-free');
    expect(() =>
      orderValueFacts([{ id: 'fact-1', sequence: 1, occurredAt: '2026-02-31T00:00:00.000Z' }]),
    ).toThrow('RFC 3339');
  });

  it('requires every caller-selected artifact to pin the same digest', () => {
    const digest = 'a'.repeat(64);
    expect(
      validatePinnedDigestReferences({
        digest,
        requiredReferences: ['command', 'receipt'],
        references: { command: digest, receipt: digest },
      }),
    ).toEqual({ digest, consistent: true });
    expect(() =>
      validatePinnedDigestReferences({
        digest,
        requiredReferences: ['command', 'receipt'],
        references: { command: digest, receipt: 'b'.repeat(64) },
      }),
    ).toThrow('receipt digest mismatch');
  });
});
