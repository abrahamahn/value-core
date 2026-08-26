use crate::amount::parse_amount_minor;
use crate::{ValueError, ValueResult};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountHistoryPosting {
    pub sequence: u64,
    pub balance_before_minor: String,
    pub amount_minor: String,
    pub balance_after_minor: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FoldedAccountHistory {
    pub final_balance_minor: String,
    pub last_sequence: u64,
}

/// # Errors
/// Returns [`ValueError`] when history is discontinuous or violates its balance policy.
pub fn fold_account_history(
    opening_balance_minor: &str,
    postings: &[AccountHistoryPosting],
    allow_negative: bool,
) -> ValueResult<FoldedAccountHistory> {
    let mut balance = parse_amount_minor(opening_balance_minor)?;
    if balance < 0 && !allow_negative {
        return Err(ValueError::new(
            "Account opening balance cannot be negative",
        ));
    }
    let mut sequence = 0;
    for posting in postings {
        if posting.sequence != sequence + 1 {
            return Err(ValueError::new(
                "Account sequence contains a gap or duplicate",
            ));
        }
        if parse_amount_minor(&posting.balance_before_minor)? != balance {
            return Err(ValueError::new(
                "Account balance-before continuity mismatch",
            ));
        }
        let next = i128::from(balance) + i128::from(parse_amount_minor(&posting.amount_minor)?);
        let stated_next = parse_amount_minor(&posting.balance_after_minor)?;
        if next != i128::from(stated_next) {
            return Err(ValueError::new("Account balance-after continuity mismatch"));
        }
        if stated_next < 0 && !allow_negative {
            return Err(ValueError::new("Account cannot become negative"));
        }
        balance = stated_next;
        sequence = posting.sequence;
    }
    Ok(FoldedAccountHistory {
        final_balance_minor: balance.to_string(),
        last_sequence: sequence,
    })
}
