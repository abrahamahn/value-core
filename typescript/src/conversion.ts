import { parseAmountMinor } from './amount.js';
import { canonicalJson } from './canonical.js';

export type ValueConversionRecord = Readonly<Record<string, unknown>>;

export interface ValueConversionPosting {
  readonly asset: string;
  readonly amountMinor: string;
}

export interface ValueConversionTransaction {
  readonly asset: string;
  readonly postings: readonly ValueConversionPosting[];
}

function requireRecord(value: unknown, name: string): ValueConversionRecord {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error(`${name} must be a data object`);
  }
  return value as ValueConversionRecord;
}

function requireString(record: ValueConversionRecord, key: string): string {
  const value = record[key];
  if (typeof value !== 'string' || value.trim().length === 0) {
    throw new Error(`Value conversion ${key} is required`);
  }
  return value;
}

function requireNonNegativeAmount(record: ValueConversionRecord, key: string): bigint {
  const parsed = parseAmountMinor(requireString(record, key));
  if (parsed < 0n) throw new Error(`Value conversion ${key} cannot be negative`);
  return parsed;
}

function requirePositiveAmount(record: ValueConversionRecord, key: string): bigint {
  const parsed = parseAmountMinor(requireString(record, key));
  if (parsed <= 0n) throw new Error(`Value conversion ${key} must be positive`);
  return parsed;
}

function parseInstant(value: string, name: string): number {
  if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/u.test(value)) {
    throw new Error(`Value conversion ${name} must be an RFC 3339 instant`);
  }
  const parsed = Date.parse(value);
  if (!Number.isFinite(parsed)) {
    throw new Error(`Value conversion ${name} must be an RFC 3339 instant`);
  }
  return parsed;
}

function balancedTransaction(asset: string, amountMinor: bigint): ValueConversionTransaction {
  return Object.freeze({
    asset,
    postings: Object.freeze([
      Object.freeze({ asset, amountMinor: (-amountMinor).toString() }),
      Object.freeze({ asset, amountMinor: amountMinor.toString() }),
    ]),
  });
}

export function buildValueConversionPlan(input: ValueConversionRecord): {
  readonly quoteId: string;
  readonly transactions: readonly ValueConversionTransaction[];
} {
  const quoteId = requireString(input, 'quoteId');
  const sourceAsset = requireString(input, 'sourceAsset');
  const destinationAsset = requireString(input, 'destinationAsset');
  if (sourceAsset === destinationAsset) {
    throw new Error('Value conversion requires distinct source and destination assets');
  }
  const sourceAmount = requirePositiveAmount(input, 'sourceAmountMinor');
  const destinationAmount = requireNonNegativeAmount(input, 'destinationAmountMinor');
  const rateNumerator = requirePositiveAmount(input, 'rateNumerator');
  const rateDenominator = requirePositiveAmount(input, 'rateDenominator');
  if (requireString(input, 'rounding') !== 'floor') {
    throw new Error('Value conversion plan requires an explicit supported rounding profile');
  }
  const expectedDestination = (sourceAmount * rateNumerator) / rateDenominator;
  if (destinationAmount !== expectedDestination) {
    throw new Error('Value conversion destination amount does not match its pinned rate');
  }
  return Object.freeze({
    quoteId,
    transactions: Object.freeze([
      balancedTransaction(sourceAsset, sourceAmount),
      balancedTransaction(destinationAsset, destinationAmount),
    ]),
  });
}

export function resolveValueConversionQuoteReplay(input: ValueConversionRecord): {
  readonly status: 'replayed';
  readonly quote: ValueConversionRecord;
} {
  const existing = requireRecord(input['existing'], 'Existing value conversion quote');
  const incoming = requireRecord(input['incoming'], 'Incoming value conversion quote');
  const existingQuoteId = requireString(existing, 'quoteId');
  if (existingQuoteId !== requireString(incoming, 'quoteId')) {
    throw new Error('Value conversion quote identity changed');
  }
  if (canonicalJson(existing) !== canonicalJson(incoming)) {
    throw new Error('Value conversion quote identity was reused with changed semantic intent');
  }
  return Object.freeze({ status: 'replayed', quote: existing });
}

export function validateValueConversionQuote(input: ValueConversionRecord): {
  readonly status: 'valid';
  readonly quoteId: string;
} {
  const quote = requireRecord(input['quote'], 'Value conversion quote');
  const quoteId = requireString(quote, 'quoteId');
  if (requireString(input, 'actorId') !== requireString(quote, 'actorId')) {
    throw new Error('Value conversion quote is not authorized for this actor');
  }
  if (requireString(input, 'rateSnapshotId') !== requireString(quote, 'rateSnapshotId')) {
    throw new Error('Value conversion rate snapshot changed');
  }
  const evaluatedAt = parseInstant(requireString(input, 'evaluatedAt'), 'evaluatedAt');
  const expiresAt = parseInstant(requireString(quote, 'expiresAt'), 'expiresAt');
  if (evaluatedAt >= expiresAt) throw new Error('Value conversion quote has expired');
  return Object.freeze({ status: 'valid', quoteId });
}

export function resolveUnknownValueConversion(input: ValueConversionRecord):
  | {
      readonly status: 'succeeded';
      readonly transactionIds: readonly string[];
      readonly resubmitAllowed: false;
    }
  | { readonly status: 'failed'; readonly resubmitAllowed: false }
  | { readonly status: 'unknown'; readonly resubmitAllowed: false } {
  requireString(input, 'commandId');
  const durableReceipt = input['durableReceipt'];
  if (durableReceipt === null || durableReceipt === undefined) {
    return Object.freeze({ status: 'unknown', resubmitAllowed: false });
  }
  const receipt = requireRecord(durableReceipt, 'Value conversion durable receipt');
  const status = requireString(receipt, 'status');
  if (status === 'failed') return Object.freeze({ status, resubmitAllowed: false });
  if (status !== 'succeeded') {
    return Object.freeze({ status: 'unknown', resubmitAllowed: false });
  }
  const transactionIds = receipt['transactionIds'];
  if (
    !Array.isArray(transactionIds) ||
    transactionIds.length === 0 ||
    !transactionIds.every((transactionId) =>
      typeof transactionId === 'string' ? transactionId.trim().length > 0 : false,
    )
  ) {
    throw new Error('Successful value conversion receipt requires transaction identities');
  }
  return Object.freeze({
    status,
    transactionIds: Object.freeze(transactionIds as string[]),
    resubmitAllowed: false,
  });
}

export function settleValueConversionExecution(input: ValueConversionRecord): {
  readonly executedSourceMinor: string;
  readonly returnedSourceMinor: string;
  readonly executedDestinationMinor: string;
} {
  const sourceAmount = requirePositiveAmount(input, 'sourceAmountMinor');
  const executedSource = requireNonNegativeAmount(input, 'executedSourceMinor');
  const executedDestination = requireNonNegativeAmount(input, 'executedDestinationMinor');
  if (executedSource > sourceAmount) {
    throw new Error('Value conversion execution exceeds its quoted source amount');
  }
  const returnedSource = sourceAmount - executedSource;
  const partialExecutionPolicy = requireString(input, 'partialExecutionPolicy');
  if (returnedSource > 0n && partialExecutionPolicy === 'forbidden') {
    throw new Error('Partial value conversion execution is forbidden by its pinned profile');
  }
  if (
    partialExecutionPolicy !== 'forbidden' &&
    partialExecutionPolicy !== 'return_unexecuted_source'
  ) {
    throw new Error('Unknown value conversion partial-execution profile');
  }
  return Object.freeze({
    executedSourceMinor: executedSource.toString(),
    returnedSourceMinor: returnedSource.toString(),
    executedDestinationMinor: executedDestination.toString(),
  });
}

export function planValueConversionCorrection(input: ValueConversionRecord): {
  readonly correctionKind: 'literal_reversal';
  readonly sourceAsset: string;
  readonly sourceAmountMinor: string;
  readonly destinationAsset: string;
  readonly destinationAmountMinor: string;
  readonly rateSnapshotId: string;
} {
  const original = requireRecord(input['original'], 'Original value conversion');
  if (requireString(input, 'correctionKind') !== 'literal_reversal') {
    throw new Error('Unknown value conversion correction kind');
  }
  return Object.freeze({
    correctionKind: 'literal_reversal',
    sourceAsset: requireString(original, 'sourceAsset'),
    sourceAmountMinor: requireNonNegativeAmount(original, 'sourceAmountMinor').toString(),
    destinationAsset: requireString(original, 'destinationAsset'),
    destinationAmountMinor: requireNonNegativeAmount(original, 'destinationAmountMinor').toString(),
    rateSnapshotId: requireString(original, 'rateSnapshotId'),
  });
}
