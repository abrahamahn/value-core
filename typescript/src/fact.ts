import { parseRfc3339Millis } from './time.js';

export interface OrderedValueFact {
  readonly id: string;
  readonly sequence: number;
  readonly occurredAt: string;
}

/** Sort value facts deterministically and enforce a positive, gap-free sequence. */
export function orderValueFacts<TFact extends OrderedValueFact>(
  facts: readonly TFact[],
): readonly TFact[] {
  for (const fact of facts) {
    if (fact.id.trim().length === 0) throw new Error('Value fact identity is required');
    if (!Number.isSafeInteger(fact.sequence) || fact.sequence <= 0) {
      throw new Error('Value fact sequence must be a positive safe integer');
    }
    parseRfc3339Millis(fact.occurredAt, 'Value fact timestamp');
  }
  const ordered = [...facts].sort(
    (left, right) => left.sequence - right.sequence || left.id.localeCompare(right.id),
  );
  for (let index = 0; index < ordered.length; index += 1) {
    if (ordered[index]?.sequence !== index + 1) {
      throw new Error('Value fact sequence must be positive and gap-free');
    }
  }
  return Object.freeze(ordered);
}

export function validatePinnedDigestReferences(input: {
  readonly digest: string;
  readonly requiredReferences: readonly string[];
  readonly references: Readonly<Record<string, string>>;
}): { readonly digest: string; readonly consistent: true } {
  if (!/^[0-9a-f]{64}$/u.test(input.digest)) {
    throw new Error('Pinned value digest must be lowercase SHA-256');
  }
  const required = new Set<string>();
  for (const name of input.requiredReferences) {
    if (name.trim().length === 0 || required.has(name)) {
      throw new Error('Pinned value reference names must be non-empty and unique');
    }
    required.add(name);
    const reference = input.references[name];
    if (reference === undefined || reference.length === 0) {
      throw new Error(`Pinned value reference ${name} is missing`);
    }
    if (reference !== input.digest) {
      throw new Error(`Pinned value reference ${name} digest mismatch`);
    }
  }
  return Object.freeze({ digest: input.digest, consistent: true });
}
