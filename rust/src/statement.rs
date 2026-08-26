use std::collections::BTreeSet;

use crate::amount::parse_amount_minor;
use crate::time::{format_rfc3339_millis, parse_rfc3339_millis};
use crate::{ValueError, ValueResult, required};

pub const MAX_STATEMENT_PAGE_SIZE: usize = 500;

#[must_use]
pub fn canonical_statement_timestamp(millis: i64) -> String {
    format_rfc3339_millis(millis)
}

/// # Errors
/// Returns an error when the timestamp is not an RFC 3339 instant.
pub fn normalize_statement_timestamp(value: &str) -> ValueResult<String> {
    parse_rfc3339_millis(value, "Value statement timestamp").map(format_rfc3339_millis)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatementPosting {
    pub posting_id: String,
    pub transaction_id: String,
    pub account_id: String,
    pub account_sequence: String,
    pub posting_sequence: u64,
    pub asset: String,
    pub amount_minor: String,
    pub balance_before_minor: String,
    pub balance_after_minor: String,
    pub occurred_at: String,
    pub recorded_at: String,
    pub source_namespace: String,
    pub source_type: String,
    pub source_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatementCursor {
    pub account_sequence: String,
    pub posting_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatementPage {
    pub account_id: String,
    pub asset: String,
    pub previous_account_sequence: String,
    pub opening_balance_minor: String,
    pub closing_account_sequence: String,
    pub closing_balance_minor: String,
    pub entries: Vec<StatementPosting>,
    pub has_more: bool,
    pub next_cursor: Option<StatementCursor>,
}
/// # Errors
/// Returns `ValueError` when input violates an authority invariant.
#[allow(clippy::too_many_lines)]
pub fn build_value_statement_page(
    account_id: &str,
    asset: &str,
    previous_account_sequence: &str,
    opening_balance_minor: &str,
    limit: usize,
    postings: &[StatementPosting],
) -> ValueResult<StatementPage> {
    required(account_id, "Value statement account is required")?;
    required(asset, "Value statement asset is required")?;
    if !(1..=MAX_STATEMENT_PAGE_SIZE).contains(&limit) {
        return Err(ValueError::new("Value statement page size is invalid"));
    }
    let previous_sequence = parse_amount_minor(previous_account_sequence)?;
    if previous_sequence < 0 {
        return Err(ValueError::new(
            "Value statement account sequence is invalid",
        ));
    }
    let mut ordered = postings.to_vec();
    ordered
        .sort_by_key(|posting| parse_amount_minor(&posting.account_sequence).unwrap_or(i64::MIN));
    let mut expected = i128::from(previous_sequence) + 1;
    let mut balance = parse_amount_minor(opening_balance_minor)?;
    let mut posting_ids = BTreeSet::new();
    let mut sequences = BTreeSet::new();
    for posting in &ordered {
        required(&posting.posting_id, "Value statement posting is required")?;
        required(
            &posting.transaction_id,
            "Value statement transaction is required",
        )?;
        required(
            &posting.source_namespace,
            "Value statement source namespace is required",
        )?;
        required(
            &posting.source_type,
            "Value statement source type is required",
        )?;
        required(&posting.source_id, "Value statement source is required")?;
        for (timestamp, field) in [
            (&posting.occurred_at, "Value statement occurrence time"),
            (&posting.recorded_at, "Value statement record time"),
        ] {
            let parsed = parse_rfc3339_millis(timestamp, field)
                .map_err(|_| ValueError::new(format!("{field} must be a canonical timestamp")))?;
            if format_rfc3339_millis(parsed) != *timestamp {
                return Err(ValueError::new(format!(
                    "{field} must be a canonical timestamp"
                )));
            }
        }
        if posting.account_id != account_id {
            return Err(ValueError::new(
                "Value statement contains a foreign account",
            ));
        }
        if posting.asset != asset {
            return Err(ValueError::new(
                "Value statement contains a cross-asset posting",
            ));
        }
        if posting.posting_sequence == 0 || posting.posting_sequence > 9_007_199_254_740_991 {
            return Err(ValueError::new(
                "Value statement posting sequence is invalid",
            ));
        }
        let account_sequence = parse_amount_minor(&posting.account_sequence)?;
        if account_sequence <= 0 {
            return Err(ValueError::new(
                "Value statement account sequence is invalid",
            ));
        }
        if !posting_ids.insert(posting.posting_id.clone())
            || !sequences.insert(posting.account_sequence.clone())
        {
            return Err(ValueError::new(
                "Value statement contains a duplicate posting identity or sequence",
            ));
        }
        if i128::from(account_sequence) != expected {
            return Err(ValueError::new(
                "Value statement account sequence is not gap-free",
            ));
        }
        let amount_minor = parse_amount_minor(&posting.amount_minor)?;
        if amount_minor == 0 {
            return Err(ValueError::new("Value statement posting must be non-zero"));
        }
        let balance_before_minor = parse_amount_minor(&posting.balance_before_minor)?;
        let balance_after_minor = parse_amount_minor(&posting.balance_after_minor)?;
        if balance_before_minor != balance {
            return Err(ValueError::new(
                "Value statement balance-before continuity mismatch",
            ));
        }
        if i128::from(balance_after_minor) != i128::from(balance) + i128::from(amount_minor) {
            return Err(ValueError::new(
                "Value statement balance-after continuity mismatch",
            ));
        }
        balance = balance_after_minor;
        expected += 1;
    }
    let entries: Vec<_> = ordered.into_iter().take(limit).collect();
    let has_more = postings.len() > entries.len();
    let closing_sequence = entries.last().map_or_else(
        || previous_sequence.to_string(),
        |posting| posting.account_sequence.clone(),
    );
    let closing_balance = entries.last().map_or_else(
        || {
            parse_amount_minor(opening_balance_minor)
                .unwrap_or_default()
                .to_string()
        },
        |posting| posting.balance_after_minor.clone(),
    );
    let next_cursor = has_more
        .then(|| {
            entries.last().map(|posting| StatementCursor {
                account_sequence: posting.account_sequence.clone(),
                posting_id: posting.posting_id.clone(),
            })
        })
        .flatten();
    Ok(StatementPage {
        account_id: account_id.to_owned(),
        asset: asset.to_owned(),
        previous_account_sequence: previous_sequence.to_string(),
        opening_balance_minor: parse_amount_minor(opening_balance_minor)?.to_string(),
        closing_account_sequence: closing_sequence,
        closing_balance_minor: closing_balance,
        entries,
        has_more,
        next_cursor,
    })
}
