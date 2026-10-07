//! HTTP path constants for the Kiva server.

pub(crate) const CONFORMANCE_RATE_LIMIT_PATH: &str = "/__kiva/conformance/rate-limit";

pub(crate) const ANALYTICS_PRODUCTS_PATH: &str = "/analytics/products";
pub(crate) const ANALYTICS_PRODUCT_PATH: &str = "/analytics/products/{dataProductId}";
pub(crate) const ANALYTICS_PRODUCT_AGGREGATE_PATH: &str =
    "/analytics/products/{dataProductId}/aggregate";

pub(crate) const CONFIDENTIAL_ADDITIVE_AGGREGATES_PATH: &str = "/confidential/additive-aggregates";
pub(crate) const CONFIDENTIAL_ADDITIVE_SHARE_SUBMISSIONS_PATH: &str =
    "/confidential/additive-share-submissions";
pub(crate) const CONFIDENTIAL_ADDITIVE_SHARE_REPLAY_RECORDS_PATH: &str =
    "/confidential/additive-share-replay-records";
pub(crate) const CONFIDENTIAL_SHARE_SIGNING_KEYS_PATH: &str =
    "/confidential/additive-share-signing-keys";
pub(crate) const CONFIDENTIAL_SHARE_SIGNING_KEY_ROTATIONS_PATH: &str =
    "/confidential/additive-share-signing-key-rotations";
pub(crate) const CONFIDENTIAL_SHARE_SIGNING_KEY_REVOCATIONS_PATH: &str =
    "/confidential/additive-share-signing-key-revocations";
pub(crate) const CONFIDENTIAL_THRESHOLD_RELEASE_REQUESTS_PATH: &str =
    "/confidential/threshold-release-requests";
pub(crate) const CONFIDENTIAL_THRESHOLD_RELEASE_APPROVALS_PATH: &str =
    "/confidential/threshold-release-approvals";
pub(crate) const CONFIDENTIAL_THRESHOLD_RELEASES_PATH: &str = "/confidential/threshold-releases";
pub(crate) const CONFIDENTIAL_SHAMIR_THRESHOLD_RECONSTRUCTIONS_PATH: &str =
    "/confidential/shamir-threshold-reconstructions";
pub(crate) const CONFIDENTIAL_DIFFERENTIAL_PRIVACY_BUDGETS_PATH: &str =
    "/confidential/differential-privacy-budgets";
pub(crate) const CONFIDENTIAL_DIFFERENTIAL_PRIVACY_BUDGET_SPENDS_PATH: &str =
    "/confidential/differential-privacy-budget-spends";
pub(crate) const CONFIDENTIAL_DIFFERENTIAL_PRIVACY_COMPOSITION_SUMMARIES_PATH: &str =
    "/confidential/differential-privacy-composition-summaries";
pub(crate) const CONFIDENTIAL_DIFFERENTIAL_PRIVACY_RELEASES_PATH: &str =
    "/confidential/differential-privacy-releases";
pub(crate) const CONFIDENTIAL_DIFFERENTIAL_PRIVACY_SAMPLED_RELEASES_PATH: &str =
    "/confidential/differential-privacy-sampled-releases";
pub(crate) const CONFIDENTIAL_JOBS_PATH: &str = "/confidential/jobs";
pub(crate) const CONFIDENTIAL_JOB_TRANSITIONS_PATH: &str = "/confidential/job-transitions";
pub(crate) const CONFIDENTIAL_JOB_AUDIT_EVENTS_PATH: &str = "/confidential/job-audit-events";
pub(crate) const CONFIDENTIAL_PARTICIPANTS_PATH: &str = "/confidential/participants";
pub(crate) const CONFIDENTIAL_PARTICIPANT_TRANSITIONS_PATH: &str =
    "/confidential/participant-transitions";
pub(crate) const CONFIDENTIAL_PARTICIPANT_AUDIT_EVENTS_PATH: &str =
    "/confidential/participant-audit-events";

pub(crate) const PDS_CONSENT_GRANTS_PATH: &str = "/pds/consent-grants";
pub(crate) const PDS_CONSENT_REVOCATIONS_PATH: &str = "/pds/consent-revocations";
pub(crate) const PDS_DATA_PRODUCTS_PATH: &str = "/pds/data-products";
pub(crate) const PDS_DATA_PRODUCT_READS_PATH: &str = "/pds/data-product-reads";
pub(crate) const PDS_PROVIDER_BINDINGS_PATH: &str = "/pds/provider-bindings";
pub(crate) const PDS_RECORD_INVENTORY_PATH: &str = "/pds/record-inventory";
pub(crate) const PDS_METADATA_EXPORTS_PATH: &str = "/pds/metadata-exports";
pub(crate) const PDS_DATA_PORTABILITY_EXPORTS_PATH: &str = "/pds/data-portability-exports";
pub(crate) const PDS_ACCESS_CHECKS_PATH: &str = "/pds/access-checks";
pub(crate) const PDS_ACCESS_AUDIT_EVENTS_PATH: &str = "/pds/access-audit-events";
pub(crate) const PDS_LIFECYCLE_EVENTS_PATH: &str = "/pds/lifecycle-events";
pub(crate) const PDS_RETENTION_POLICIES_PATH: &str = "/pds/retention-policies";
pub(crate) const PDS_LIFECYCLE_CASES_PATH: &str = "/pds/lifecycle-cases";
pub(crate) const PDS_GOVERNANCE_PROFILES_PATH: &str = "/pds/governance-profiles";
