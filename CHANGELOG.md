# Changelog

## 0.3.2

- Execute the shared Hold/idempotency/reconciliation lifecycle corpus in both TypeScript and Rust.
- Add checked Rust RFC 3339 formatting and reject non-canonical four-digit-year rate/statement output.
- Align Rust safe-integer bounds with TypeScript for rate intervals and ordered fact sequences.
- Add direct Rust coverage for the remaining public conversion, transaction-profile, time, and statement
  primitives.

## 0.3.1

- Replace the handwritten Rust SHA-256 implementation with the audited `sha2` crate.
- Execute canonical JSON, exact arithmetic, transaction, reversal, and posting-manifest vectors in
  both TypeScript and Rust.
- Package shared conformance fixtures inside the Rust crate and verify the packaged crate in CI.

## 0.3.0

- Add caller-configurable semantic command payload projection without embedding application field
  policy in the core.
- Add exact half-even rational rate arithmetic and generic rate snapshot/freshness primitives.
- Add deterministic, gap-free value fact ordering and configurable pinned-digest validation.
- Expose strict RFC 3339 parsing and canonical UTC formatting in TypeScript as well as Rust.
- Preserve cross-language behavior for the extracted command, rate, time, and fact invariants.

## 0.2.0

- Replace the TypeScript conversion catch-all record API with explicit input and result contracts.
- Reject conversion plans and corrections that would create zero-value postings.
- Reject non-permitted negative opening balances before applying a transaction.
- Validate hold integrity at every public lifecycle transition.
- Reject impossible RFC 3339 calendar dates and expose the Rust timestamp primitives.
- Add cross-language canonical JSON/digest vectors and expanded invariant failure tests.

`ValueConversionRecord` is intentionally removed. Consumers should use the exported conversion
interfaces or a quote type satisfying `ValueConversionQuoteIdentity`.
