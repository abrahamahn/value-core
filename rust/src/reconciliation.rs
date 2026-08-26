use std::collections::{BTreeMap, BTreeSet};

use crate::amount::parse_amount_minor;
use crate::{ValueError, ValueResult, required};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconciliationBalance {
    pub account_id: String,
    pub asset: String,
    pub amount_minor: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconciliationDifference {
    pub account_id: String,
    pub asset: String,
    pub expected_minor: String,
    pub actual_minor: String,
    pub difference_minor: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconciliationResult {
    pub closed: bool,
    pub differences: Vec<ReconciliationDifference>,
}

fn index_balances(balances: &[ReconciliationBalance]) -> ValueResult<BTreeMap<(&str, &str), i64>> {
    let mut indexed = BTreeMap::new();
    for balance in balances {
        required(
            &balance.account_id,
            "Reconciliation account identity and asset are required",
        )?;
        required(
            &balance.asset,
            "Reconciliation account identity and asset are required",
        )?;
        let key = (balance.asset.as_str(), balance.account_id.as_str());
        if indexed
            .insert(key, parse_amount_minor(&balance.amount_minor)?)
            .is_some()
        {
            return Err(ValueError::new(
                "Reconciliation balance identity is duplicated",
            ));
        }
    }
    Ok(indexed)
}

/// # Errors
/// Returns [`ValueError`] when either balance set contains malformed or duplicate facts.
pub fn reconcile_balances(
    expected: &[ReconciliationBalance],
    actual: &[ReconciliationBalance],
) -> ValueResult<ReconciliationResult> {
    let expected = index_balances(expected)?;
    let actual = index_balances(actual)?;
    let keys: BTreeSet<_> = expected.keys().chain(actual.keys()).copied().collect();
    let differences = keys
        .into_iter()
        .filter_map(|(asset, account_id)| {
            let expected_minor = expected.get(&(asset, account_id)).copied().unwrap_or(0);
            let actual_minor = actual.get(&(asset, account_id)).copied().unwrap_or(0);
            (expected_minor != actual_minor).then(|| ReconciliationDifference {
                account_id: account_id.into(),
                asset: asset.into(),
                expected_minor: expected_minor.to_string(),
                actual_minor: actual_minor.to_string(),
                difference_minor: (i128::from(actual_minor) - i128::from(expected_minor))
                    .to_string(),
            })
        })
        .collect::<Vec<_>>();
    Ok(ReconciliationResult {
        closed: differences.is_empty(),
        differences,
    })
}
