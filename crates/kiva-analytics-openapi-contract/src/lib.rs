//! Governed analytics `OpenAPI` response contract types.

#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsProduct {
    pub data_product_id: Uuid,
    pub title: String,
    pub purpose: String,
    pub release_kind: AnalyticsReleaseKind,
    pub approved_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsProductPage {
    pub products: Vec<AnalyticsProduct>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsAggregate {
    pub data_product_id: Uuid,
    pub release_kind: AnalyticsReleaseKind,
    pub value: i64,
    pub released_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum AnalyticsReleaseKind {
    #[serde(rename = "threshold-release")]
    ThresholdRelease,
    #[serde(rename = "differential-privacy-release")]
    DifferentialPrivacyRelease,
}
