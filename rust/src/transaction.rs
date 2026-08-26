use serde::Serialize;
use serde_json::to_value;

use crate::amount::parse_amount_minor;
use crate::canonical::domain_separated_digest;
use crate::{ValueError, ValueResult, is_lower_sha256, nonempty};

pub const DEFAULT_POSTING_MANIFEST_DOMAIN: &str = "value-core/posting-manifest";
pub const DEFAULT_POSTING_MANIFEST_CONTRACT_VERSION: &str = "v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PostingManifestDigestProfile<'a> {
    pub domain: &'a str,
    pub contract_version: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalPosting {
    pub account_id: String,
    pub asset: String,
    pub amount_minor: String,
}

fn validate_posting(posting: &CanonicalPosting) -> ValueResult<()> {
    nonempty(&posting.account_id, "Posting account is required")?;
    nonempty(&posting.asset, "Posting asset is required")?;
    if parse_amount_minor(&posting.amount_minor)? == 0 {
        return Err(ValueError::new("Canonical postings must be non-zero"));
    }
    Ok(())
}
/// # Errors
/// Returns `ValueError` when input violates an authority invariant.
pub fn create_posting_manifest_digest(postings: &[CanonicalPosting]) -> ValueResult<String> {
    create_posting_manifest_digest_with_profile(
        postings,
        PostingManifestDigestProfile {
            domain: DEFAULT_POSTING_MANIFEST_DOMAIN,
            contract_version: DEFAULT_POSTING_MANIFEST_CONTRACT_VERSION,
        },
    )
}

/// # Errors
/// Returns [`ValueError`] when postings or the digest profile are invalid.
pub fn create_posting_manifest_digest_with_profile(
    postings: &[CanonicalPosting],
    profile: PostingManifestDigestProfile<'_>,
) -> ValueResult<String> {
    let mut manifest = Vec::with_capacity(postings.len());
    for (index, posting) in postings.iter().enumerate() {
        validate_posting(posting)?;
        manifest.push(serde_json::json!({
            "postingSequence": index + 1,
            "accountId": posting.account_id,
            "asset": posting.asset,
            "amountMinor": posting.amount_minor,
        }));
    }
    domain_separated_digest(
        profile.domain,
        profile.contract_version,
        &to_value(manifest).map_err(|error| ValueError::new(error.to_string()))?,
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountBalance {
    pub account_id: String,
    pub asset: String,
    pub balance_minor: String,
    pub allow_negative: bool,
}

/// Applies a balanced posting set to detached balances without partial mutation.
///
/// # Errors
/// Returns [`ValueError`] when the transaction or any resulting balance is invalid.
pub fn apply_balanced_transaction(
    balances: &[AccountBalance],
    postings: &[CanonicalPosting],
) -> ValueResult<Vec<AccountBalance>> {
    validate_balanced_transaction(postings)?;
    let mut indexed = std::collections::BTreeMap::new();
    for balance in balances {
        nonempty(
            &balance.account_id,
            "Account balance identity and asset are required",
        )?;
        nonempty(
            &balance.asset,
            "Account balance identity and asset are required",
        )?;
        let opening_balance = parse_amount_minor(&balance.balance_minor)?;
        if opening_balance < 0 && !balance.allow_negative {
            return Err(ValueError::new(format!(
                "Account {} cannot start with negative value",
                balance.account_id
            )));
        }
        if indexed
            .insert(balance.account_id.as_str(), balance)
            .is_some()
        {
            return Err(ValueError::new("Account balance identity is duplicated"));
        }
    }
    let mut deltas = std::collections::BTreeMap::<&str, i128>::new();
    for posting in postings {
        let balance = indexed.get(posting.account_id.as_str()).ok_or_else(|| {
            ValueError::new(format!(
                "Posting account {} is unavailable",
                posting.account_id
            ))
        })?;
        if balance.asset != posting.asset {
            return Err(ValueError::new("Posting asset does not match its account"));
        }
        let delta = i128::from(parse_amount_minor(&posting.amount_minor)?);
        let current = deltas.entry(&posting.account_id).or_default();
        *current = current
            .checked_add(delta)
            .ok_or_else(|| ValueError::new("Value arithmetic overflow"))?;
    }
    balances
        .iter()
        .map(|balance| {
            let next = i128::from(parse_amount_minor(&balance.balance_minor)?)
                + deltas
                    .get(balance.account_id.as_str())
                    .copied()
                    .unwrap_or(0);
            let next =
                i64::try_from(next).map_err(|_| ValueError::new("Value arithmetic overflow"))?;
            if next < 0 && !balance.allow_negative {
                return Err(ValueError::new(format!(
                    "Account {} has insufficient value",
                    balance.account_id
                )));
            }
            Ok(AccountBalance {
                account_id: balance.account_id.clone(),
                asset: balance.asset.clone(),
                balance_minor: next.to_string(),
                allow_negative: balance.allow_negative,
            })
        })
        .collect()
}

/// Creates a literal inverse of a balanced transaction.
///
/// # Errors
/// Returns [`ValueError`] when the original transaction is invalid.
pub fn create_transaction_reversal(
    postings: &[CanonicalPosting],
) -> ValueResult<Vec<CanonicalPosting>> {
    validate_balanced_transaction(postings)?;
    let reversal = postings
        .iter()
        .map(|posting| {
            Ok(CanonicalPosting {
                account_id: posting.account_id.clone(),
                asset: posting.asset.clone(),
                amount_minor: parse_amount_minor(&posting.amount_minor)?
                    .checked_neg()
                    .ok_or_else(|| ValueError::new("Value arithmetic overflow"))?
                    .to_string(),
            })
        })
        .collect::<ValueResult<Vec<_>>>()?;
    validate_balanced_transaction(&reversal)?;
    Ok(reversal)
}
/// # Errors
/// Returns `ValueError` when input violates an authority invariant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BalancedTransaction {
    pub total_minor: &'static str,
}

/// # Errors
/// Returns [`ValueError`] when postings are empty, malformed, or unbalanced.
pub fn validate_balanced_transaction(
    postings: &[CanonicalPosting],
) -> ValueResult<BalancedTransaction> {
    if postings.len() < 2 {
        return Err(ValueError::new(
            "A balanced transaction requires at least two postings",
        ));
    }
    let mut totals = std::collections::BTreeMap::<&str, i128>::new();
    for posting in postings {
        validate_posting(posting)?;
        *totals.entry(&posting.asset).or_default() +=
            i128::from(parse_amount_minor(&posting.amount_minor)?);
    }
    if let Some((asset, _)) = totals.iter().find(|(_, total)| **total != 0) {
        return Err(ValueError::new(format!(
            "Posting balance for asset {asset} is non-zero"
        )));
    }
    Ok(BalancedTransaction { total_minor: "0" })
}
/// # Errors
/// Returns `ValueError` when input violates an authority invariant.
pub fn validate_single_asset_transaction(
    asset: &str,
    postings: &[CanonicalPosting],
    debit_means_increase: bool,
) -> ValueResult<SingleAssetTransaction> {
    if debit_means_increase {
        return Err(ValueError::new(
            "Posting sign convention is always the named account perspective",
        ));
    }
    nonempty(asset, "Transaction asset is required")?;
    if postings.iter().any(|posting| posting.asset != asset) {
        return Err(ValueError::new(
            "A canonical transaction may name exactly one asset",
        ));
    }
    validate_balanced_transaction(postings)?;
    Ok(SingleAssetTransaction {
        asset: asset.to_owned(),
        postings: postings.to_vec(),
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SingleAssetTransaction {
    pub asset: String,
    pub postings: Vec<CanonicalPosting>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PostingManifest {
    pub declared_count: usize,
    pub postings: Vec<CanonicalPosting>,
    pub closed: bool,
    pub digest: String,
}
/// # Errors
/// Returns `ValueError` when input violates an authority invariant.
pub fn validate_posting_manifest(
    declared_count: usize,
    postings: &[CanonicalPosting],
    closed: bool,
    digest: &str,
) -> ValueResult<PostingManifest> {
    validate_posting_manifest_with_profile(
        declared_count,
        postings,
        closed,
        digest,
        PostingManifestDigestProfile {
            domain: DEFAULT_POSTING_MANIFEST_DOMAIN,
            contract_version: DEFAULT_POSTING_MANIFEST_CONTRACT_VERSION,
        },
    )
}

/// # Errors
/// Returns [`ValueError`] when a posting manifest violates balance, closure, or digest invariants.
pub fn validate_posting_manifest_with_profile(
    declared_count: usize,
    postings: &[CanonicalPosting],
    closed: bool,
    digest: &str,
    profile: PostingManifestDigestProfile<'_>,
) -> ValueResult<PostingManifest> {
    if !(2..=9_007_199_254_740_991).contains(&declared_count) {
        return Err(ValueError::new(
            "Posting manifest must declare at least two postings",
        ));
    }
    if !closed {
        return Err(ValueError::new(
            "Posting manifest must be closed before commit",
        ));
    }
    if postings.len() != declared_count {
        return Err(ValueError::new(
            "Posting manifest count does not match its closed posting set",
        ));
    }
    validate_balanced_transaction(postings)?;
    if !is_lower_sha256(digest) {
        return Err(ValueError::new(
            "Posting manifest requires a lowercase SHA-256 digest",
        ));
    }
    if digest != create_posting_manifest_digest_with_profile(postings, profile)? {
        return Err(ValueError::new("Posting manifest digest mismatch"));
    }
    Ok(PostingManifest {
        declared_count,
        postings: postings.to_vec(),
        closed,
        digest: digest.to_owned(),
    })
}
