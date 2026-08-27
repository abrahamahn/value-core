use crate::amount::parse_amount_minor;
use crate::time::{format_rfc3339_millis, parse_rfc3339_millis};
use crate::{ValueError, ValueResult, required};

const MAX_RFC3339_MILLIS: i64 = 8_640_000_000_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RationalRateResult {
    pub amount_minor: String,
}

/// Apply a positive rational rate to a non-negative amount using bankers' rounding.
///
/// # Errors
/// Returns [`ValueError`] when an amount is malformed, a rational term is not positive, or the
/// rounded result exceeds the signed 64-bit range.
pub fn multiply_rational_half_even(
    amount_minor: &str,
    numerator: &str,
    denominator: &str,
) -> ValueResult<RationalRateResult> {
    let amount = parse_amount_minor(amount_minor)?;
    let numerator = parse_amount_minor(numerator)?;
    let denominator = parse_amount_minor(denominator)?;
    if amount < 0 || numerator <= 0 || denominator <= 0 {
        return Err(ValueError::new(
            "Value rate requires a non-negative amount and positive rational terms",
        ));
    }
    let product = i128::from(amount) * i128::from(numerator);
    let denominator = i128::from(denominator);
    let quotient = product / denominator;
    let remainder = product % denominator;
    let doubled = remainder * 2;
    let rounded = if doubled > denominator || (doubled == denominator && quotient % 2 != 0) {
        quotient + 1
    } else {
        quotient
    };
    Ok(RationalRateResult {
        amount_minor: i64::try_from(rounded)
            .map_err(|_| ValueError::new("Value amount exceeds the supported signed range"))?
            .to_string(),
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueRateSnapshotInput {
    pub snapshot_id: String,
    pub base_asset: String,
    pub quote_asset: String,
    pub numerator: String,
    pub denominator: String,
    pub observed_at: String,
    pub recorded_at: String,
    pub effective_at: String,
    pub max_staleness_seconds: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueRateSnapshot {
    pub snapshot_id: String,
    pub base_asset: String,
    pub quote_asset: String,
    pub numerator: String,
    pub denominator: String,
    pub observed_at: String,
    pub recorded_at: String,
    pub effective_at: String,
    pub max_staleness_seconds: i64,
    pub expires_at: String,
}

/// Validate and canonicalize a deterministic rational rate observation.
///
/// # Errors
/// Returns [`ValueError`] when identities, rational terms, timestamps, or staleness are invalid.
pub fn create_value_rate_snapshot(input: ValueRateSnapshotInput) -> ValueResult<ValueRateSnapshot> {
    required(
        &input.snapshot_id,
        "Value rate snapshot identity and assets are required",
    )?;
    required(
        &input.base_asset,
        "Value rate snapshot identity and assets are required",
    )?;
    required(
        &input.quote_asset,
        "Value rate snapshot identity and assets are required",
    )?;
    if input.base_asset == input.quote_asset {
        return Err(ValueError::new(
            "Value rate snapshot requires distinct assets",
        ));
    }
    let numerator = parse_amount_minor(&input.numerator)?;
    let denominator = parse_amount_minor(&input.denominator)?;
    if numerator <= 0 || denominator <= 0 {
        return Err(ValueError::new(
            "Value rate numerator and denominator must be positive",
        ));
    }
    if input.max_staleness_seconds <= 0 {
        return Err(ValueError::new(
            "Value rate maximum staleness must be a positive integer",
        ));
    }
    let observed = parse_rfc3339_millis(&input.observed_at, "Value rate observedAt")?;
    let recorded = parse_rfc3339_millis(&input.recorded_at, "Value rate recordedAt")?;
    let effective = parse_rfc3339_millis(&input.effective_at, "Value rate effectiveAt")?;
    if observed > recorded || recorded > effective {
        return Err(ValueError::new(
            "Value rate observed, recorded, and effective times are out of order",
        ));
    }
    let expires = input
        .max_staleness_seconds
        .checked_mul(1_000)
        .and_then(|staleness| effective.checked_add(staleness))
        .ok_or_else(|| ValueError::new("Value rate expiry time is outside the supported range"))?;
    if !(-MAX_RFC3339_MILLIS..=MAX_RFC3339_MILLIS).contains(&expires) {
        return Err(ValueError::new(
            "Value rate expiry time is outside the supported RFC 3339 calendar range",
        ));
    }
    Ok(ValueRateSnapshot {
        snapshot_id: input.snapshot_id,
        base_asset: input.base_asset,
        quote_asset: input.quote_asset,
        numerator: numerator.to_string(),
        denominator: denominator.to_string(),
        observed_at: input.observed_at,
        recorded_at: input.recorded_at,
        effective_at: input.effective_at,
        max_staleness_seconds: input.max_staleness_seconds,
        expires_at: format_rfc3339_millis(expires),
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueRateFreshness {
    Fresh {
        retained_snapshot_id: String,
        refresh_required: bool,
    },
    Stale {
        retained_snapshot_id: String,
        refresh_required: bool,
    },
}

/// Determine whether a retained rate observation has crossed its refresh interval.
///
/// # Errors
/// Returns [`ValueError`] when the identity, interval, or timestamps are invalid.
pub fn evaluate_value_rate_freshness(
    snapshot_id: &str,
    captured_at: &str,
    evaluated_at: &str,
    refresh_interval_seconds: i64,
) -> ValueResult<ValueRateFreshness> {
    required(snapshot_id, "Value rate snapshot identity is required")?;
    if refresh_interval_seconds <= 0 {
        return Err(ValueError::new(
            "Value rate refresh interval must be a positive integer",
        ));
    }
    let captured = parse_rfc3339_millis(captured_at, "Value rate capturedAt")?;
    let evaluated = parse_rfc3339_millis(evaluated_at, "Value rate evaluatedAt")?;
    let age = evaluated
        .checked_sub(captured)
        .ok_or_else(|| ValueError::new("Value rate age is outside the supported range"))?;
    if age < 0 {
        return Err(ValueError::new(
            "Value rate evaluation cannot precede its snapshot",
        ));
    }
    let interval = refresh_interval_seconds.checked_mul(1_000).ok_or_else(|| {
        ValueError::new("Value rate refresh interval is outside the supported range")
    })?;
    Ok(if age >= interval {
        ValueRateFreshness::Stale {
            retained_snapshot_id: snapshot_id.to_owned(),
            refresh_required: true,
        }
    } else {
        ValueRateFreshness::Fresh {
            retained_snapshot_id: snapshot_id.to_owned(),
            refresh_required: false,
        }
    })
}
