use std::fmt::Write;

use serde_json::Value;

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

#[allow(clippy::many_single_char_names)]
#[allow(clippy::too_many_lines)]
pub(crate) fn sha256(bytes: &[u8]) -> [u8; 32] {
    const H0: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    const K: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];
    let bit_len = (bytes.len() as u64) * 8;
    let mut message = bytes.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());
    let mut hash = H0;
    for chunk in message.chunks_exact(64) {
        let mut w = [0_u32; 64];
        for (index, word) in chunk.chunks_exact(4).enumerate() {
            w[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for index in 16..64 {
            let s0 = w[index - 15].rotate_right(7)
                ^ w[index - 15].rotate_right(18)
                ^ (w[index - 15] >> 3);
            let s1 = w[index - 2].rotate_right(17)
                ^ w[index - 2].rotate_right(19)
                ^ (w[index - 2] >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = hash;
        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[index])
                .wrapping_add(w[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        for (state, value) in hash.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *state = state.wrapping_add(value);
        }
    }
    let mut output = [0_u8; 32];
    for (index, word) in hash.iter().enumerate() {
        output[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    output
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
    for byte in sha256(payload.as_bytes()) {
        write!(output, "{byte:02x}").map_err(|error| ValueError::new(error.to_string()))?;
    }
    Ok(output)
}
