/** Exact signed 64-bit arithmetic for minor units of arbitrary assets. */
const DECIMAL_INTEGER_PATTERN = /^-?(?:0|[1-9][0-9]*)$/u;
const MAX_AMOUNT_MINOR = (1n << 63n) - 1n;
const MIN_AMOUNT_MINOR = -(1n << 63n);

export function parseAmountMinor(value: string): bigint {
  if (!DECIMAL_INTEGER_PATTERN.test(value) || value === "-0") {
    throw new Error("Value amount must be a canonical decimal integer");
  }
  const parsed = BigInt(value);
  if (parsed < MIN_AMOUNT_MINOR || parsed > MAX_AMOUNT_MINOR) {
    throw new Error("Value amount exceeds the supported signed range");
  }
  return parsed;
}

function checkedAmount(value: bigint): bigint {
  if (value < MIN_AMOUNT_MINOR || value > MAX_AMOUNT_MINOR) {
    throw new Error("Value arithmetic overflow");
  }
  return value;
}

function floorDivide(
  numerator: bigint,
  denominator: bigint,
): { quotient: bigint; remainder: bigint } {
  if (denominator <= 0n)
    throw new Error("Rational denominator must be positive");
  let quotient = numerator / denominator;
  let remainder = numerator % denominator;
  if (remainder < 0n) {
    quotient -= 1n;
    remainder += denominator;
  }
  return { quotient, remainder };
}

export type ValueArithmeticInput =
  | { readonly operation: "add"; readonly left: string; readonly right: string }
  | {
      readonly operation: "round_rational";
      readonly numerator: string;
      readonly denominator: string;
      readonly mode: "floor";
    };

export interface ValueArithmeticResult {
  readonly amountMinor: string;
  readonly remainderNumerator?: string;
}

export function evaluateValueArithmetic(
  input: ValueArithmeticInput,
): ValueArithmeticResult {
  if (input.operation === "add") {
    return {
      amountMinor: checkedAmount(
        parseAmountMinor(input.left) + parseAmountMinor(input.right),
      ).toString(),
    };
  }
  const { quotient, remainder } = floorDivide(
    parseAmountMinor(input.numerator),
    parseAmountMinor(input.denominator),
  );
  return {
    amountMinor: quotient.toString(),
    remainderNumerator: remainder.toString(),
  };
}

export function multiplyRationalFloor(
  amountMinor: string,
  numerator: string,
  denominator: string,
): { readonly amountMinor: string; readonly remainderNumerator: string } {
  const product = parseAmountMinor(amountMinor) * parseAmountMinor(numerator);
  const result = floorDivide(product, parseAmountMinor(denominator));
  return {
    amountMinor: checkedAmount(result.quotient).toString(),
    remainderNumerator: result.remainder.toString(),
  };
}
