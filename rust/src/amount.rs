use crate::{ValueError, ValueResult};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArithmeticOperation {
    Add {
        left: String,
        right: String,
    },
    RoundRationalFloor {
        numerator: String,
        denominator: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArithmeticResult {
    pub amount_minor: String,
    pub remainder_numerator: Option<String>,
}
/// # Errors
/// Returns `ValueError` when input violates an authority invariant.
pub fn parse_amount_minor(value: &str) -> ValueResult<i64> {
    if value.is_empty()
        || value == "-"
        || value == "-0"
        || value.starts_with('+')
        || (value.starts_with('0') && value.len() > 1)
        || (value.starts_with("-0") && value.len() > 2)
        || !value
            .bytes()
            .enumerate()
            .all(|(index, byte)| byte.is_ascii_digit() || (index == 0 && byte == b'-'))
    {
        return Err(ValueError::new(
            "Value amount must be a canonical decimal integer",
        ));
    }
    value
        .parse::<i64>()
        .map_err(|_| ValueError::new("Value amount exceeds the supported signed range"))
}

fn floor_divide(numerator: i128, denominator: i128) -> ValueResult<(i128, i128)> {
    if denominator <= 0 {
        return Err(ValueError::new("Rational denominator must be positive"));
    }
    let mut quotient = numerator / denominator;
    let mut remainder = numerator % denominator;
    if remainder < 0 {
        quotient -= 1;
        remainder += denominator;
    }
    Ok((quotient, remainder))
}

fn checked_i64(value: i128) -> ValueResult<i64> {
    i64::try_from(value).map_err(|_| ValueError::new("Value arithmetic overflow"))
}
/// # Errors
/// Returns `ValueError` when input violates an authority invariant.
pub fn evaluate_value_arithmetic(input: ArithmeticOperation) -> ValueResult<ArithmeticResult> {
    match input {
        ArithmeticOperation::Add { left, right } => Ok(ArithmeticResult {
            amount_minor: parse_amount_minor(&left)?
                .checked_add(parse_amount_minor(&right)?)
                .ok_or_else(|| ValueError::new("Value arithmetic overflow"))?
                .to_string(),
            remainder_numerator: None,
        }),
        ArithmeticOperation::RoundRationalFloor {
            numerator,
            denominator,
        } => {
            let (quotient, remainder) = floor_divide(
                i128::from(parse_amount_minor(&numerator)?),
                i128::from(parse_amount_minor(&denominator)?),
            )?;
            Ok(ArithmeticResult {
                amount_minor: checked_i64(quotient)?.to_string(),
                remainder_numerator: Some(checked_i64(remainder)?.to_string()),
            })
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RationalFloorResult {
    pub amount_minor: String,
    pub remainder_numerator: String,
}

/// # Errors
/// Returns [`ValueError`] when the denominator is zero or arithmetic overflows.
pub fn multiply_rational_floor(
    amount_minor: &str,
    numerator: &str,
    denominator: &str,
) -> ValueResult<RationalFloorResult> {
    let product =
        i128::from(parse_amount_minor(amount_minor)?) * i128::from(parse_amount_minor(numerator)?);
    let (quotient, remainder) =
        floor_divide(product, i128::from(parse_amount_minor(denominator)?))?;
    Ok(RationalFloorResult {
        amount_minor: checked_i64(quotient)?.to_string(),
        remainder_numerator: checked_i64(remainder)?.to_string(),
    })
}
