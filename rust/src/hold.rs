use crate::amount::parse_amount_minor;
use crate::transaction::{CanonicalPosting, validate_balanced_transaction};
use crate::{ValueError, ValueResult, required};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HoldState {
    Open,
    Released,
    Settled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueHold {
    pub hold_id: String,
    pub account_id: String,
    pub asset: String,
    pub amount_minor: String,
    pub state: HoldState,
}

fn validate_hold(hold: &ValueHold) -> ValueResult<i64> {
    required(&hold.hold_id, "Hold identity is required")?;
    required(&hold.account_id, "Hold account is required")?;
    required(&hold.asset, "Hold asset is required")?;
    let amount = parse_amount_minor(&hold.amount_minor)?;
    if amount <= 0 {
        return Err(ValueError::new("Hold amount must be positive"));
    }
    Ok(amount)
}

/// # Errors
/// Returns [`ValueError`] when a hold is malformed or exceeds available value.
pub fn create_value_hold(
    hold_id: &str,
    account_id: &str,
    asset: &str,
    amount_minor: &str,
    available_balance_minor: &str,
) -> ValueResult<ValueHold> {
    required(hold_id, "Hold identity is required")?;
    required(account_id, "Hold account is required")?;
    required(asset, "Hold asset is required")?;
    let amount = parse_amount_minor(amount_minor)?;
    let available = parse_amount_minor(available_balance_minor)?;
    if amount <= 0 {
        return Err(ValueError::new("Hold amount must be positive"));
    }
    if available < amount {
        return Err(ValueError::new("Insufficient available value for hold"));
    }
    Ok(ValueHold {
        hold_id: hold_id.into(),
        account_id: account_id.into(),
        asset: asset.into(),
        amount_minor: amount.to_string(),
        state: HoldState::Open,
    })
}

/// # Errors
/// Returns [`ValueError`] unless the hold is open.
pub fn release_value_hold(hold: &ValueHold) -> ValueResult<ValueHold> {
    if hold.state != HoldState::Open {
        return Err(ValueError::new("Only an open hold can be released"));
    }
    validate_hold(hold)?;
    let mut released = hold.clone();
    released.state = HoldState::Released;
    Ok(released)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HoldSettlement {
    pub hold: ValueHold,
    pub settled_amount_minor: String,
    pub released_amount_minor: String,
    pub postings: Vec<CanonicalPosting>,
}

/// # Errors
/// Returns [`ValueError`] unless an open hold can settle to a distinct destination.
pub fn settle_value_hold(
    hold: &ValueHold,
    destination_account_id: &str,
    amount_minor: Option<&str>,
) -> ValueResult<HoldSettlement> {
    if hold.state != HoldState::Open {
        return Err(ValueError::new("Only an open hold can be settled"));
    }
    let held_amount = validate_hold(hold)?;
    required(
        destination_account_id,
        "Hold settlement destination account is required",
    )?;
    if destination_account_id == hold.account_id {
        return Err(ValueError::new(
            "Hold settlement requires a distinct destination account",
        ));
    }
    let settled = parse_amount_minor(amount_minor.unwrap_or(&hold.amount_minor))?;
    if settled <= 0 || settled > held_amount {
        return Err(ValueError::new(
            "Hold settlement amount must be positive and cannot exceed the hold",
        ));
    }
    let postings = vec![
        CanonicalPosting {
            account_id: hold.account_id.clone(),
            asset: hold.asset.clone(),
            amount_minor: (-settled).to_string(),
        },
        CanonicalPosting {
            account_id: destination_account_id.into(),
            asset: hold.asset.clone(),
            amount_minor: settled.to_string(),
        },
    ];
    validate_balanced_transaction(&postings)?;
    let mut settled_hold = hold.clone();
    settled_hold.state = HoldState::Settled;
    Ok(HoldSettlement {
        hold: settled_hold,
        settled_amount_minor: settled.to_string(),
        released_amount_minor: (held_amount - settled).to_string(),
        postings,
    })
}
