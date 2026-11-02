//! UTC projection without losing an accepted leap second or decimal fraction.

use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

/// Keep the runtime's calendar, offset and leap-position validation. Its time
/// value substitutes `:60` with `:59.999999999`, so use it only for the UTC
/// calendar/minute and retain the original second marker and fraction ourselves.
/// Both conversion and formatting must succeed: a valid local RFC3339 year
/// can cross the runtime or the four-digit UTC wire range after offset removal.
pub(crate) fn normalize(value: &str) -> Option<String> {
    let parsed = OffsetDateTime::parse(value, &Rfc3339).ok()?;
    let leap = value.get(17..19) == Some("60");
    let fraction = value.get(19..)?.strip_prefix('.').map_or("", |suffix| {
        let end = suffix
            .bytes()
            .position(|byte| !byte.is_ascii_digit())
            .unwrap_or(suffix.len());
        suffix[..end].trim_end_matches('0')
    });
    let utc = parsed
        .checked_to_offset(UtcOffset::UTC)?
        .replace_nanosecond(0)
        .ok()?
        .format(&Rfc3339)
        .ok()?;
    let mut result = utc.strip_suffix('Z')?.to_owned();
    if leap {
        // RFC3339 offsets have minute precision, so offset conversion does
        // not move the leap marker out of the final second of its UTC minute.
        result.truncate(result.len().checked_sub(2)?);
        result.push_str("60");
    }
    if !fraction.is_empty() {
        result.push('.');
        result.push_str(fraction);
    }
    result.push('Z');
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn handles_both_utc_wire_year_boundaries_without_panicking() {
        for value in [
            "9999-12-31T23:30:00-01:00",
            "9999-12-31T23:59:59.999999999999-00:01",
            "0000-01-01T00:30:00+01:00",
            "0000-01-01T00:00:00.000000000001+00:01",
        ] {
            assert_eq!(normalize(value), None, "{value}");
        }
        for (input, expected) in [
            ("9999-12-31T22:30:00-01:00", "9999-12-31T23:30:00Z"),
            ("9999-12-31T23:30:00+01:00", "9999-12-31T22:30:00Z"),
            ("0000-01-01T01:30:00+01:00", "0000-01-01T00:30:00Z"),
            ("0000-01-01T00:30:00-01:00", "0000-01-01T01:30:00Z"),
        ] {
            assert_eq!(normalize(input).as_deref(), Some(expected), "{input}");
        }
    }

    #[test]
    fn preserves_leap_seconds_and_full_decimal_fractions_in_utc() {
        for (input, expected) in [
            ("2016-12-31T23:59:60Z", "2016-12-31T23:59:60Z"),
            ("2016-12-31T23:59:60.250Z", "2016-12-31T23:59:60.25Z"),
            ("2016-12-31T23:59:60.750Z", "2016-12-31T23:59:60.75Z"),
            ("2017-01-01T08:59:60.250+09:00", "2016-12-31T23:59:60.25Z"),
            ("2016-12-31T18:29:60.750-05:30", "2016-12-31T23:59:60.75Z"),
            ("2015-06-30t23:59:60.000z", "2015-06-30T23:59:60Z"),
            (
                "2016-12-31T23:59:60.123456789012Z",
                "2016-12-31T23:59:60.123456789012Z",
            ),
            (
                "1970-01-01T00:30:00.000000000001+01:00",
                "1969-12-31T23:30:00.000000000001Z",
            ),
            (
                "2016-12-31T23:59:59.999999999Z",
                "2016-12-31T23:59:59.999999999Z",
            ),
        ] {
            assert_eq!(normalize(input).as_deref(), Some(expected), "{input}");
        }
        for input in ["2016-12-31", "2016-12-31T12:00:60Z", "2016-12-31T23:59:61Z"] {
            assert!(normalize(input).is_none(), "{input}");
        }
    }
}
