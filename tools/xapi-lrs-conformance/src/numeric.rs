//! Exact decimal input and the DC-NUMBER minimum binary32 preservation relation.

#![expect(
    clippy::redundant_pub_crate,
    reason = "Provider helpers are internal to the runner"
)]

use std::cmp::Ordering;

use num_bigint::BigUint;
use serde_json::Value;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Decimal {
    negative: bool,
    digits: String,
    exponent: i64,
}

impl Decimal {
    fn parse(text: &str) -> Result<Self, String> {
        let (mantissa, exponent) = text.split_once(['e', 'E']).unwrap_or((text, "0"));
        let exponent = exponent.parse::<i64>().map_err(|_| unavailable())?;
        let negative = mantissa.starts_with('-');
        let mantissa = mantissa.trim_start_matches('-');
        let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
        let digits = format!("{whole}{fraction}");
        let exponent = exponent
            .checked_sub(i64::try_from(fraction.len()).map_err(|_| unavailable())?)
            .ok_or_else(unavailable)?;
        Self::normalized(negative, &digits, exponent)
    }

    fn normalized(negative: bool, digits: &str, mut exponent: i64) -> Result<Self, String> {
        let digits = digits.trim_start_matches('0');
        if digits.is_empty() {
            return Ok(Self {
                negative: false,
                digits: "0".into(),
                exponent: 0,
            });
        }
        let significant = digits.trim_end_matches('0');
        exponent = exponent
            .checked_add(
                i64::try_from(digits.len() - significant.len()).map_err(|_| unavailable())?,
            )
            .ok_or_else(unavailable)?;
        Ok(Self {
            negative,
            digits: significant.into(),
            exponent,
        })
    }

    fn from_value(value: &Value) -> Result<Self, String> {
        let number = value.as_number().ok_or("expected a JSON number")?;
        Self::parse(&number.to_string())
    }

    fn from_binary32(value: f32) -> Result<Self, String> {
        if !value.is_finite() {
            return Err(unavailable());
        }
        let bits = value.to_bits();
        let field = (bits >> 23) & 0xff;
        let mantissa = (bits & 0x7f_ffff) | if field == 0 { 0 } else { 1 << 23 };
        let power = i32::try_from(field.max(1)).map_err(|_| unavailable())? - 150;
        let mut coefficient = BigUint::from(mantissa);
        let exponent = if power >= 0 {
            coefficient <<= usize::try_from(power).map_err(|_| unavailable())?;
            0
        } else {
            coefficient *= BigUint::from(5_u8).pow(power.unsigned_abs());
            i64::from(power)
        };
        Self::normalized(value.is_sign_negative(), &coefficient.to_string(), exponent)
    }

    fn binary32(&self) -> Option<f32> {
        format!(
            "{}{}e{}",
            if self.negative { "-" } else { "" },
            self.digits,
            self.exponent
        )
        .parse::<f32>()
        .ok()
        .filter(|v| v.is_finite())
    }

    fn magnitude(&self) -> i128 {
        i128::from(self.exponent) + i128::try_from(self.digits.len()).unwrap_or(i128::MAX)
    }

    fn compare(&self, other: &Self) -> Ordering {
        if self.digits == "0" || other.digits == "0" {
            return match (self.digits == "0", other.digits == "0") {
                (true, true) => Ordering::Equal,
                (true, false) => {
                    if other.negative {
                        Ordering::Greater
                    } else {
                        Ordering::Less
                    }
                }
                _ => {
                    if self.negative {
                        Ordering::Less
                    } else {
                        Ordering::Greater
                    }
                }
            };
        }
        if self.negative != other.negative {
            return if self.negative {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        let magnitude = self.magnitude().cmp(&other.magnitude());
        let ordering = if magnitude == Ordering::Equal {
            let size = self.digits.len().max(other.digits.len());
            self.digits
                .bytes()
                .chain(std::iter::repeat(b'0'))
                .take(size)
                .cmp(
                    other
                        .digits
                        .bytes()
                        .chain(std::iter::repeat(b'0'))
                        .take(size),
                )
        } else {
            magnitude
        };
        if self.negative {
            ordering.reverse()
        } else {
            ordering
        }
    }
}

fn unavailable() -> String {
    "ORACLE-UNAVAILABLE: DC-NUMBER decimal exponent or binary32 overflow is outside this provider's bounds".into()
}

pub(super) fn compare(left: &Value, right: &Value) -> Result<Ordering, String> {
    Ok(Decimal::from_value(left)?.compare(&Decimal::from_value(right)?))
}

pub(super) fn integral(value: &Value) -> Result<bool, String> {
    Ok(Decimal::from_value(value)?.exponent >= 0)
}

/// Exact equality, the enclosing binary32 interval, or a directed decimal
/// rounding of one endpoint that parses back to that endpoint. Merely landing
/// in the same rounding bin as the input does not suffice.
pub(super) fn preserved(expected: &Value, actual: &Value) -> Result<bool, String> {
    if !expected.is_number() || !actual.is_number() {
        return Ok(false);
    }
    let left = Decimal::from_value(expected)?;
    let right = Decimal::from_value(actual)?;
    if left == right {
        return Ok(true);
    }
    let rounded = left.binary32().ok_or_else(unavailable)?;
    let center = Decimal::from_binary32(rounded)?;
    if center == left {
        return endpoint_spelling(&center, rounded, &right);
    }
    let adjacent = if center.compare(&left).is_lt() {
        rounded.next_up()
    } else {
        rounded.next_down()
    };
    let other = Decimal::from_binary32(adjacent)?;
    let (low, high) = if center.compare(&other).is_lt() {
        (&center, &other)
    } else {
        (&other, &center)
    };
    Ok(
        (!right.compare(low).is_lt() && !right.compare(high).is_gt())
            || endpoint_spelling(&center, rounded, &right)?
            || endpoint_spelling(&other, adjacent, &right)?,
    )
}

fn endpoint_spelling(endpoint: &Decimal, binary: f32, actual: &Decimal) -> Result<bool, String> {
    if actual
        .binary32()
        .is_none_or(|v| v.to_bits() != binary.to_bits())
    {
        return Ok(false);
    }
    for precision in 1..=endpoint.digits.len() {
        let prefix = &endpoint.digits[..precision];
        let exponent = endpoint.exponent
            + i64::try_from(endpoint.digits.len() - precision).map_err(|_| unavailable())?;
        let floor = Decimal::normalized(endpoint.negative, prefix, exponent)?;
        if actual == &floor {
            return Ok(true);
        }
        if precision < endpoint.digits.len() {
            let coefficient =
                prefix.parse::<BigUint>().map_err(|_| unavailable())? + BigUint::from(1_u8);
            if actual
                == &Decimal::normalized(endpoint.negative, &coefficient.to_string(), exponent)?
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_decimal_comparison_retains_digits_and_exponents() -> Result<(), String> {
        for (left, right, equal) in [
            ("1.0", "1e0", true),
            ("-0.00", "0", true),
            ("0.10000000000000000001", "0.1", false),
            ("9007199254740993.0", "9007199254740992.0", false),
            ("1e100000", "10e99999", true),
            ("1e-100000", "0", false),
        ] {
            assert_eq!(
                Decimal::parse(left)?
                    .compare(&Decimal::parse(right)?)
                    .is_eq(),
                equal
            );
        }
        Ok(())
    }

    #[test]
    fn precision_relation_covers_decimal_subnormal_and_serialization_edges()
    -> Result<(), Box<dyn std::error::Error>> {
        for (left, right, valid) in [
            ("0.1", "0.10000000149011612", true),
            ("0.1", "0.100001", false),
            ("100000008", "1.0000001e8", true),
            ("100000008", "100000011.0", false),
            ("-100000008", "-100000010.0", true),
            ("1e-46", "0", true),
            ("1e-46", "1e-44", false),
            ("1.00000000000000000001", "1.0", true),
            ("1.0", "1.00000001", false),
        ] {
            assert_eq!(
                preserved(&serde_json::from_str(left)?, &serde_json::from_str(right)?)?,
                valid,
                "{left} -> {right}"
            );
        }
        assert!(
            preserved(
                &serde_json::from_str("1e40")?,
                &serde_json::from_str("1.01e40")?
            )
            .is_err()
        );
        Ok(())
    }
}
