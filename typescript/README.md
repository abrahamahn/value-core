# @abrahamahn/value-core

The TypeScript implementation of
[`value-core`](https://github.com/abrahamahn/value-core#readme): domain-neutral exact amounts,
balanced postings, atomic detached balance application, holds, reversals, idempotency, statements,
conversion plans, and reconciliation. It has no runtime dependencies and no infrastructure or
product assumptions.

```ts
import { validateBalancedTransaction } from '@abrahamahn/value-core';

validateBalancedTransaction([
  { accountId: 'source', asset: 'credits', amountMinor: '-10' },
  { accountId: 'destination', asset: 'credits', amountMinor: '10' },
]);
```

Run `pnpm build`, `pnpm typecheck`, `pnpm lint`, and `pnpm test` from this directory.
Applications retain responsibility for authorization, IDs, clocks, durable atomic persistence,
and infrastructure adapters.
