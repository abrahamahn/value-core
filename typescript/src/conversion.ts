import { parseAmountMinor } from './amount.js';
import { canonicalJson } from './canonical.js';

type DataRecord = Readonly<Record<string, unknown>>;

export interface ValueConversionPlanInput {
  readonly quoteId: string;
  readonly sourceAsset: string;
  readonly destinationAsset: string;
  readonly sourceAmountMinor: string;
  readonly destinationAmountMinor: string;
  readonly rateNumerator: string;
  readonly rateDenominator: string;
  readonly rounding: 'floor';
}

export interface ValueConversionPosting {
  readonly asset: string;
  readonly amountMinor: string;
}

export interface ValueConversionTransaction {
  readonly asset: string;
  readonly postings: readonly ValueConversionPosting[];
}

export interface ValueConversionPlan {
  readonly quoteId: string;
  readonly transactions: readonly ValueConversionTransaction[];
}

export interface ValueConversionQuoteIdentity {
  readonly quoteId: string;
}

export interface ValueConversionQuoteReplayInput<
  TQuote extends ValueConversionQuoteIdentity = ValueConversionQuoteIdentity,
> {
  readonly existing: TQuote;
  readonly incoming: TQuote;
}

export interface ValueConversionQuoteReplay<
  TQuote extends ValueConversionQuoteIdentity = ValueConversionQuoteIdentity,
> {
  readonly status: 'replayed';
  readonly quote: TQuote;
}

export interface ValueConversionQuote {
  readonly quoteId: string;
  readonly actorId: string;
  readonly rateSnapshotId: string;
  readonly expiresAt: string;
}

export interface ValueConversionQuoteValidationInput {
  readonly quote: ValueConversionQuote;
  readonly actorId: string;
  readonly rateSnapshotId: string;
  readonly evaluatedAt: string;
}

export interface ValidValueConversionQuote {
  readonly status: 'valid';
  readonly quoteId: string;
}

export interface DurableValueConversionReceipt {
  readonly status: string;
  readonly transactionIds?: readonly string[];
}

export interface UnknownValueConversionInput {
  readonly commandId: string;
  readonly durableReceipt?: DurableValueConversionReceipt | null;
}

export type UnknownValueConversionResolution =
  | {
      readonly status: 'succeeded';
      readonly transactionIds: readonly string[];
      readonly resubmitAllowed: false;
    }
  | { readonly status: 'failed'; readonly resubmitAllowed: false }
  | { readonly status: 'unknown'; readonly resubmitAllowed: false };

export interface ValueConversionExecutionInput {
  readonly sourceAmountMinor: string;
  readonly executedSourceMinor: string;
  readonly executedDestinationMinor: string;
  readonly partialExecutionPolicy: 'forbidden' | 'return_unexecuted_source';
}

export interface ValueConversionExecutionSettlement {
  readonly executedSourceMinor: string;
  readonly returnedSourceMinor: string;
  readonly executedDestinationMinor: string;
}

export interface OriginalValueConversion {
  readonly sourceAsset: string;
  readonly sourceAmountMinor: string;
  readonly destinationAsset: string;
  readonly destinationAmountMinor: string;
  readonly rateSnapshotId: string;
}

export interface ValueConversionCorrectionInput {
  readonly original: OriginalValueConversion;
  readonly correctionKind: 'literal_reversal';
  /** Accepted for compatibility; literal reversals intentionally ignore the active market rate. */
  readonly activeRateSnapshotId?: string;
}

export interface ValueConversionCorrection {
  readonly correctionKind: 'literal_reversal';
  readonly sourceAsset: string;
  readonly sourceAmountMinor: string;
  readonly destinationAsset: string;
  readonly destinationAmountMinor: string;
  readonly rateSnapshotId: string;
}

function requireRecord(value: unknown, name: string): DataRecord {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error(`${name} must be a data object`);
  }
  return value as DataRecord;
}

function requireString(record: DataRecord, key: string): string {
  const value = record[key];
  if (typeof value !== 'string' || value.trim().length === 0) {
    throw new Error(`Value conversion ${key} is required`);
  }
  return value;
}

function requireNonNegativeAmount(record: DataRecord, key: string): bigint {
  const parsed = parseAmountMinor(requireString(record, key));
  if (parsed < 0n) throw new Error(`Value conversion ${key} cannot be negative`);
  return parsed;
}

function requirePositiveAmount(record: DataRecord, key: string): bigint {
  const parsed = parseAmountMinor(requireString(record, key));
  if (parsed <= 0n) throw new Error(`Value conversion ${key} must be positive`);
  return parsed;
}

function parseInstant(value: string, name: string): number {
  const match =
    /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.\d+)?(?:Z|([+-])(\d{2}):(\d{2}))$/u.exec(
      value,
    );
  if (match === null) {
    throw new Error(`Value conversion ${name} must be an RFC 3339 instant`);
  }
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const hour = Number(match[4]);
  const minute = Number(match[5]);
  const second = Number(match[6]);
  const offsetHour = match[8] === undefined ? 0 : Number(match[8]);
  const offsetMinute = match[9] === undefined ? 0 : Number(match[9]);
  const leapYear = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
  const daysInMonth = [0, 31, leapYear ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
  if (
    month < 1 ||
    month > 12 ||
    day < 1 ||
    day > (daysInMonth[month] ?? 0) ||
    hour > 23 ||
    minute > 59 ||
    second > 59 ||
    offsetHour > 23 ||
    offsetMinute > 59
  ) {
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

export function buildValueConversionPlan(input: ValueConversionPlanInput): ValueConversionPlan {
  const record = requireRecord(input, 'Value conversion plan');
  const quoteId = requireString(record, 'quoteId');
  const sourceAsset = requireString(record, 'sourceAsset');
  const destinationAsset = requireString(record, 'destinationAsset');
  if (sourceAsset === destinationAsset) {
    throw new Error('Value conversion requires distinct source and destination assets');
  }
  const sourceAmount = requirePositiveAmount(record, 'sourceAmountMinor');
  const destinationAmount = requirePositiveAmount(record, 'destinationAmountMinor');
  const rateNumerator = requirePositiveAmount(record, 'rateNumerator');
  const rateDenominator = requirePositiveAmount(record, 'rateDenominator');
  if (requireString(record, 'rounding') !== 'floor') {
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

export function resolveValueConversionQuoteReplay<TQuote extends ValueConversionQuoteIdentity>(
  input: ValueConversionQuoteReplayInput<TQuote>,
): ValueConversionQuoteReplay<TQuote> {
  const inputRecord = requireRecord(input, 'Value conversion quote replay');
  const existing = requireRecord(inputRecord['existing'], 'Existing value conversion quote');
  const incoming = requireRecord(inputRecord['incoming'], 'Incoming value conversion quote');
  const existingQuoteId = requireString(existing, 'quoteId');
  if (existingQuoteId !== requireString(incoming, 'quoteId')) {
    throw new Error('Value conversion quote identity changed');
  }
  if (canonicalJson(existing) !== canonicalJson(incoming)) {
    throw new Error('Value conversion quote identity was reused with changed semantic intent');
  }
  return Object.freeze({ status: 'replayed', quote: input.existing });
}

export function validateValueConversionQuote(
  input: ValueConversionQuoteValidationInput,
): ValidValueConversionQuote {
  const inputRecord = requireRecord(input, 'Value conversion quote validation');
  const quote = requireRecord(inputRecord['quote'], 'Value conversion quote');
  const quoteId = requireString(quote, 'quoteId');
  if (requireString(inputRecord, 'actorId') !== requireString(quote, 'actorId')) {
    throw new Error('Value conversion quote is not authorized for this actor');
  }
  if (requireString(inputRecord, 'rateSnapshotId') !== requireString(quote, 'rateSnapshotId')) {
    throw new Error('Value conversion rate snapshot changed');
  }
  const evaluatedAt = parseInstant(requireString(inputRecord, 'evaluatedAt'), 'evaluatedAt');
  const expiresAt = parseInstant(requireString(quote, 'expiresAt'), 'expiresAt');
  if (evaluatedAt >= expiresAt) throw new Error('Value conversion quote has expired');
  return Object.freeze({ status: 'valid', quoteId });
}

export function resolveUnknownValueConversion(
  input: UnknownValueConversionInput,
): UnknownValueConversionResolution {
  const inputRecord = requireRecord(input, 'Unknown value conversion');
  requireString(inputRecord, 'commandId');
  const durableReceipt = inputRecord['durableReceipt'];
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

export function settleValueConversionExecution(
  input: ValueConversionExecutionInput,
): ValueConversionExecutionSettlement {
  const record = requireRecord(input, 'Value conversion execution');
  const sourceAmount = requirePositiveAmount(record, 'sourceAmountMinor');
  const executedSource = requireNonNegativeAmount(record, 'executedSourceMinor');
  const executedDestination = requireNonNegativeAmount(record, 'executedDestinationMinor');
  if (executedSource > sourceAmount) {
    throw new Error('Value conversion execution exceeds its quoted source amount');
  }
  const returnedSource = sourceAmount - executedSource;
  const partialExecutionPolicy = requireString(record, 'partialExecutionPolicy');
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

export function planValueConversionCorrection(
  input: ValueConversionCorrectionInput,
): ValueConversionCorrection {
  const inputRecord = requireRecord(input, 'Value conversion correction');
  const original = requireRecord(inputRecord['original'], 'Original value conversion');
  if (requireString(inputRecord, 'correctionKind') !== 'literal_reversal') {
    throw new Error('Unknown value conversion correction kind');
  }
  return Object.freeze({
    correctionKind: 'literal_reversal',
    sourceAsset: requireString(original, 'sourceAsset'),
    sourceAmountMinor: requirePositiveAmount(original, 'sourceAmountMinor').toString(),
    destinationAsset: requireString(original, 'destinationAsset'),
    destinationAmountMinor: requirePositiveAmount(original, 'destinationAmountMinor').toString(),
    rateSnapshotId: requireString(original, 'rateSnapshotId'),
  });
}
