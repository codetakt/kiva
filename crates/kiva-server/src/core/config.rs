//! Server-wide configuration constants.

use std::num::NonZeroU64;

pub(crate) const DATABASE_URL_ENV: &str = "DATABASE_URL";
pub(crate) const KIVA_TENANT_ID_ENV: &str = "KIVA_TENANT_ID";
pub(crate) const KIVA_MAX_STATEMENT_BODY_BYTES_ENV: &str = "KIVA_MAX_STATEMENT_BODY_BYTES";
pub(crate) const KIVA_RATE_LIMIT_PATH_ENV: &str = "KIVA_RATE_LIMIT_PATH";
pub(crate) const KIVA_RATE_LIMIT_REQUEST_COUNT_ENV: &str = "KIVA_RATE_LIMIT_REQUEST_COUNT";
pub(crate) const KIVA_RATE_LIMIT_WINDOW_SECONDS_ENV: &str = "KIVA_RATE_LIMIT_WINDOW_SECONDS";
pub(crate) const KIVA_REQUIRE_AUTHORIZATION_ENV: &str = "KIVA_REQUIRE_AUTHORIZATION";
pub(crate) const KIVA_ADL_COMPAT_BASIC_AUTH_ENV: &str = "KIVA_ADL_COMPAT_BASIC_AUTH";
pub(crate) const KIVA_IDP_ISSUER_ENV: &str = "KIVA_IDP_ISSUER";
pub(crate) const KIVA_IDP_AUDIENCE_ENV: &str = "KIVA_IDP_AUDIENCE";
pub(crate) const KIVA_IDP_JWKS_URL_ENV: &str = "KIVA_IDP_JWKS_URL";
pub(crate) const KIVA_IDP_JWKS_FILE_ENV: &str = "KIVA_IDP_JWKS_FILE";
pub(crate) const KIVA_IDP_INTROSPECTION_URL_ENV: &str = "KIVA_IDP_INTROSPECTION_URL";
pub(crate) const KIVA_IDP_CLIENT_ID_ENV: &str = "KIVA_IDP_CLIENT_ID";
pub(crate) const KIVA_IDP_CLIENT_SECRET_ENV: &str = "KIVA_IDP_CLIENT_SECRET";
pub(crate) const KIVA_IDP_TENANT_CLAIM_ENV: &str = "KIVA_IDP_TENANT_CLAIM";
pub(crate) const KIVA_IDP_IGNORED_SCOPES_ENV: &str = "KIVA_IDP_IGNORED_SCOPES";
pub(crate) const KIVA_CONFORMANCE_WRITER_A_TOKEN_ENV: &str = "KIVA_CONFORMANCE_WRITER_A_TOKEN";
pub(crate) const KIVA_CONFORMANCE_READER_TOKEN_ENV: &str = "KIVA_CONFORMANCE_READER_TOKEN";
pub(crate) const KIVA_CONFORMANCE_INVALID_TOKEN_ENV: &str = "KIVA_CONFORMANCE_INVALID_TOKEN";
pub(crate) const DEFAULT_TENANT_ID: &str = "default";
pub(crate) const DOCUMENT_JSON_CONTENT_TYPE: &str = "application/json";
pub(crate) const MULTIPART_MIXED_CONTENT_TYPE: &str = "multipart/mixed";
pub(crate) const XAPI_VERSION: &str = kiva_http_core::XAPI_VERSION;
pub(crate) const XAPI_VERSION_HEADER: &str = "X-Experience-API-Version";
pub(crate) const XAPI_HASH_HEADER: &str = "X-Experience-API-Hash";
pub(crate) const SIGNATURE_ATTACHMENT_USAGE_TYPE: &str =
    "http://adlnet.gov/expapi/attachments/signature";
pub(crate) const CONFIDENTIAL_DIFFERENTIAL_PRIVACY_ADVANCED_DELTA_MICROUNITS: NonZeroU64 =
    NonZeroU64::MIN;
pub(crate) const CONFIDENTIAL_SHAMIR_VERIFICATION_PROFILE: &str = "kiva-shamir-hash-vss-v1";
pub(crate) const CONFIDENTIAL_JOB_MAX_ACTIVE_PER_QUOTA_KEY: usize = 64;
pub(crate) const CONFIDENTIAL_JOB_MAX_RUNNING_PER_TENANT: usize = 16;
pub(crate) const PDS_SUBJECT_ACCOUNT_HOME_PAGE: &str = "https://kiva.example/pds-subjects";
