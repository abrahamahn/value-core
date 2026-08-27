/** Deterministic JSON and domain-separated SHA-256 evidence. */
function compareCodeUnits(left: string, right: string): number {
  if (left < right) return -1;
  if (left > right) return 1;
  return 0;
}

function canonicalize(value: unknown, ancestors: WeakSet<object>): unknown {
  if (
    value === undefined ||
    typeof value === "bigint" ||
    typeof value === "function" ||
    typeof value === "symbol"
  ) {
    throw new Error("Canonical JSON contains an unsupported non-data value");
  }
  if (
    typeof value === "number" &&
    (!Number.isFinite(value) || Object.is(value, -0))
  ) {
    throw new Error(
      "Canonical JSON numbers must be finite and cannot be negative zero",
    );
  }
  if (value === null || typeof value !== "object") return value;
  if (ancestors.has(value))
    throw new Error("Canonical JSON cannot contain a cycle");
  ancestors.add(value);
  try {
    if (Array.isArray(value)) {
      return value.map((item) => canonicalize(item, ancestors));
    }
    const prototype = Object.getPrototypeOf(value) as object | null;
    if (prototype !== Object.prototype && prototype !== null) {
      throw new Error("Canonical JSON accepts only plain data objects");
    }
    return Object.fromEntries(
      Object.entries(value as Readonly<Record<string, unknown>>)
        .sort(([left], [right]) => compareCodeUnits(left, right))
        .map(([key, item]) => [key, canonicalize(item, ancestors)]),
    );
  } finally {
    ancestors.delete(value);
  }
}

export function canonicalJson(value: unknown): string {
  return JSON.stringify(canonicalize(value, new WeakSet<object>()));
}

function toHex(bytes: Uint8Array): string {
  let output = "";
  for (const byte of bytes) output += byte.toString(16).padStart(2, "0");
  return output;
}

export async function domainSeparatedDigest(
  domain: string,
  contractVersion: string,
  value: unknown,
): Promise<string> {
  if (domain.trim().length === 0)
    throw new Error("Canonical digest domain is required");
  if (contractVersion.trim().length === 0) {
    throw new Error("Canonical digest contract version is required");
  }
  const cryptoView: { readonly crypto?: { readonly subtle?: SubtleCrypto } } =
    globalThis;
  if (cryptoView.crypto?.subtle === undefined) {
    throw new Error(
      "Web Crypto SHA-256 is required for canonical value digests",
    );
  }
  const payload = new TextEncoder().encode(
    `${domain}\u0000${contractVersion}\u0000${canonicalJson(value)}`,
  );
  const digest = await cryptoView.crypto.subtle.digest("SHA-256", payload);
  return toHex(new Uint8Array(digest));
}
