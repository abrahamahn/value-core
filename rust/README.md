# value-core

The Rust implementation of [`value-core`](https://github.com/abrahamahn/value-core#readme):
domain-neutral exact amounts, balanced postings, atomic detached balance application, holds,
reversals, idempotency, statements, conversion plans, and reconciliation. It performs no
infrastructure I/O and uses `serde`/`serde_json` for structured evidence plus the audited `sha2`
implementation for domain-separated hashes.
The public rate and fact modules provide half-even rational rates, deterministic freshness,
gap-free ordering, and pinned-digest validation. The public `time` module provides strict RFC 3339
parsing and canonical UTC formatting without introducing a clock dependency.

```rust
use value_core::transaction::{CanonicalPosting, validate_balanced_transaction};

validate_balanced_transaction(&[
    CanonicalPosting {
        account_id: "source".into(),
        asset: "credits".into(),
        amount_minor: "-10".into(),
    },
    CanonicalPosting {
        account_id: "destination".into(),
        asset: "credits".into(),
        amount_minor: "10".into(),
    },
])?;
```

Run `cargo build --all-targets`, `cargo check --all-targets`,
`cargo clippy --all-targets -- -D warnings`, and `cargo test` from this directory. Applications
retain responsibility for authorization, IDs, clocks, durable atomic persistence, and adapters.
