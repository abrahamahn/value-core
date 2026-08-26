use crate::{ValueError, ValueResult};

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let adjusted_year = year - i64::from(month <= 2);
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year - era * 400;
    let shifted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

pub(crate) fn parse_rfc3339_millis(value: &str, context: &str) -> ValueResult<i64> {
    let bytes = value.as_bytes();
    if bytes.len() < 20
        || bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || bytes.get(10) != Some(&b'T')
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
    {
        return Err(ValueError::new(format!(
            "{context} must be an RFC 3339 instant"
        )));
    }
    let number = |start: usize, end: usize| -> Option<i64> {
        std::str::from_utf8(bytes.get(start..end)?)
            .ok()?
            .parse()
            .ok()
    };
    let year = number(0, 4);
    let month = number(5, 7);
    let day = number(8, 10);
    let hour = number(11, 13);
    let minute = number(14, 16);
    let second = number(17, 19);
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) =
        (year, month, day, hour, minute, second)
    else {
        return Err(ValueError::new(format!(
            "{context} must be an RFC 3339 instant"
        )));
    };
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return Err(ValueError::new(format!(
            "{context} must be an RFC 3339 instant"
        )));
    }
    let mut cursor = 19;
    let mut millis = 0_i64;
    if bytes.get(cursor) == Some(&b'.') {
        cursor += 1;
        let start = cursor;
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
        if cursor == start {
            return Err(ValueError::new(format!(
                "{context} must be an RFC 3339 instant"
            )));
        }
        let fraction = std::str::from_utf8(&bytes[start..cursor]).unwrap_or_default();
        let three = format!("{fraction:0<3}");
        millis = three[..3].parse().unwrap_or(0);
    }
    let offset_seconds = match bytes.get(cursor) {
        Some(b'Z') if cursor + 1 == bytes.len() => 0,
        Some(sign @ (b'+' | b'-'))
            if cursor + 6 == bytes.len() && bytes.get(cursor + 3) == Some(&b':') =>
        {
            let offset_hour = number(cursor + 1, cursor + 3).unwrap_or(99);
            let offset_minute = number(cursor + 4, cursor + 6).unwrap_or(99);
            if offset_hour > 23 || offset_minute > 59 {
                return Err(ValueError::new(format!(
                    "{context} must be an RFC 3339 instant"
                )));
            }
            let offset = (offset_hour * 60 + offset_minute) * 60;
            if *sign == b'+' { offset } else { -offset }
        }
        _ => {
            return Err(ValueError::new(format!(
                "{context} must be an RFC 3339 instant"
            )));
        }
    };
    let seconds = days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second
        - offset_seconds;
    seconds
        .checked_mul(1_000)
        .and_then(|value| value.checked_add(millis))
        .ok_or_else(|| ValueError::new(format!("{context} is outside the supported range")))
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = shifted_month + if shifted_month < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

pub(crate) fn format_rfc3339_millis(millis: i64) -> String {
    let seconds = millis.div_euclid(1_000);
    let fraction = millis.rem_euclid(1_000);
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{fraction:03}Z")
}
