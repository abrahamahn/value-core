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
