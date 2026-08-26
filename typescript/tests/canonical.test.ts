import { describe, expect, it } from 'vitest';

import { canonicalJson, domainSeparatedDigest } from '../src/canonical.js';

describe('canonical evidence', () => {
  it('matches the cross-language canonical JSON and digest golden vector', async () => {
    const value = { z: [3, { b: true, a: 'x' }], '\u{e000}': 2, '😀': 1 };

    expect(canonicalJson(value)).toBe('{"z":[3,{"a":"x","b":true}],"😀":1,"":2}');
    await expect(domainSeparatedDigest('value-core/test', 'v1', value)).resolves.toBe(
      'd422be436e96980b0c5d83b09c9e6049d4a1834c16e68fe874bf57b9b7b5de62',
    );
  });

  it('rejects values that cannot have stable data semantics', () => {
    expect(() => canonicalJson({ amount: -0 })).toThrow('negative zero');
    expect(() => canonicalJson({ amount: 1n })).toThrow('unsupported');
    const cyclic: { self?: unknown } = {};
    cyclic.self = cyclic;
    expect(() => canonicalJson(cyclic)).toThrow('cycle');
  });
});
