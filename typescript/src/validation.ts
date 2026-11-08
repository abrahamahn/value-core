const LOWER_SHA256_PATTERN = /^[0-9a-f]{64}$/u;

export function requireIdentity(value: string, field: string): void {
  if (value.trim().length === 0) {
    throw new Error(`${field} is required`);
  }
}

export function isLowerSha256(value: string): boolean {
  return LOWER_SHA256_PATTERN.test(value);
}

export function isPlainDataObject(
  value: object,
): value is Readonly<Record<string, unknown>> {
  const prototype = Object.getPrototypeOf(value) as object | null;
  return prototype === Object.prototype || prototype === null;
}
