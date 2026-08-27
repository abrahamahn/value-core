use std::fmt::Write;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{ValueError, ValueResult, required};

fn javascript_number(number: &serde_json::Number) -> ValueResult<String> {
    let value = number
        .as_f64()
        .ok_or_else(|| ValueError::new("Canonical JSON numbers must be finite"))?;
    if !value.is_finite() || (value == 0.0 && value.is_sign_negative()) {
        return Err(ValueError::new(
            "Canonical JSON numbers must be finite and cannot be negative zero",
        ));
    }
    if value == 0.0 {
        return Ok("0".into());
    }
    // JavaScript has one numeric type. Even when serde_json retained an exact
    // integer token, canonicalization must first observe its IEEE-754 value.
    let mut rendered = serde_json::Number::from_f64(value)
        .ok_or_else(|| ValueError::new("Canonical JSON numbers must be finite"))?
        .to_string();
    if let Some((mantissa, exponent)) = rendered
        .split_once('e')
        .or_else(|| rendered.split_once('E'))
    {
        let exponent: i32 = exponent.parse().map_err(|_| {
            ValueError::new("Canonical JSON number exponent is outside the supported range")
        })?;
        let absolute = value.abs();
        if (1e-6..1e21).contains(&absolute) {
            let negative = mantissa.starts_with('-');
            let unsigned = mantissa.trim_start_matches('-');
            let decimal_index = unsigned.find('.').unwrap_or(unsigned.len());
            let digits = unsigned.replace('.', "");
            let target = i32::try_from(decimal_index).unwrap_or(i32::MAX) + exponent;
            let fixed = if target <= 0 {
                format!(
                    "0.{}{}",
                    "0".repeat(usize::try_from(-target).expect("target is non-positive")),
                    digits
                )
            } else if usize::try_from(target).unwrap_or(usize::MAX) >= digits.len() {
                format!(
                    "{}{}",
                    digits,
                    "0".repeat(usize::try_from(target).unwrap_or(digits.len()) - digits.len())
                )
            } else {
                let target = usize::try_from(target).expect("target is positive");
                format!("{}.{}", &digits[..target], &digits[target..])
            };
            return Ok(if negative { format!("-{fixed}") } else { fixed });
        }
        let exponent_sign = if exponent >= 0 { "+" } else { "" };
        return Ok(format!("{mantissa}e{exponent_sign}{exponent}"));
    }
    if rendered.ends_with(".0") {
        rendered.truncate(rendered.len() - 2);
    }
    Ok(rendered)
}

fn write_canonical(value: &Value, output: &mut String) -> ValueResult<()> {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        Value::Number(number) => output.push_str(&javascript_number(number)?),
        Value::String(value) => output.push_str(
            &serde_json::to_string(value).map_err(|error| ValueError::new(error.to_string()))?,
        ),
        Value::Array(items) => {
            output.push('[');
            for (index, item) in items.iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                write_canonical(item, output)?;
            }
            output.push(']');
        }
        Value::Object(object) => {
            let mut keys: Vec<&String> = object.keys().collect();
            keys.sort_by(|left, right| left.encode_utf16().cmp(right.encode_utf16()));
            output.push('{');
            for (index, key) in keys.into_iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                output.push_str(
                    &serde_json::to_string(key)
                        .map_err(|error| ValueError::new(error.to_string()))?,
                );
                output.push(':');
                write_canonical(
                    object
                        .get(key)
                        .ok_or_else(|| ValueError::new("Canonical JSON object key is missing"))?,
                    output,
                )?;
            }
            output.push('}');
        }
    }
    Ok(())
}

/// # Errors
/// Returns `ValueError` when input violates an authority invariant.
pub fn canonical_json(value: &Value) -> ValueResult<String> {
    let mut output = String::new();
    write_canonical(value, &mut output)?;
    Ok(output)
}

/// # Errors
/// Returns `ValueError` when input violates an authority invariant.
pub fn domain_separated_digest(
    domain: &str,
    contract_version: &str,
    value: &Value,
) -> ValueResult<String> {
    required(domain, "Canonical digest domain is required")?;
    required(
        contract_version,
        "Canonical digest contract version is required",
    )?;
    let payload = format!("{domain}\0{contract_version}\0{}", canonical_json(value)?);
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(payload.as_bytes()) {
        write!(output, "{byte:02x}").map_err(|error| ValueError::new(error.to_string()))?;
    }
    Ok(output)
}
