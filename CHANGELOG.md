# Changelog

## 0.2.0

- Replace the TypeScript conversion catch-all record API with explicit input and result contracts.
- Reject conversion plans and corrections that would create zero-value postings.
- Reject non-permitted negative opening balances before applying a transaction.
- Validate hold integrity at every public lifecycle transition.
- Reject impossible RFC 3339 calendar dates and expose the Rust timestamp primitives.
- Add cross-language canonical JSON/digest vectors and expanded invariant failure tests.

`ValueConversionRecord` is intentionally removed. Consumers should use the exported conversion
interfaces or a quote type satisfying `ValueConversionQuoteIdentity`.
