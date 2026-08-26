import { parseAmountMinor } from './amount.js';
import { domainSeparatedDigest } from './canonical.js';

export interface PostingManifestDigestProfile {
  readonly domain: string;
  readonly contractVersion: string;
}

export const DEFAULT_POSTING_MANIFEST_DIGEST_PROFILE: PostingManifestDigestProfile = Object.freeze({
  domain: 'value-core/posting-manifest',
  contractVersion: 'v1',
});

export interface CanonicalPostingInput {
  readonly accountId: string;
  readonly asset: string;
  readonly amountMinor: string;
}

export interface PostingManifestInput {
  readonly declaredCount: number;
  readonly postings: readonly CanonicalPostingInput[];
  readonly closed: boolean;
  readonly digest?: string;
}

function assertPostingShape(posting: CanonicalPostingInput): bigint {
  if (posting.accountId.length === 0) throw new Error('Posting account is required');
  if (posting.asset.length === 0) throw new Error('Posting asset is required');
  const amount = parseAmountMinor(posting.amountMinor);
  if (amount === 0n) throw new Error('Canonical postings must be non-zero');
  return amount;
}

function totalsByAsset(postings: readonly CanonicalPostingInput[]): ReadonlyMap<string, bigint> {
  const totals = new Map<string, bigint>();
  for (const posting of postings) {
    const amount = assertPostingShape(posting);
    totals.set(posting.asset, (totals.get(posting.asset) ?? 0n) + amount);
  }
  return totals;
}

export function createPostingManifestDigest(
  postings: readonly CanonicalPostingInput[],
  profile: PostingManifestDigestProfile = DEFAULT_POSTING_MANIFEST_DIGEST_PROFILE,
): Promise<string> {
  for (const posting of postings) assertPostingShape(posting);
  return domainSeparatedDigest(
    profile.domain,
    profile.contractVersion,
    postings.map((posting, index) => ({
      postingSequence: index + 1,
      accountId: posting.accountId,
      asset: posting.asset,
      amountMinor: posting.amountMinor,
    })),
  );
}

export function validateBalancedTransaction(postings: readonly CanonicalPostingInput[]): {
  readonly totalMinor: '0';
} {
  if (postings.length < 2) throw new Error('A balanced transaction requires at least two postings');
  for (const [asset, total] of totalsByAsset(postings)) {
    if (total !== 0n) throw new Error(`Posting balance for asset ${asset} is non-zero`);
  }
  return { totalMinor: '0' };
}

export function validateSingleAssetTransaction(input: {
  readonly asset: string;
  readonly postings: readonly CanonicalPostingInput[];
  readonly debitMeansIncrease?: boolean;
}): {
  readonly asset: string;
  readonly postings: readonly CanonicalPostingInput[];
} {
  if (input.debitMeansIncrease === true) {
    throw new Error('Posting sign convention is always the named account perspective');
  }
  if (input.asset.length === 0) throw new Error('Transaction asset is required');
  if (input.postings.some((posting) => posting.asset !== input.asset)) {
    throw new Error('A canonical transaction may name exactly one asset');
  }
  validateBalancedTransaction(input.postings);
  return { asset: input.asset, postings: input.postings };
}

export async function validatePostingManifest(
  input: PostingManifestInput,
  profile: PostingManifestDigestProfile = DEFAULT_POSTING_MANIFEST_DIGEST_PROFILE,
): Promise<PostingManifestInput> {
  if (!Number.isSafeInteger(input.declaredCount) || input.declaredCount < 2) {
    throw new Error('Posting manifest must declare at least two postings');
  }
  if (!input.closed) throw new Error('Posting manifest must be closed before commit');
  if (input.postings.length !== input.declaredCount) {
    throw new Error('Posting manifest count does not match its closed posting set');
  }
  validateBalancedTransaction(input.postings);
  if (input.digest === undefined || !/^[0-9a-f]{64}$/u.test(input.digest)) {
    throw new Error('Posting manifest requires a lowercase SHA-256 digest');
  }
  if (input.digest !== (await createPostingManifestDigest(input.postings, profile))) {
    throw new Error('Posting manifest digest mismatch');
  }
  return input;
}

export interface AccountBalance {
  readonly accountId: string;
  readonly asset: string;
  readonly balanceMinor: string;
  readonly allowNegative?: boolean;
}

/**
 * Applies all posting deltas to a detached balance set. Validation completes before a result is
 * returned, so callers can persist the result atomically without observing partial mutation.
 */
export function applyBalancedTransaction(input: {
  readonly balances: readonly AccountBalance[];
  readonly postings: readonly CanonicalPostingInput[];
}): readonly AccountBalance[] {
  validateBalancedTransaction(input.postings);
  const balances = new Map<string, AccountBalance>();
  for (const balance of input.balances) {
    if (balances.has(balance.accountId)) throw new Error('Account balance identity is duplicated');
    if (balance.accountId.trim().length === 0 || balance.asset.trim().length === 0) {
      throw new Error('Account balance identity and asset are required');
    }
    parseAmountMinor(balance.balanceMinor);
    balances.set(balance.accountId, balance);
  }

  const deltas = new Map<string, bigint>();
  for (const posting of input.postings) {
    const balance = balances.get(posting.accountId);
    if (balance === undefined)
      throw new Error(`Posting account ${posting.accountId} is unavailable`);
    if (balance.asset !== posting.asset)
      throw new Error('Posting asset does not match its account');
    deltas.set(
      posting.accountId,
      (deltas.get(posting.accountId) ?? 0n) + parseAmountMinor(posting.amountMinor),
    );
  }

  return Object.freeze(
    input.balances.map((balance) => {
      const next = parseAmountMinor(balance.balanceMinor) + (deltas.get(balance.accountId) ?? 0n);
      const checked = parseAmountMinor(next.toString());
      if (checked < 0n && balance.allowNegative !== true) {
        throw new Error(`Account ${balance.accountId} has insufficient value`);
      }
      return Object.freeze({ ...balance, balanceMinor: checked.toString() });
    }),
  );
}

/** Creates the literal inverse of a balanced transaction without changing its asset or accounts. */
export function createTransactionReversal(
  postings: readonly CanonicalPostingInput[],
): readonly CanonicalPostingInput[] {
  validateBalancedTransaction(postings);
  const reversal = Object.freeze(
    postings.map((posting) =>
      Object.freeze({
        ...posting,
        amountMinor: (-parseAmountMinor(posting.amountMinor)).toString(),
      }),
    ),
  );
  validateBalancedTransaction(reversal);
  return reversal;
}
