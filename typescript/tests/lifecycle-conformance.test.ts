import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

import {
  createValueHold,
  releaseValueHold,
  settleValueHold,
  type ValueHold,
} from "../src/hold.js";
import {
  createValueCommandDigest,
  resolveValueCommandReplay,
  type ValueCommand,
} from "../src/idempotency.js";
import {
  reconcileBalances,
  type ReconciliationBalance,
} from "../src/reconciliation.js";

interface LifecycleFixture {
  readonly profile: string;
  readonly hold: {
    readonly create: {
      readonly holdId: string;
      readonly accountId: string;
      readonly asset: string;
      readonly amountMinor: string;
      readonly availableBalanceMinor: string;
    };
    readonly open: ValueHold;
    readonly settlement: {
      readonly destinationAccountId: string;
      readonly amountMinor: string;
      readonly settledAmountMinor: string;
      readonly releasedAmountMinor: string;
      readonly state: ValueHold["state"];
      readonly postings: readonly {
        readonly accountId: string;
        readonly asset: string;
        readonly amountMinor: string;
      }[];
    };
    readonly releaseState: ValueHold["state"];
    readonly failures: readonly {
      readonly kind: "over_available" | "same_destination" | "double_release";
      readonly error: string;
    }[];
  };
  readonly idempotency: {
    readonly existing: ValueCommand;
    readonly sameIntent: ValueCommand;
    readonly changedIntent: ValueCommand;
    readonly digest: string;
  };
  readonly reconciliation: {
    readonly expected: readonly ReconciliationBalance[];
    readonly actual: readonly ReconciliationBalance[];
    readonly closed: boolean;
    readonly differences: readonly {
      readonly accountId: string;
      readonly asset: string;
      readonly expectedMinor: string;
      readonly actualMinor: string;
      readonly differenceMinor: string;
    }[];
  };
}

const fixture = JSON.parse(
  readFileSync(
    new URL("../../rust/fixtures/lifecycle-v1.json", import.meta.url),
    "utf8",
  ),
) as LifecycleFixture;

describe("cross-language lifecycle conformance", () => {
  it("matches Hold placement, settlement, release, and failure vectors", () => {
    expect(fixture.profile).toBe("value-core-lifecycle-v1");
    const hold = createValueHold(fixture.hold.create);
    expect(hold).toEqual(fixture.hold.open);
    expect(
      settleValueHold({
        hold,
        destinationAccountId: fixture.hold.settlement.destinationAccountId,
        amountMinor: fixture.hold.settlement.amountMinor,
      }),
    ).toEqual({
      hold: { ...hold, state: fixture.hold.settlement.state },
      settledAmountMinor: fixture.hold.settlement.settledAmountMinor,
      releasedAmountMinor: fixture.hold.settlement.releasedAmountMinor,
      postings: fixture.hold.settlement.postings,
    });
    expect(releaseValueHold(hold).state).toBe(fixture.hold.releaseState);

    for (const failure of fixture.hold.failures) {
      const operation = (): unknown => {
        switch (failure.kind) {
          case "over_available":
            return createValueHold({
              ...fixture.hold.create,
              amountMinor: "101",
            });
          case "same_destination":
            return settleValueHold({
              hold,
              destinationAccountId: hold.accountId,
            });
          case "double_release":
            return releaseValueHold(releaseValueHold(hold));
        }
      };
      expect(operation).toThrow(failure.error);
    }
  });

  it("matches semantic replay and command digest vectors", async () => {
    await expect(
      createValueCommandDigest(fixture.idempotency.existing),
    ).resolves.toBe(fixture.idempotency.digest);
    await expect(
      resolveValueCommandReplay({
        existing: fixture.idempotency.existing,
        incoming: fixture.idempotency.sameIntent,
      }),
    ).resolves.toEqual({
      status: "replayed",
      digest: fixture.idempotency.digest,
    });
    await expect(
      resolveValueCommandReplay({
        existing: fixture.idempotency.existing,
        incoming: fixture.idempotency.changedIntent,
      }),
    ).rejects.toThrow("changed semantic intent");
  });

  it("matches deterministic reconciliation difference vectors", () => {
    expect(
      reconcileBalances({
        expected: fixture.reconciliation.expected,
        actual: fixture.reconciliation.actual,
      }),
    ).toEqual({
      closed: fixture.reconciliation.closed,
      differences: fixture.reconciliation.differences,
    });
  });
});
