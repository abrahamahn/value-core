import { describe, expect, it } from 'vitest';

import { projectValueCommandPayload, resolveValueCommandReplay } from '../src/idempotency.js';

describe('value command idempotency', () => {
  it('replays identical semantic intent', async () => {
    const command = {
      commandId: 'command-1',
      contractVersion: 'v1',
      payload: { amountMinor: '7', asset: 'credits' },
    };
    await expect(
      resolveValueCommandReplay({ existing: command, incoming: command }),
    ).resolves.toEqual(expect.objectContaining({ status: 'replayed' }));
  });

  it('rejects a duplicate command identity with changed intent', async () => {
    await expect(
      resolveValueCommandReplay({
        existing: {
          commandId: 'command-1',
          contractVersion: 'v1',
          payload: { amount: '7' },
        },
        incoming: {
          commandId: 'command-1',
          contractVersion: 'v1',
          payload: { amount: '8' },
        },
      }),
    ).rejects.toThrow('changed semantic intent');
  });

  it('projects consumer-selected non-semantic fields recursively', () => {
    expect(
      projectValueCommandPayload(
        {
          amountMinor: '7',
          presentation: { label: 'new' },
          nested: { traceId: 'trace-1', stable: true },
        },
        ['presentation', 'traceId'],
      ),
    ).toEqual({ amountMinor: '7', nested: { stable: true } });
  });

  it('rejects cyclic or non-data command payloads', () => {
    const cyclic: Record<string, unknown> = {};
    cyclic['self'] = cyclic;
    expect(() => projectValueCommandPayload(cyclic, [])).toThrow('cycle');
    expect(() => projectValueCommandPayload(new Date(), [])).toThrow('plain data');
  });
});
