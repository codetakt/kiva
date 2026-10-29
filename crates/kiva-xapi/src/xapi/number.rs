//! Exact finite JSON decimal ordering for field bounds and immutable values.

use num_bigint::BigInt;
use serde_json::{Number, Value};
use std::cmp::Ordering;

/// Tests mathematical integrality without rounding or imposing a machine-word limit.
#[must_use]
pub fn is_nonnegative_integer(number: &Number) -> bool {
    ExactNumber::from_number(number)
        .is_some_and(|value| !value.negative && value.exponent >= BigInt::from(0))
}

/// Compares an original JSON number with an octet count without floating-point conversion.
#[must_use]
pub fn attachment_length_matches(number: &Number, length: usize) -> bool {
    ExactNumber::from_number(number) == ExactNumber::from_number(&Number::from(length))
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ExactNumber {
    negative: bool,
    digits: String,
    exponent: BigInt,
}

impl ExactNumber {
    pub(super) fn from_number(number: &Number) -> Option<Self> {
        let token = number.to_string();
        let negative = token.starts_with('-');
        let unsigned = token.strip_prefix('-').unwrap_or(&token);
        let (coefficient, exponent) = unsigned.split_once(['e', 'E']).unwrap_or((unsigned, "0"));
        let mut exponent: BigInt = exponent.parse().ok()?;
        if let Some((_, fraction)) = coefficient.split_once('.') {
            exponent -= BigInt::from(fraction.len());
        }
        let digits = coefficient.replace('.', "");
        let digits = digits.trim_start_matches('0');
        if digits.is_empty() {
            return Some(Self {
                negative: false,
                digits: "0".into(),
                exponent: 0.into(),
            });
        }
        let significant = digits.trim_end_matches('0');
        exponent += BigInt::from(digits.len() - significant.len());
        Some(Self {
            negative,
            digits: significant.to_owned(),
            exponent,
        })
    }

    pub(super) fn one(negative: bool) -> Self {
        Self {
            negative,
            digits: "1".into(),
            exponent: 0.into(),
        }
    }

    fn magnitude_cmp(&self, other: &Self) -> Ordering {
        match (self.digits == "0", other.digits == "0") {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            (false, false) => {}
        }
        let order = (&self.exponent + BigInt::from(self.digits.len()))
            .cmp(&(&other.exponent + BigInt::from(other.digits.len())));
        if order != Ordering::Equal {
            return order;
        }
        // Equal decimal magnitudes need at most the supplied significant
        // digits, never an allocation proportional to a possibly huge exponent.
        self.digits
            .bytes()
            .chain(std::iter::repeat(b'0'))
            .zip(other.digits.bytes().chain(std::iter::repeat(b'0')))
            .take(self.digits.len().max(other.digits.len()))
            .find_map(|(a, b)| (a != b).then(|| a.cmp(&b)))
            .unwrap_or(Ordering::Equal)
    }
}

impl Ord for ExactNumber {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (true, true) => self.magnitude_cmp(other).reverse(),
            (false, false) => self.magnitude_cmp(other),
        }
    }
}
impl PartialOrd for ExactNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub(super) fn json_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => {
            match (ExactNumber::from_number(a), ExactNumber::from_number(b)) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| json_equal(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, a)| b.get(key).is_some_and(|b| json_equal(a, b)))
        }
        _ => left == right,
    }
}
