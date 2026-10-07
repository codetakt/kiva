//! Projection from analytics application records to the `OpenAPI` wire contract.

use kiva_analytics_openapi_contract::{
    AnalyticsAggregate as OpenApiAggregate, AnalyticsProduct as OpenApiProduct,
    AnalyticsProductPage as OpenApiProductPage, AnalyticsReleaseKind,
};
use kiva_application::analytics::{AnalyticsAggregate, AnalyticsProduct};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

fn rfc3339(value: OffsetDateTime) -> Option<String> {
    value.format(&Rfc3339).ok()
}

fn release_kind(label: &str) -> Option<AnalyticsReleaseKind> {
    match label {
        "threshold-release" => Some(AnalyticsReleaseKind::ThresholdRelease),
        "differential-privacy-release" => Some(AnalyticsReleaseKind::DifferentialPrivacyRelease),
        _other => None,
    }
}

pub(crate) fn product(product: AnalyticsProduct) -> Option<OpenApiProduct> {
    Some(OpenApiProduct {
        data_product_id: product.data_product_id,
        title: product.title,
        purpose: product.purpose,
        release_kind: release_kind(&product.release_kind)?,
        approved_at: rfc3339(product.approved_at)?,
    })
}

pub(crate) fn product_page(
    products: Vec<AnalyticsProduct>,
    next_cursor: Option<String>,
) -> Option<OpenApiProductPage> {
    let products = products
        .into_iter()
        .map(product)
        .collect::<Option<Vec<_>>>()?;
    Some(OpenApiProductPage {
        products,
        next_cursor,
    })
}

pub(crate) fn aggregate(aggregate: &AnalyticsAggregate) -> Option<OpenApiAggregate> {
    Some(OpenApiAggregate {
        data_product_id: aggregate.data_product_id,
        release_kind: release_kind(&aggregate.release_kind)?,
        value: aggregate.value,
        released_at: rfc3339(aggregate.released_at)?,
    })
}
