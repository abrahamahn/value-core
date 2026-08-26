use serde_json::Value;

use crate::amount::parse_amount_minor;
use crate::canonical::canonical_json;
use crate::time::parse_rfc3339_millis;
use crate::{ValueError, ValueResult, required};

fn required_amount(value: &str, key: &str) -> ValueResult<i64> {
    required(value, &format!("Value conversion {key} is required"))?;
    parse_amount_minor(value)
}

fn positive_amount(value: &str, key: &str) -> ValueResult<i64> {
    let amount = required_amount(value, key)?;
    if amount <= 0 {
        return Err(ValueError::new(format!(
            "Value conversion {key} must be positive"
        )));
    }
    Ok(amount)
}

fn non_negative_amount(value: &str, key: &str) -> ValueResult<i64> {
    let amount = required_amount(value, key)?;
    if amount < 0 {
        return Err(ValueError::new(format!(
            "Value conversion {key} cannot be negative"
        )));
    }
    Ok(amount)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversionPosting {
    pub asset: String,
    pub amount_minor: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversionTransaction {
    pub asset: String,
    pub postings: Vec<ConversionPosting>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversionPlan {
    pub quote_id: String,
    pub transactions: Vec<ConversionTransaction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversionQuoteReplay {
    pub status: &'static str,
    pub quote: Value,
}

fn balanced(asset: &str, amount: i64) -> ConversionTransaction {
    ConversionTransaction {
        asset: asset.to_owned(),
        postings: vec![
            ConversionPosting {
                asset: asset.to_owned(),
                amount_minor: (-amount).to_string(),
            },
            ConversionPosting {
                asset: asset.to_owned(),
                amount_minor: amount.to_string(),
            },
        ],
    }
}

#[allow(clippy::too_many_arguments)]
/// # Errors
/// Returns `ValueError` when input violates an authority invariant.
pub fn build_value_conversion_plan(
    quote_id: &str,
    source_asset: &str,
    destination_asset: &str,
    source_amount_minor: &str,
    destination_amount_minor: &str,
    rate_numerator: &str,
    rate_denominator: &str,
    rounding: &str,
) -> ValueResult<ConversionPlan> {
    required(quote_id, "Value conversion quoteId is required")?;
    required(source_asset, "Value conversion sourceAsset is required")?;
    required(
        destination_asset,
        "Value conversion destinationAsset is required",
    )?;
    if source_asset == destination_asset {
        return Err(ValueError::new(
            "Value conversion requires distinct source and destination assets",
        ));
    }
    let source_amount_minor = positive_amount(source_amount_minor, "sourceAmountMinor")?;
    let destination_amount_minor =
        non_negative_amount(destination_amount_minor, "destinationAmountMinor")?;
    let rate_numerator = positive_amount(rate_numerator, "rateNumerator")?;
    let rate_denominator = positive_amount(rate_denominator, "rateDenominator")?;
    if rounding != "floor" {
        return Err(ValueError::new(
            "Value conversion plan requires an explicit supported rounding profile",
        ));
    }
    let expected =
        i128::from(source_amount_minor) * i128::from(rate_numerator) / i128::from(rate_denominator);
    if expected != i128::from(destination_amount_minor) {
        return Err(ValueError::new(
            "Value conversion destination amount does not match its pinned rate",
        ));
    }
    Ok(ConversionPlan {
        quote_id: quote_id.to_owned(),
        transactions: vec![
            balanced(source_asset, source_amount_minor),
            balanced(destination_asset, destination_amount_minor),
        ],
    })
}
/// # Errors
/// Returns `ValueError` when input violates an authority invariant.
pub fn resolve_value_conversion_quote_replay(
    existing: &Value,
    incoming: &Value,
) -> ValueResult<ConversionQuoteReplay> {
    if !existing.is_object() {
        return Err(ValueError::new(
            "Existing value conversion quote must be a data object",
        ));
    }
    if !incoming.is_object() {
        return Err(ValueError::new(
            "Incoming value conversion quote must be a data object",
        ));
    }
    let existing_id = existing
        .get("quoteId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let incoming_id = incoming
        .get("quoteId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    required(existing_id, "Value conversion quoteId is required")?;
    required(incoming_id, "Value conversion quoteId is required")?;
    if existing_id != incoming_id {
        return Err(ValueError::new("Value conversion quote identity changed"));
    }
    if canonical_json(existing)? != canonical_json(incoming)? {
        return Err(ValueError::new(
            "Value conversion quote identity was reused with changed semantic intent",
        ));
    }
    Ok(ConversionQuoteReplay {
        status: "replayed",
        quote: existing.clone(),
    })
}
/// # Errors
/// Returns `ValueError` when input violates an authority invariant.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidConversionQuote {
    pub status: &'static str,
    pub quote_id: String,
}

/// # Errors
/// Returns [`ValueError`] when quote identity, assets, amount, or expiry is invalid.
pub fn validate_value_conversion_quote(
    quote_id: &str,
    quote_actor_id: &str,
    actor_id: &str,
    quote_rate_snapshot_id: &str,
    rate_snapshot_id: &str,
    evaluated_at: &str,
    expires_at: &str,
) -> ValueResult<ValidConversionQuote> {
    required(quote_id, "Value conversion quoteId is required")?;
    required(actor_id, "Value conversion actorId is required")?;
    required(quote_actor_id, "Value conversion actorId is required")?;
    if actor_id != quote_actor_id {
        return Err(ValueError::new(
            "Value conversion quote is not authorized for this actor",
        ));
    }
    required(
        rate_snapshot_id,
        "Value conversion rateSnapshotId is required",
    )?;
    required(
        quote_rate_snapshot_id,
        "Value conversion rateSnapshotId is required",
    )?;
    if rate_snapshot_id != quote_rate_snapshot_id {
        return Err(ValueError::new("Value conversion rate snapshot changed"));
    }
    required(evaluated_at, "Value conversion evaluatedAt is required")?;
    required(expires_at, "Value conversion expiresAt is required")?;
    if parse_rfc3339_millis(evaluated_at, "Value conversion evaluatedAt")?
        >= parse_rfc3339_millis(expires_at, "Value conversion expiresAt")?
    {
        return Err(ValueError::new("Value conversion quote has expired"));
    }
    Ok(ValidConversionQuote {
        status: "valid",
        quote_id: quote_id.to_owned(),
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnknownConversionResolution {
    Succeeded {
        transaction_ids: Vec<String>,
        resubmit_allowed: bool,
    },
    Failed {
        resubmit_allowed: bool,
    },
    Unknown {
        resubmit_allowed: bool,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DurableConversionReceipt {
    pub status: String,
    pub transaction_ids: Vec<String>,
}
/// # Errors
/// Returns `ValueError` when input violates an authority invariant.
pub fn resolve_unknown_value_conversion(
    command_id: &str,
    durable_receipt: Option<&DurableConversionReceipt>,
) -> ValueResult<UnknownConversionResolution> {
    required(command_id, "Value conversion commandId is required")?;
    let Some(receipt) = durable_receipt else {
        return Ok(UnknownConversionResolution::Unknown {
            resubmit_allowed: false,
        });
    };
    match receipt.status.as_str() {
        status if status.trim().is_empty() => {
            Err(ValueError::new("Value conversion status is required"))
        }
        "failed" => Ok(UnknownConversionResolution::Failed {
            resubmit_allowed: false,
        }),
        "succeeded" => {
            if receipt.transaction_ids.is_empty()
                || receipt
                    .transaction_ids
                    .iter()
                    .any(|id| id.trim().is_empty())
            {
                Err(ValueError::new(
                    "Successful value conversion receipt requires transaction identities",
                ))
            } else {
                Ok(UnknownConversionResolution::Succeeded {
                    transaction_ids: receipt.transaction_ids.clone(),
                    resubmit_allowed: false,
                })
            }
        }
        _ => Ok(UnknownConversionResolution::Unknown {
            resubmit_allowed: false,
        }),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversionExecution {
    pub executed_source_minor: String,
    pub returned_source_minor: String,
    pub executed_destination_minor: String,
}
/// # Errors
/// Returns `ValueError` when input violates an authority invariant.
pub fn settle_value_conversion_execution(
    source_amount_minor: &str,
    executed_source_minor: &str,
    executed_destination_minor: &str,
    policy: &str,
) -> ValueResult<ConversionExecution> {
    let source_amount_minor = positive_amount(source_amount_minor, "sourceAmountMinor")?;
    let executed_source_minor = non_negative_amount(executed_source_minor, "executedSourceMinor")?;
    let executed_destination_minor =
        non_negative_amount(executed_destination_minor, "executedDestinationMinor")?;
    if executed_source_minor > source_amount_minor {
        return Err(ValueError::new(
            "Value conversion execution exceeds its quoted source amount",
        ));
    }
    let returned = source_amount_minor - executed_source_minor;
    if returned > 0 && policy == "forbidden" {
        return Err(ValueError::new(
            "Partial value conversion execution is forbidden by its pinned profile",
        ));
    }
    if !matches!(policy, "forbidden" | "return_unexecuted_source") {
        return Err(ValueError::new(
            "Unknown value conversion partial-execution profile",
        ));
    }
    Ok(ConversionExecution {
        executed_source_minor: executed_source_minor.to_string(),
        returned_source_minor: returned.to_string(),
        executed_destination_minor: executed_destination_minor.to_string(),
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversionCorrection {
    pub correction_kind: String,
    pub source_asset: String,
    pub source_amount_minor: String,
    pub destination_asset: String,
    pub destination_amount_minor: String,
    pub rate_snapshot_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OriginalConversion {
    pub source_asset: String,
    pub source_amount_minor: String,
    pub destination_asset: String,
    pub destination_amount_minor: String,
    pub rate_snapshot_id: String,
}
/// # Errors
/// Returns `ValueError` when input violates an authority invariant.
pub fn plan_value_conversion_correction(
    correction_kind: &str,
    original: OriginalConversion,
) -> ValueResult<ConversionCorrection> {
    if correction_kind != "literal_reversal" {
        return Err(ValueError::new("Unknown value conversion correction kind"));
    }
    required(
        &original.source_asset,
        "Value conversion sourceAsset is required",
    )?;
    required(
        &original.destination_asset,
        "Value conversion destinationAsset is required",
    )?;
    required(
        &original.rate_snapshot_id,
        "Value conversion rateSnapshotId is required",
    )?;
    let source_amount_minor =
        non_negative_amount(&original.source_amount_minor, "sourceAmountMinor")?;
    let destination_amount_minor =
        non_negative_amount(&original.destination_amount_minor, "destinationAmountMinor")?;
    Ok(ConversionCorrection {
        correction_kind: "literal_reversal".into(),
        source_asset: original.source_asset,
        source_amount_minor: source_amount_minor.to_string(),
        destination_asset: original.destination_asset,
        destination_amount_minor: destination_amount_minor.to_string(),
        rate_snapshot_id: original.rate_snapshot_id,
    })
}
