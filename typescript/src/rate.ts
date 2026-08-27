import { parseAmountMinor } from "./amount.js";
import { formatRfc3339Millis, parseRfc3339Millis } from "./time.js";

export interface RationalRateResult {
  readonly amountMinor: string;
}

/** Apply a positive rational rate to a non-negative amount using bankers' rounding. */
export function multiplyRationalHalfEven(
  amountMinor: string,
  numerator: string,
  denominator: string,
): RationalRateResult {
  const amount = parseAmountMinor(amountMinor);
  const parsedNumerator = parseAmountMinor(numerator);
  const parsedDenominator = parseAmountMinor(denominator);
  if (amount < 0n || parsedNumerator <= 0n || parsedDenominator <= 0n) {
    throw new Error(
      "Value rate requires a non-negative amount and positive rational terms",
    );
  }
  const product = amount * parsedNumerator;
  const quotient = product / parsedDenominator;
  const remainder = product % parsedDenominator;
  const doubled = remainder * 2n;
  const rounded =
    doubled > parsedDenominator ||
    (doubled === parsedDenominator && quotient % 2n !== 0n)
      ? quotient + 1n
      : quotient;
  return Object.freeze({
    amountMinor: parseAmountMinor(rounded.toString()).toString(),
  });
}

export interface ValueRateSnapshotInput {
  readonly snapshotId: string;
  readonly baseAsset: string;
  readonly quoteAsset: string;
  readonly numerator: string;
  readonly denominator: string;
  readonly observedAt: string;
  readonly recordedAt: string;
  readonly effectiveAt: string;
  readonly maxStalenessSeconds: number;
}

export interface ValueRateSnapshot extends ValueRateSnapshotInput {
  readonly expiresAt: string;
}

/** Validate and canonicalize a deterministic rational rate observation. */
export function createValueRateSnapshot(
  input: ValueRateSnapshotInput,
): ValueRateSnapshot {
  if (
    input.snapshotId.trim().length === 0 ||
    input.baseAsset.trim().length === 0 ||
    input.quoteAsset.trim().length === 0
  ) {
    throw new Error("Value rate snapshot identity and assets are required");
  }
  if (input.baseAsset === input.quoteAsset) {
    throw new Error("Value rate snapshot requires distinct assets");
  }
  if (
    parseAmountMinor(input.numerator) <= 0n ||
    parseAmountMinor(input.denominator) <= 0n
  ) {
    throw new Error("Value rate numerator and denominator must be positive");
  }
  if (
    !Number.isSafeInteger(input.maxStalenessSeconds) ||
    input.maxStalenessSeconds <= 0
  ) {
    throw new Error(
      "Value rate maximum staleness must be a positive safe integer",
    );
  }
  const observedAt = parseRfc3339Millis(
    input.observedAt,
    "Value rate observedAt",
  );
  const recordedAt = parseRfc3339Millis(
    input.recordedAt,
    "Value rate recordedAt",
  );
  const effectiveAt = parseRfc3339Millis(
    input.effectiveAt,
    "Value rate effectiveAt",
  );
  if (observedAt > recordedAt || recordedAt > effectiveAt) {
    throw new Error(
      "Value rate observed, recorded, and effective times are out of order",
    );
  }
  const expiresAt = effectiveAt + input.maxStalenessSeconds * 1_000;
  if (!Number.isSafeInteger(expiresAt)) {
    throw new Error("Value rate expiry time is outside the supported range");
  }
  return Object.freeze({
    ...input,
    numerator: parseAmountMinor(input.numerator).toString(),
    denominator: parseAmountMinor(input.denominator).toString(),
    expiresAt: formatRfc3339Millis(expiresAt, "Value rate expiresAt"),
  });
}

export type ValueRateFreshness =
  | {
      readonly status: "fresh";
      readonly retainedSnapshotId: string;
      readonly refreshRequired: false;
    }
  | {
      readonly status: "stale";
      readonly retainedSnapshotId: string;
      readonly refreshRequired: true;
    };

/** Determine whether a retained rate observation has crossed its refresh interval. */
export function evaluateValueRateFreshness(input: {
  readonly snapshotId: string;
  readonly capturedAt: string;
  readonly evaluatedAt: string;
  readonly refreshIntervalSeconds: number;
}): ValueRateFreshness {
  if (input.snapshotId.trim().length === 0)
    throw new Error("Value rate snapshot identity is required");
  if (
    !Number.isSafeInteger(input.refreshIntervalSeconds) ||
    input.refreshIntervalSeconds <= 0
  ) {
    throw new Error(
      "Value rate refresh interval must be a positive safe integer",
    );
  }
  const capturedAt = parseRfc3339Millis(
    input.capturedAt,
    "Value rate capturedAt",
  );
  const evaluatedAt = parseRfc3339Millis(
    input.evaluatedAt,
    "Value rate evaluatedAt",
  );
  const ageMillis = evaluatedAt - capturedAt;
  if (ageMillis < 0)
    throw new Error("Value rate evaluation cannot precede its snapshot");
  return ageMillis >= input.refreshIntervalSeconds * 1_000
    ? Object.freeze({
        status: "stale",
        retainedSnapshotId: input.snapshotId,
        refreshRequired: true,
      })
    : Object.freeze({
        status: "fresh",
        retainedSnapshotId: input.snapshotId,
        refreshRequired: false,
      });
}
