import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import { canonicalJson, domainSeparatedDigest } from '../src/canonical.js';

describe('canonical evidence', () => {
  it('matches every language-neutral canonical JSON and digest vector', async () => {
    interface CanonicalVector {
      readonly name: string;
      readonly domain: string;
      readonly contractVersion: string;
      readonly value: unknown;
      readonly canonical: string;
      readonly digest: string;
    }
    const fixture = JSON.parse(
      readFileSync(new URL('../../conformance/canonical-v1.json', import.meta.url), 'utf8'),
    ) as { readonly profile: string; readonly vectors: readonly CanonicalVector[] };

    expect(fixture.profile).toBe('value-core-canonical-v1');
    for (const vector of fixture.vectors) {
      expect(canonicalJson(vector.value), vector.name).toBe(vector.canonical);
      await expect(
        domainSeparatedDigest(vector.domain, vector.contractVersion, vector.value),
        vector.name,
      ).resolves.toBe(vector.digest);
    }
  });

  it('rejects values that cannot have stable data semantics', () => {
    expect(() => canonicalJson({ amount: -0 })).toThrow('negative zero');
    expect(() => canonicalJson({ amount: 1n })).toThrow('unsupported');
    const cyclic: { self?: unknown } = {};
    cyclic.self = cyclic;
    expect(() => canonicalJson(cyclic)).toThrow('cycle');
  });
});
