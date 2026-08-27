# value-core

`value-core` provides small, deterministic building blocks for applications that own, reserve,
move, account for, convert, or reconcile value. The repository contains two independent,
idiomatic implementations:

- [`typescript/`](typescript/) — npm package `@abrahamahn/value-core`
- [`rust/`](rust/) — Cargo crate `value-core` (imported as `value_core`)

Both implementations are application-independent. They do not know about consumer products,
payments providers, HTTP, databases, queues, or user interfaces.

## Responsibilities

The supported public surface covers:

- canonical signed 64-bit minor-unit parsing and checked arithmetic;
- balanced, non-zero ledger postings with per-asset conservation;
- single-asset transaction validation and deterministic posting manifests;
- atomic application of posting sets to detached account balances;
- exact literal transaction reversals;
- holds, release, full or partial settlement, and settlement posting plans;
- semantic command digests, caller-configurable payload projection, and duplicate conflicts;
- gap-free account-history folding and balance continuity;
- deterministic gap-free value fact ordering and pinned-digest reference validation;
- deterministic account reconciliation differences;
- paged account statements with sequence and balance continuity;
- deterministic cross-asset conversion plans represented as independently balanced transactions;
- exact half-even rational rates with generic snapshots and freshness evaluation;
- strict RFC 3339 calendar validation and canonical UTC formatting in both APIs;
- canonical JSON and domain-separated SHA-256 evidence.

## What this library is not

`value-core` is not a wallet service, payment processor, database schema, event bus, authorization
system, exchange-rate provider, or compliance product. It deliberately contains no casino, wager,
player, game, promotion, provider, PostgreSQL, Redis, HTTP, or WebSocket policy.

Applications decide who may issue commands, which accounts may go negative, how IDs and time are
produced, where facts are stored, and how atomic persistence is implemented. The core validates
and returns deterministic domain values that an application adapter can commit.

## Invariants

- Amounts are canonical decimal integers within the signed 64-bit range.
- Every posting is non-zero and identifies an account and asset.
- Transactions contain at least two postings and conserve every asset independently.
- Balance application validates the complete posting set before returning any new balances.
- Accounts are non-negative by default; negative balances require an explicit account policy.
- Hold amounts are positive, cannot exceed available value, and follow `open → released|settled`.
- Conversion source and destination amounts are positive and must match the pinned rational rate.
- Reversals are exact sign inversions of an already balanced posting set.
- Reusing a command ID with changed contract version or payload is rejected.
- Rate snapshots use positive rational terms, ordered timestamps, and deterministic expiry.
- Account histories and statements are gap-free and balance-continuous.
- Ordered value facts are identity-bearing, timestamp-valid, and gap-free.
- Reconciliation emits exact, deterministically ordered differences.

These are in-process guarantees. An adapter must persist a validated transaction, account updates,
receipt, and related facts within one real storage transaction to provide durable atomicity.

## TypeScript example

```ts
import { applyBalancedTransaction } from '@abrahamahn/value-core';

const balances = applyBalancedTransaction({
  balances: [
    { accountId: 'buyer', asset: 'credits', balanceMinor: '100' },
    { accountId: 'seller', asset: 'credits', balanceMinor: '0' },
  ],
  postings: [
    { accountId: 'buyer', asset: 'credits', amountMinor: '-25' },
    { accountId: 'seller', asset: 'credits', amountMinor: '25' },
  ],
});
```

```bash
cd typescript
pnpm install --frozen-lockfile
pnpm build
pnpm typecheck
pnpm lint
pnpm test
```

## Rust example

Install the crate directly from the repository (pin a tag or revision in production):

```toml
[dependencies]
value-core = { git = "https://github.com/abrahamahn/value-core.git" }
```

```rust
use value_core::transaction::{
    AccountBalance, CanonicalPosting, apply_balanced_transaction,
};

let next = apply_balanced_transaction(
    &[
        AccountBalance {
            account_id: "buyer".into(),
            asset: "credits".into(),
            balance_minor: "100".into(),
            allow_negative: false,
        },
        AccountBalance {
            account_id: "seller".into(),
            asset: "credits".into(),
            balance_minor: "0".into(),
            allow_negative: false,
        },
    ],
    &[
        CanonicalPosting {
            account_id: "buyer".into(),
            asset: "credits".into(),
            amount_minor: "-25".into(),
        },
        CanonicalPosting {
            account_id: "seller".into(),
            asset: "credits".into(),
            amount_minor: "25".into(),
        },
    ],
)?;
```

```bash
cd rust
cargo build --all-targets
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test
```

## Extension and integration

Keep application policy outside the core. A normal integration validates authorization and domain
rules first, asks `value-core` to create or validate postings, then commits the command receipt,
transaction, postings, balances, and outbox facts through an application-owned repository.

Manifest digests support an explicit domain/version profile so an application can preserve an
existing digest contract without placing product terminology in this library. Storage adapters can
map the public account, posting, statement, and reconciliation values to any persistence system.

The TypeScript conversion surface uses explicit input and result interfaces; arbitrary quote
payloads remain usable through the generic replay contract as long as they carry a `quoteId`. The
Rust `time` module exposes strict RFC 3339 parsing and canonical formatting for adapters that need
the same timestamp semantics as statements and quote expiry checks.

The two language implementations intentionally expose ecosystem-idiomatic names while protecting
the same domain invariants. They do not call each other and can be installed independently.

## License

MIT
