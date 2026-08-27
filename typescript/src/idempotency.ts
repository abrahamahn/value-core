import { domainSeparatedDigest } from './canonical.js';

export interface ValueCommand<TPayload = unknown> {
  readonly commandId: string;
  readonly contractVersion: string;
  readonly payload: TPayload;
}

export interface CommandReplay {
  readonly status: 'replayed';
  readonly digest: string;
}

/**
 * Project a command payload onto its semantic fields. Consumers own the exclusion policy; the
 * core only applies it recursively and rejects data that cannot be canonicalized safely.
 */
export function projectValueCommandPayload(
  value: unknown,
  excludedKeys: ReadonlySet<string> | readonly string[],
  ancestors = new WeakSet<object>(),
): unknown {
  if (value === null || typeof value !== 'object') return value;
  if (ancestors.has(value)) throw new Error('Value command payload cannot contain a cycle');
  const prototype = Object.getPrototypeOf(value) as object | null;
  if (!Array.isArray(value) && prototype !== Object.prototype && prototype !== null) {
    throw new Error('Value command payload accepts only plain data objects');
  }
  const excluded = excludedKeys instanceof Set ? excludedKeys : new Set(excludedKeys);
  ancestors.add(value);
  try {
    if (Array.isArray(value)) {
      return value.map((item) => projectValueCommandPayload(item, excluded, ancestors));
    }
    return Object.fromEntries(
      Object.entries(value as Readonly<Record<string, unknown>>)
        .filter(([key]) => !excluded.has(key))
        .map(([key, item]) => [key, projectValueCommandPayload(item, excluded, ancestors)]),
    );
  } finally {
    ancestors.delete(value);
  }
}

function validateCommand(command: ValueCommand): void {
  if (command.commandId.trim().length === 0) throw new Error('Value command identity is required');
  if (command.contractVersion.trim().length === 0) {
    throw new Error('Value command contract version is required');
  }
}

export function createValueCommandDigest(command: ValueCommand): Promise<string> {
  validateCommand(command);
  return domainSeparatedDigest('value-core/command', command.contractVersion, {
    commandId: command.commandId,
    payload: command.payload,
  });
}

export async function resolveValueCommandReplay(input: {
  readonly existing: ValueCommand;
  readonly incoming: ValueCommand;
}): Promise<CommandReplay> {
  validateCommand(input.existing);
  validateCommand(input.incoming);
  if (input.existing.commandId !== input.incoming.commandId) {
    throw new Error('Value command identity changed');
  }
  const [existingDigest, incomingDigest] = await Promise.all([
    createValueCommandDigest(input.existing),
    createValueCommandDigest(input.incoming),
  ]);
  if (existingDigest !== incomingDigest) {
    throw new Error('Value command identity was reused with changed semantic intent');
  }
  return Object.freeze({ status: 'replayed', digest: existingDigest });
}
