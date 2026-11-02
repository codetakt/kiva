//! Optional definition validation, without grading responses or interpreting parameters.

use crate::core::response::HttpError;
use axum::http::StatusCode;
use num_bigint::BigInt;
use serde_json::Value;
use std::{cmp::Ordering, collections::BTreeSet};

fn invalid(reason: &str) -> HttpError {
    HttpError::new(
        StatusCode::BAD_REQUEST,
        format!("invalid interaction definition: {reason}"),
    )
}

pub(crate) fn validate(statement: &Value) -> Result<(), HttpError> {
    if let Some(batch) = statement.as_array() {
        for item in batch {
            validate(item)?;
        }
        return Ok(());
    }
    let object = &statement["object"];
    match object["objectType"].as_str().unwrap_or("Activity") {
        "Activity" => definition(&object["definition"])?,
        "SubStatement" => validate(object)?,
        _ => {}
    }
    for role in ["parent", "grouping", "category", "other"] {
        if let Some(activities) = statement.pointer(&format!("/context/contextActivities/{role}")) {
            for activity in activities
                .as_array()
                .map_or_else(|| vec![activities], |a| a.iter().collect())
            {
                definition(&activity["definition"])?;
            }
        }
    }
    Ok(())
}

pub(crate) fn definition(value: &Value) -> Result<(), HttpError> {
    let Some(kind) = value["interactionType"].as_str() else {
        return Ok(());
    };
    let components = match kind {
        "choice" | "sequencing" => &["choices"][..],
        "likert" => &["scale"],
        "matching" => &["source", "target"],
        "performance" => &["steps"],
        _ => &[],
    };
    for field in ["choices", "scale", "source", "target", "steps"] {
        if let Some(list) = value[field].as_array() {
            if !components.contains(&field) {
                return Err(invalid(
                    "component list does not apply to this interaction type",
                ));
            }
            let mut ids = BTreeSet::new();
            for component in list {
                if let Some(id) = component["id"].as_str()
                    && !ids.insert(id)
                {
                    return Err(invalid(
                        "component identifiers must be distinct within a list",
                    ));
                }
            }
        }
    }
    for pattern in value["correctResponsesPattern"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        let present = |field: &str, id: &str| {
            value[field].as_array().is_none_or(|list| {
                list.iter()
                    .any(|component| component["id"].as_str() == Some(id))
            })
        };
        let valid = match kind {
            "true-false" => matches!(pattern, "true" | "false"),
            "choice" | "sequencing" => pattern.split("[,]").all(|id| present("choices", id)),
            "likert" => !pattern.contains("[,]") && present("scale", pattern),
            "matching" => pattern.split("[,]").all(|pair| {
                let parts = pair.split("[.]").collect::<Vec<_>>();
                parts.len() == 2 && present("source", parts[0]) && present("target", parts[1])
            }),
            "performance" => without_parameters(pattern).is_none_or(|pattern| {
                pattern.split("[,]").all(|pair| {
                    pair.split_once("[.]")
                        .is_some_and(|(id, _)| present("steps", id))
                })
            }),
            "numeric" => numeric_range(pattern),
            // Free strings may be empty, contain whitespace and have arbitrary
            // parameter values. Their content is not an LRS rejection criterion.
            _ => true, // Structural validation owns the controlled vocabulary.
        };
        if !valid {
            return Err(invalid(
                "correct response pattern does not match its interaction type or component identifiers",
            ));
        }
    }
    Ok(())
}

fn without_parameters(mut value: &str) -> Option<&str> {
    while value.starts_with('{') {
        let (_, rest) = value.split_once('}')?;
        value = rest;
    }
    Some(value)
}

struct Decimal {
    negative: bool,
    digits: String,
    order: BigInt,
}

impl Decimal {
    fn parse(value: &str) -> Option<Self> {
        let (negative, value) = value.strip_prefix('-').map_or_else(
            || (false, value.strip_prefix('+').unwrap_or(value)),
            |v| (true, v),
        );
        let mut split = value.split(['e', 'E']);
        let significand = split.next()?;
        let exponent = match split.next() {
            Some(exp) => {
                let digits = exp.strip_prefix(['+', '-']).unwrap_or(exp);
                if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                    return None;
                }
                exp.parse::<BigInt>().ok()?
            }
            None => BigInt::from(0),
        };
        if split.next().is_some() {
            return None;
        }
        let (integer, fraction) = significand.split_once('.').unwrap_or((significand, ""));
        if integer.is_empty() && fraction.is_empty()
            || !integer
                .bytes()
                .chain(fraction.bytes())
                .all(|b| b.is_ascii_digit())
        {
            return None;
        }
        let digits = format!("{integer}{fraction}")
            .trim_start_matches('0')
            .to_owned();
        let order = exponent + BigInt::from(digits.len()) - BigInt::from(fraction.len());
        Some(Self {
            negative: negative && !digits.is_empty(),
            digits,
            order,
        })
    }

    fn compare(&self, other: &Self) -> Ordering {
        if self.negative != other.negative {
            return if self.negative {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        let absolute = if self.digits.is_empty() || other.digits.is_empty() {
            self.digits
                .is_empty()
                .cmp(&other.digits.is_empty())
                .reverse()
        } else {
            self.order.cmp(&other.order).then_with(|| {
                let n = self.digits.len().max(other.digits.len());
                self.digits
                    .bytes()
                    .chain(std::iter::repeat(b'0'))
                    .take(n)
                    .cmp(other.digits.bytes().chain(std::iter::repeat(b'0')).take(n))
            })
        };
        if self.negative {
            absolute.reverse()
        } else {
            absolute
        }
    }
}

fn numeric_range(value: &str) -> bool {
    let pieces = value.split("[:]").collect::<Vec<_>>();
    if pieces.len() == 1 {
        return Decimal::parse(value).is_some();
    }
    if pieces.len() != 2 || pieces.iter().all(|v| v.is_empty()) {
        return false;
    }
    let low = if pieces[0].is_empty() {
        None
    } else {
        let Some(n) = Decimal::parse(pieces[0]) else {
            return false;
        };
        Some(n)
    };
    let high = if pieces[1].is_empty() {
        None
    } else {
        let Some(n) = Decimal::parse(pieces[1]) else {
            return false;
        };
        Some(n)
    };
    match (low, high) {
        (Some(low), Some(high)) => low.compare(&high) != Ordering::Greater,
        _ => true,
    }
}
