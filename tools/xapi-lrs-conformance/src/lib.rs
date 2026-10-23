//! Black-box `xAPI` LRS conformance aid runner.
//!
//! The runner consumes clause-indexed YAML test cases, validates their
//! traceability metadata, and can execute the cases against an HTTP endpoint.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fmt::{self, Display};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use iri_string::types::{IriStr, UriStr};
use reqwest::{
    Client, Method, StatusCode,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};
use url::Url;

mod assertions;
mod media_type;
mod preservation;
use assertions::Check;

const DEFAULT_CASE_ROOT: &str = "tests/conformance/http";
const REPORT_SCHEMA_VERSION: u16 = 2;
const ENDPOINT_PROFILE_SCHEMA_VERSION: u16 = 1;
const STANDARD: &str = "IEEE/ISO/IEC 39274-1-1-2025";
const RUNTIME_CAPTURE_NOW_PLUS_30S: &str = "kiva_now_plus_30s";
const ENDPOINT_PROFILE_TOO_LARGE: &str = "deterministic-too-large-threshold";
const ENDPOINT_PROFILE_RATE_LIMIT: &str = "deterministic-rate-limit-trigger";
const ENDPOINT_CAPTURE_TOO_LARGE_BODY: &str = "kiva_endpoint_profile_too_large_statement_body";
const ENDPOINT_CAPTURE_RATE_LIMIT_REQUEST_COUNT: &str =
    "kiva_endpoint_profile_rate_limit_request_count";
const ENDPOINT_CAPTURE_RATE_LIMIT_WINDOW_SECONDS: &str =
    "kiva_endpoint_profile_rate_limit_window_seconds";
const ENDPOINT_CAPTURE_RATE_LIMIT_PATH: &str = "kiva_endpoint_profile_rate_limit_path";
const MAX_ENDPOINT_PROFILE_PROBE_BODY_BYTES: usize = 16 * 1024 * 1024;
const MAX_ENDPOINT_PROFILE_RATE_LIMIT_REQUESTS: usize = 1_000;
const MAX_ENDPOINT_PROFILE_RATE_LIMIT_WINDOW_SECONDS: u64 = 3_600;

type Captures = BTreeMap<String, String>;

/// Check an absolute URI (ASCII) or IRI against RFC 3986/3987 syntax.
///
/// This performs no resolution, normalization or scheme-specific policy check.
#[must_use]
pub fn identifier_has_valid_syntax(value: &str, ascii_only: bool) -> bool {
    if ascii_only {
        UriStr::new(value).is_ok()
    } else {
        IriStr::new(value).is_ok()
    }
}

/// Errors produced by the conformance runner.
#[derive(Debug, Error)]
pub enum Error {
    /// A case file or directory could not be read.
    #[error("failed to read {path}: {source}")]
    Read {
        /// Path that could not be read.
        path: PathBuf,
        /// I/O error reported by the operating system.
        source: io::Error,
    },

    /// A generated report could not be written.
    #[error("failed to write {path}: {source}")]
    Write {
        /// Path that could not be written.
        path: PathBuf,
        /// I/O error reported by the operating system.
        source: io::Error,
    },

    /// A YAML case file could not be parsed.
    #[error("failed to parse YAML {path}: {source}")]
    ParseYaml {
        /// Path that could not be parsed.
        path: PathBuf,
        /// YAML parser error.
        source: serde_yaml::Error,
    },

    /// JSON report serialization failed.
    #[error("failed to serialize JSON report: {0}")]
    SerializeJson(#[from] serde_json::Error),

    /// A test case file violates the runner schema or invariants.
    #[error("{0}")]
    Validation(String),
}

/// Options controlling a conformance aid run.
#[derive(Debug, Clone)]
pub struct RunOptions {
    /// Case files or directories to load. Defaults to `tests/conformance/http`.
    pub case_roots: Vec<PathBuf>,
    /// Base LRS endpoint. If absent, only case validation is performed.
    pub endpoint: Option<Url>,
    /// Headers to inject into every HTTP request before case-specific headers.
    pub common_headers: BTreeMap<String, String>,
    /// Named authentication profiles available to case requests.
    pub auth_profiles: BTreeMap<String, AuthProfile>,
    /// Optional deterministic endpoint profile for profile-sensitive cases.
    pub endpoint_profile: Option<EndpointProfile>,
    /// Force validation-only mode even when an endpoint is present.
    pub validate_only: bool,
    /// Allow cases marked destructive to execute.
    pub allow_destructive: bool,
    /// Optional suite, case, and requirement filters.
    pub filter: RunFilter,
}

/// Request authentication material selected by a named test profile.
#[derive(Debug, Clone)]
pub struct AuthProfile {
    /// Headers injected when a request names this profile.
    pub headers: BTreeMap<String, String>,
}

/// Deterministic LRS endpoint profile for otherwise implementation-specific cases.
#[derive(Debug, Clone)]
pub struct EndpointProfile {
    profiles: BTreeMap<String, EndpointProfileCapability>,
}

/// Loads and validates an endpoint profile YAML document.
pub fn load_endpoint_profile(path: &Path) -> Result<EndpointProfile, Error> {
    let bytes = fs::read(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let document = serde_yaml::from_slice::<EndpointProfileDocument>(&bytes).map_err(|source| {
        Error::ParseYaml {
            path: path.to_path_buf(),
            source,
        }
    })?;
    EndpointProfile::from_document(path, document)
}

#[derive(Debug, Clone)]
enum EndpointProfileCapability {
    TooLarge(TooLargeEndpointProfile),
    RateLimit(RateLimitEndpointProfile),
    ParameterVocabulary(Vec<ParameterContext>),
    ResourceAbsence(BTreeMap<String, String>),
}

#[derive(Debug, Clone)]
struct TooLargeEndpointProfile {
    probe_body_bytes: usize,
}

#[derive(Debug, Clone)]
struct RateLimitEndpointProfile {
    request_count: usize,
    window_seconds: u64,
    path: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EndpointProfileDocument {
    schema_version: u16,
    standard: String,
    profiles: BTreeMap<String, EndpointProfileConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ParameterContext {
    method: String,
    path: String,
    /// Independent deployment/configuration evidence, never inferred from responses.
    evidence: String,
    unrecognized: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EndpointProfileConfig {
    /// Independent evidence for exact case selectors, valid immediately before
    /// that case and maintained throughout it; never inferred from its 404.
    #[serde(default)]
    case_evidence: BTreeMap<String, String>,
    #[serde(default)]
    contexts: Vec<ParameterContext>,
    #[serde(default)]
    max_request_body_bytes: Option<usize>,
    #[serde(default)]
    probe_body_bytes: Option<usize>,
    #[serde(default)]
    request_count: Option<usize>,
    #[serde(default)]
    window_seconds: Option<u64>,
    #[serde(default)]
    path: Option<String>,
}

impl EndpointProfile {
    fn from_document(path: &Path, document: EndpointProfileDocument) -> Result<Self, Error> {
        if document.schema_version != ENDPOINT_PROFILE_SCHEMA_VERSION {
            return Err(Error::Validation(format!(
                "{} endpoint profile schema_version must be {ENDPOINT_PROFILE_SCHEMA_VERSION}",
                path.display()
            )));
        }
        if document.standard != STANDARD {
            return Err(Error::Validation(format!(
                "{} endpoint profile standard must be {STANDARD}",
                path.display()
            )));
        }
        if document.profiles.is_empty() {
            return Err(Error::Validation(format!(
                "{} endpoint profile must declare at least one profile",
                path.display()
            )));
        }

        let mut profiles = BTreeMap::new();
        for (name, config) in document.profiles {
            validate_endpoint_profile_name(&format!("{} endpoint profile", path.display()), &name)?;
            let capability = endpoint_profile_capability(path, &name, &config)?;
            profiles.insert(name, capability);
        }
        Ok(Self { profiles })
    }

    fn has_profile(&self, name: &str) -> bool {
        self.profiles.contains_key(name)
    }

    fn get(&self, name: &str) -> Option<&EndpointProfileCapability> {
        self.profiles.get(name)
    }
}

/// Filters controlling which validated cases are included in a run.
#[derive(Debug, Clone, Default)]
pub struct RunFilter {
    /// Suite identifiers to include. Empty means all suites.
    pub suites: BTreeSet<String>,
    /// Case identifiers to include. Empty means all cases.
    pub cases: BTreeSet<String>,
    /// Requirement identifiers to include. Empty means all requirements.
    pub requirements: BTreeSet<String>,
}

impl RunFilter {
    fn is_empty(&self) -> bool {
        self.suites.is_empty() && self.cases.is_empty() && self.requirements.is_empty()
    }

    fn matches(&self, suite_id: &str, case: &TestCase) -> bool {
        (self.suites.is_empty() || self.suites.contains(suite_id))
            && (self.cases.is_empty() || self.cases.contains(&case.id))
            && (self.requirements.is_empty()
                || case
                    .requirements
                    .iter()
                    .any(|requirement| self.requirements.contains(requirement)))
    }
}

/// Runner mode recorded in the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunMode {
    /// Validate case files without sending HTTP requests.
    ValidateOnly,
    /// Execute HTTP requests against a supplied endpoint.
    ExecuteHttp,
}

impl Display for RunMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValidateOnly => formatter.write_str("validate-only"),
            Self::ExecuteHttp => formatter.write_str("execute-http"),
        }
    }
}

/// Overall report status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReportStatus {
    /// All case files were valid, and HTTP execution was not requested.
    Validated,
    /// All executed cases passed.
    Passed,
    /// At least one executed case failed.
    Failed,
    /// No failure was found, but required target fixtures were unavailable.
    Incomplete,
}

impl Display for ReportStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validated => formatter.write_str("validated"),
            Self::Passed => formatter.write_str("passed"),
            Self::Failed => formatter.write_str("failed"),
            Self::Incomplete => formatter.write_str("incomplete"),
        }
    }
}

/// Per-case outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CaseOutcome {
    /// The case file was valid, and HTTP execution was not requested.
    Validated,
    /// The case was executed and passed.
    Passed,
    /// The case was executed and failed.
    Failed,
    /// The target fixture needed to execute the case was unavailable.
    NotRun,
}

impl Display for CaseOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validated => formatter.write_str("validated"),
            Self::Passed => formatter.write_str("passed"),
            Self::Failed => formatter.write_str("failed"),
            Self::NotRun => formatter.write_str("not-run"),
        }
    }
}

/// Machine-readable conformance aid report.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// Machine-readable report schema version.
    pub schema_version: u16,
    /// Target standard for the loaded suite.
    pub standard: &'static str,
    /// Runner mode.
    pub mode: RunMode,
    /// Overall report status.
    pub status: ReportStatus,
    /// Aggregate case counts.
    pub totals: Totals,
    /// Per-suite results.
    pub suites: Vec<SuiteReport>,
}

impl Report {
    /// Returns whether the report should produce a successful process exit.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(self.status, ReportStatus::Validated | ReportStatus::Passed)
    }
}

/// Aggregate case counts.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct Totals {
    /// Total number of loaded cases.
    pub total: usize,
    /// Number of executed passing cases.
    pub passed: usize,
    /// Number of executed failing cases.
    pub failed: usize,
    /// Number of validation-only cases.
    pub validated: usize,
    /// Number of cases whose required target fixture was unavailable.
    pub not_run: usize,
}

/// Per-suite report.
#[derive(Debug, Clone, Serialize)]
pub struct SuiteReport {
    /// Suite identifier.
    pub id: String,
    /// Path to the YAML suite file.
    pub path: PathBuf,
    /// Per-case results.
    pub cases: Vec<CaseReport>,
}

/// Per-case report.
#[derive(Debug, Clone, Serialize)]
pub struct CaseReport {
    /// Stable test case identifier.
    pub id: String,
    /// Requirement identifiers discharged or exercised by this case.
    pub requirements: Vec<String>,
    /// Safety classification for this case.
    pub safety: CaseSafety,
    /// Case outcome.
    pub outcome: CaseOutcome,
    /// Human-readable failure messages.
    pub failures: Vec<String>,
    /// Missing fixture reasons; these do not assert target nonconformance.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub not_run_reasons: Vec<String>,
    /// Actual responses for status leaves excluded from the conformance claim.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub diagnostic_statuses: Vec<u16>,
    /// Each bounded retrieval observation, including failures before completion.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub retrieval_attempts: Vec<RetrievalAttempt>,
}

/// A bounded retrieval observation; no earlier mandatory failure is discarded.
#[derive(Debug, Clone, Serialize)]
pub struct RetrievalAttempt {
    step: Option<String>,
    status: Option<u16>,
    pending_visibility: bool,
    failures: Vec<String>,
}

/// Safety classification for a conformance test case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseSafety {
    /// Whether the case may mutate or delete target LRS state.
    pub destructive: bool,
    /// Whether the case requires an isolated disposable LRS target.
    pub requires_isolated_lrs: bool,
}

/// Loads, validates, and optionally executes conformance aid cases.
pub async fn run(options: RunOptions) -> Result<Report, Error> {
    let mode = if options.endpoint.is_some() && !options.validate_only {
        RunMode::ExecuteHttp
    } else {
        RunMode::ValidateOnly
    };

    let paths = collect_case_paths(&options.case_roots)?;
    let suites = load_suites(&paths)?;
    validate_suites(&suites)?;
    validate_header_map("common headers", &options.common_headers)?;
    validate_auth_profiles(&options.auth_profiles)?;
    validate_run_filter(&options.filter)?;
    let suites = filter_suites(suites, &options.filter)?;

    match (mode, options.endpoint.as_ref()) {
        (RunMode::ValidateOnly, _) => Ok(validation_report(suites)),
        (RunMode::ExecuteHttp, Some(endpoint)) => {
            execute_suites(
                suites,
                endpoint,
                &options.common_headers,
                &options.auth_profiles,
                options.endpoint_profile.as_ref(),
                options.allow_destructive,
            )
            .await
        }
        (RunMode::ExecuteHttp, None) => Err(Error::Validation(
            "HTTP execution requires an endpoint".to_owned(),
        )),
    }
}

#[derive(Debug)]
struct LoadedSuite {
    path: PathBuf,
    document: SuiteDocument,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SuiteDocument {
    suite: SuiteMetadata,
    cases: Vec<TestCase>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SuiteMetadata {
    id: String,
    standard: String,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TestCase {
    id: String,
    requirements: Vec<String>,
    safety: CaseSafety,
    #[serde(default)]
    requires_endpoint_profiles: Vec<String>,
    #[serde(default)]
    request: Option<RequestSpec>,
    #[serde(default)]
    expect: Option<ResponseExpectation>,
    #[serde(default)]
    steps: Vec<TestStep>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TestStep {
    id: String,
    /// Execute only when the immediately preceding, unconditional step returned this status.
    #[serde(default)]
    when_previous_status: Option<u16>,
    /// Branch on an earlier unconditional admission, even after intervening reads.
    #[serde(default)]
    when_step_status: Option<StepStatusCondition>,
    request: RequestSpec,
    expect: ResponseExpectation,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StepStatusCondition {
    step: String,
    status: u16,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestSpec {
    #[serde(default)]
    auth_profile: Option<String>,
    method: String,
    path: String,
    #[serde(default)]
    follow_more: Option<String>,
    /// Collect a bounded complete `StatementResult` chain before set predicates.
    #[serde(default)]
    collect_statement_pages: Option<usize>,
    #[serde(default)]
    query: BTreeMap<String, String>,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    #[serde(default, deserialize_with = "deserialize_optional_json")]
    body: Option<Value>,
    #[serde(default)]
    raw_body: Option<String>,
    #[serde(default)]
    raw_body_hex: Option<String>,
    #[serde(default)]
    await_retrieval_ms: Option<u64>,
    #[serde(default)]
    unrecognized_parameter: Option<String>,
    #[serde(default)]
    raw_body_crlf_lines: Option<Vec<String>>,
    #[serde(default)]
    repeat: Option<RepeatSpec>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RepeatSpec {
    count: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResponseExpectation {
    status: u16,
    #[serde(default)]
    status_classification: StatusClassification,
    #[serde(default)]
    status_in: Vec<u16>,
    #[serde(default)]
    checks: Vec<Check>,
    #[serde(default)]
    headers: BTreeMap<String, HeaderExpectation>,
    #[serde(default)]
    body: Option<BodyExpectation>,
    #[serde(default)]
    json: Option<JsonExpectation>,
    #[serde(default)]
    capture: BTreeMap<String, CaptureSpec>,
}

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum StatusClassification {
    #[default]
    Required,
    Diagnostic,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CaptureSpec {
    #[serde(default)]
    nonmatching_etag: bool,
    #[serde(default)]
    as_json: bool,
    #[serde(default)]
    pointer: Option<String>,
    #[serde(default)]
    header: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HeaderExpectation {
    #[serde(default)]
    present: Option<bool>,
    #[serde(default)]
    equals: Option<String>,
    #[serde(default)]
    contains: Option<String>,
    #[serde(default)]
    rfc3339: Option<bool>,
    #[serde(default)]
    iso8601: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BodyExpectation {
    #[serde(default)]
    non_empty: Option<bool>,
    #[serde(default)]
    contains: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonExpectation {
    pointers: BTreeMap<String, JsonPointerExpectation>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonPointerExpectation {
    #[serde(default)]
    present: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_json")]
    equals: Option<Value>,
    #[serde(default, deserialize_with = "deserialize_optional_json")]
    not_equals: Option<Value>,
    #[serde(default, deserialize_with = "deserialize_optional_json")]
    contains: Option<Value>,
    #[serde(default, deserialize_with = "deserialize_optional_json")]
    not_contains: Option<Value>,
}

fn deserialize_optional_json<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Value>, D::Error> {
    // An explicitly supplied JSON null is distinct from an omitted YAML field.
    Value::deserialize(deserializer).map(Some)
}

fn collect_case_paths(inputs: &[PathBuf]) -> Result<Vec<PathBuf>, Error> {
    let roots = if inputs.is_empty() {
        vec![PathBuf::from(DEFAULT_CASE_ROOT)]
    } else {
        inputs.to_vec()
    };
    let mut paths = Vec::new();
    for root in roots {
        require_generated_case_files(&root)?;
        collect_case_path(&root, &mut paths)?;
    }
    paths.sort();
    if paths.is_empty() {
        return Err(Error::Validation(
            "no YAML conformance case files found".to_owned(),
        ));
    }
    Ok(paths)
}

// A fresh repository checkout must not silently run only the handwritten
// subset. Explicit custom case files/directories remain supported.
fn require_generated_case_files(root: &Path) -> Result<(), Error> {
    if !root.ends_with(DEFAULT_CASE_ROOT) {
        return Ok(());
    }
    let inventory_path = root.join("../../../spec/conformance/lrs-generated-artifacts.json");
    let bytes = fs::read(&inventory_path).map_err(|source| Error::Read {
        path: inventory_path.clone(),
        source,
    })?;
    let inventory: Value = serde_json::from_slice(&bytes)
        .map_err(|error| Error::Validation(format!("invalid artifact inventory: {error}")))?;
    let stages = inventory["stages"]
        .as_array()
        .ok_or_else(|| Error::Validation("artifact inventory is missing stages".to_owned()))?;
    for stage in stages {
        let outputs = stage["outputs"]
            .as_array()
            .ok_or_else(|| Error::Validation("artifact stage is missing outputs".to_owned()))?;
        for output in outputs {
            let name = output
                .as_str()
                .ok_or_else(|| Error::Validation("artifact output must be a path".to_owned()))?;
            if let Some(relative) = name.strip_prefix("tests/conformance/http/")
                && !root.join(relative).is_file()
            {
                return Err(Error::Validation(format!(
                    "missing generated case file {name}; run python -B scripts/ci/lrs_generated_artifacts.py generate"
                )));
            }
        }
    }
    Ok(())
}

fn collect_case_path(path: &Path, output: &mut Vec<PathBuf>) -> Result<(), Error> {
    let metadata = fs::metadata(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;

    if metadata.is_file() {
        if is_yaml_path(path) {
            output.push(path.to_path_buf());
            return Ok(());
        }
        return Err(Error::Validation(format!(
            "case file path must end with .yaml or .yml: {}",
            path.display()
        )));
    }

    if !metadata.is_dir() {
        return Err(Error::Validation(format!(
            "case path must be a file or directory: {}",
            path.display()
        )));
    }

    let mut children = fs::read_dir(path)
        .map_err(|source| Error::Read {
            path: path.to_path_buf(),
            source,
        })?
        .map(|entry| {
            entry
                .map(|directory_entry| directory_entry.path())
                .map_err(|source| Error::Read {
                    path: path.to_path_buf(),
                    source,
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    children.sort();

    for child in children {
        if child.is_dir() || is_yaml_path(&child) {
            collect_case_path(&child, output)?;
        }
    }
    Ok(())
}

fn is_yaml_path(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| matches!(extension, "yaml" | "yml"))
}

fn load_suites(paths: &[PathBuf]) -> Result<Vec<LoadedSuite>, Error> {
    paths
        .iter()
        .map(|path| {
            let bytes = fs::read(path).map_err(|source| Error::Read {
                path: path.clone(),
                source,
            })?;
            let document = serde_yaml::from_slice::<SuiteDocument>(&bytes).map_err(|source| {
                Error::ParseYaml {
                    path: path.clone(),
                    source,
                }
            })?;
            Ok(LoadedSuite {
                path: path.clone(),
                document,
            })
        })
        .collect()
}

fn validate_suites(suites: &[LoadedSuite]) -> Result<(), Error> {
    let mut case_ids = BTreeSet::new();
    for loaded in suites {
        validate_suite_metadata(loaded)?;
        if loaded.document.cases.is_empty() {
            return Err(Error::Validation(format!(
                "{} must contain at least one case",
                loaded.path.display()
            )));
        }
        for case in &loaded.document.cases {
            validate_case(loaded, case, &mut case_ids)?;
        }
    }
    Ok(())
}

fn validate_suite_metadata(loaded: &LoadedSuite) -> Result<(), Error> {
    let metadata = &loaded.document.suite;
    if metadata.id.is_empty() {
        return Err(Error::Validation(format!(
            "{} suite.id must not be empty",
            loaded.path.display()
        )));
    }
    if metadata.standard != STANDARD {
        return Err(Error::Validation(format!(
            "{} suite.standard must be {STANDARD}",
            loaded.path.display()
        )));
    }
    if matches!(metadata.description.as_deref(), Some("")) {
        return Err(Error::Validation(format!(
            "{} suite.description must not be empty when present",
            loaded.path.display()
        )));
    }
    Ok(())
}

fn validate_case(
    loaded: &LoadedSuite,
    case: &TestCase,
    case_ids: &mut BTreeSet<String>,
) -> Result<(), Error> {
    let label = format!("{} {}", loaded.path.display(), case.id);
    if case.id.is_empty() {
        return Err(Error::Validation(format!(
            "{} case.id must not be empty",
            loaded.path.display()
        )));
    }
    if !case_ids.insert(case.id.clone()) {
        return Err(Error::Validation(format!("duplicate case id: {}", case.id)));
    }
    validate_requirements(&label, &case.requirements)?;
    validate_safety(&label, case.safety)?;
    validate_endpoint_profile_requirements(&label, &case.requires_endpoint_profiles)?;
    validate_case_shape(&label, case)?;
    validate_case_steps(&label, case)
}

fn validate_requirements(label: &str, requirements: &[String]) -> Result<(), Error> {
    if requirements.is_empty() {
        return Err(Error::Validation(format!(
            "{label} requirements must not be empty"
        )));
    }
    let mut seen = BTreeSet::new();
    for requirement in requirements {
        if requirement.is_empty() {
            return Err(Error::Validation(format!(
                "{label} requirements entries must not be empty"
            )));
        }
        if !seen.insert(requirement) {
            return Err(Error::Validation(format!(
                "{label} contains duplicate requirement {requirement}"
            )));
        }
    }
    Ok(())
}

fn validate_safety(label: &str, safety: CaseSafety) -> Result<(), Error> {
    if safety.destructive && !safety.requires_isolated_lrs {
        return Err(Error::Validation(format!(
            "{label} destructive cases must require an isolated LRS"
        )));
    }
    Ok(())
}

fn validate_endpoint_profile_requirements(label: &str, profiles: &[String]) -> Result<(), Error> {
    let mut seen = BTreeSet::new();
    for profile in profiles {
        validate_endpoint_profile_name(label, profile)?;
        if !seen.insert(profile) {
            return Err(Error::Validation(format!(
                "{label} repeats endpoint profile requirement {profile}"
            )));
        }
    }
    Ok(())
}

fn validate_case_shape(label: &str, case: &TestCase) -> Result<(), Error> {
    let has_single_request = case.request.is_some() || case.expect.is_some();
    let has_steps = !case.steps.is_empty();
    if has_single_request == has_steps {
        return Err(Error::Validation(format!(
            "{label} must specify either request/expect or steps"
        )));
    }
    if has_single_request && (case.request.is_none() || case.expect.is_none()) {
        return Err(Error::Validation(format!(
            "{label} request and expect must be specified together"
        )));
    }
    Ok(())
}

fn validate_case_steps(label: &str, case: &TestCase) -> Result<(), Error> {
    let mut available_captures = runtime_capture_names(&case.requires_endpoint_profiles);
    if let (Some(request), Some(expectation)) = (&case.request, &case.expect) {
        validate_request(&format!("{label} request"), request, &available_captures)?;
        validate_expectation(&format!("{label} expect"), expectation, &available_captures)?;
        validate_page_collection(label, request, expectation)?;
        return register_captures(
            &format!("{label} expect"),
            &expectation.capture,
            &mut available_captures,
        );
    }

    let mut step_ids = BTreeSet::new();
    let mut previous_step: Option<&TestStep> = None;
    let mut prior_steps: BTreeMap<&str, &TestStep> = BTreeMap::new();
    for step in &case.steps {
        if let Some(status) = step.when_previous_status {
            let valid_guard = previous_step.is_some_and(|previous| {
                previous.when_previous_status.is_none()
                    && previous.when_step_status.is_none()
                    && previous.expect.status_classification == StatusClassification::Required
                    && (status == previous.expect.status
                        || previous.expect.status_in.contains(&status))
            });
            if !valid_guard {
                return Err(Error::Validation(format!(
                    "{label} step {}: when_previous_status requires an allowed status from an immediately preceding unconditional step",
                    step.id
                )));
            }
        }
        if let Some(condition) = &step.when_step_status {
            let valid_guard = step.when_previous_status.is_none()
                && prior_steps
                    .get(condition.step.as_str())
                    .is_some_and(|prior| {
                        prior.when_previous_status.is_none()
                            && prior.when_step_status.is_none()
                            && prior.expect.status_classification == StatusClassification::Required
                            && (condition.status == prior.expect.status
                                || prior.expect.status_in.contains(&condition.status))
                    });
            if !valid_guard {
                return Err(Error::Validation(format!(
                    "{label} step {}: when_step_status requires an allowed status from an earlier unconditional step and cannot combine guards",
                    step.id
                )));
            }
        }
        prior_steps.insert(&step.id, step);
        previous_step = Some(step);
        let step_label = format!("{label} step {}", step.id);
        if step.id.is_empty() {
            return Err(Error::Validation(format!(
                "{label} step.id must not be empty"
            )));
        }
        if !step_ids.insert(&step.id) {
            return Err(Error::Validation(format!(
                "{label} contains duplicate step id {}",
                step.id
            )));
        }
        validate_request(
            &format!("{step_label} request"),
            &step.request,
            &available_captures,
        )?;
        validate_expectation(
            &format!("{step_label} expect"),
            &step.expect,
            &available_captures,
        )?;
        validate_page_collection(&step_label, &step.request, &step.expect)?;
        register_captures(
            &format!("{step_label} expect"),
            &step.expect.capture,
            &mut available_captures,
        )?;
    }
    Ok(())
}

fn validate_page_collection(
    label: &str,
    request: &RequestSpec,
    expectation: &ResponseExpectation,
) -> Result<(), Error> {
    if request.collect_statement_pages.is_some()
        && (expectation.status != 200
            || !expectation.status_in.is_empty()
            || expectation.status_classification != StatusClassification::Required
            || !expectation.capture.is_empty()
            || expectation.body.is_some()
            || expectation
                .json
                .as_ref()
                .is_some_and(|json| json.pointers.keys().any(|p| p != "/statements"))
            || expectation.checks.iter().any(|c| {
                !matches!(
                    c,
                    Check::HeaderTime { .. } | Check::JsonOrder { .. } | Check::JsonShape { .. }
                )
            }))
    {
        return Err(Error::Validation(format!(
            "{label}: collected pages require 200, no captures/body, aggregate /statements predicates and only shape/order/header-time checks"
        )));
    }
    Ok(())
}

#[expect(
    clippy::too_many_lines,
    reason = "Keep request field and cross-field validation together with their diagnostic labels"
)]
fn validate_request(
    label: &str,
    request: &RequestSpec,
    available_captures: &BTreeSet<String>,
) -> Result<(), Error> {
    if let Some(limit) = request.collect_statement_pages
        && (limit == 0
            || limit > 1000
            || request.method != "GET"
            || request.path != "/statements"
            || request.follow_more.is_some()
            || request.repeat.is_some()
            || request.query.contains_key("statementId")
            || request.query.contains_key("voidedStatementId"))
    {
        return Err(Error::Validation(format!(
            "{label}: collect_statement_pages requires an initial multiple-Statement GET, no repeat, and 1..1000 pages"
        )));
    }
    if let Some(more) = &request.follow_more {
        validate_template_references(label, more, available_captures)?;
        if request.method != "GET" || !request.query.is_empty() {
            return Err(Error::Validation(format!(
                "{label}: follow_more requires GET without added query parameters"
            )));
        }
    }
    if request.method.is_empty() {
        return Err(Error::Validation(format!(
            "{label} request.method must not be empty"
        )));
    }
    parse_method(&request.method).map_err(|message| {
        Error::Validation(format!("{label} request.method is invalid: {message}"))
    })?;
    validate_template_references(&format!("{label} path"), &request.path, available_captures)?;
    if !request.path.starts_with('/') && !is_endpoint_profile_path_template(&request.path) {
        return Err(Error::Validation(format!(
            "{label} request.path must start with / or use the endpoint profile path capture"
        )));
    }
    if let Some(profile_name) = &request.auth_profile {
        validate_auth_profile_name(label, profile_name)?;
    }
    for key in request.query.keys() {
        if key.is_empty() {
            return Err(Error::Validation(format!(
                "{label} query keys must not be empty"
            )));
        }
    }
    for (key, value) in &request.query {
        validate_template_references(
            &format!("{label} query parameter {key}"),
            value,
            available_captures,
        )?;
    }
    for key in request.headers.keys() {
        if key.is_empty() {
            return Err(Error::Validation(format!(
                "{label} header names must not be empty"
            )));
        }
    }
    for (key, value) in &request.headers {
        validate_template_references(&format!("{label} header {key}"), value, available_captures)?;
    }
    if let Some(body) = &request.body {
        validate_json_value_templates(&format!("{label} body"), body, available_captures)?;
    }
    if let Some(raw_body) = &request.raw_body {
        validate_template_references(&format!("{label} raw_body"), raw_body, available_captures)?;
    }
    if let Some(lines) = &request.raw_body_crlf_lines {
        for (index, line) in lines.iter().enumerate() {
            validate_template_references(
                &format!("{label} raw_body_crlf_lines[{index}]"),
                line,
                available_captures,
            )?;
        }
    }
    if let Some(repeat) = &request.repeat {
        validate_template_references(
            &format!("{label} repeat.count"),
            &repeat.count,
            available_captures,
        )?;
    }
    if let Some(hex) = &request.raw_body_hex {
        assertions::decode_hex(hex).map_err(Error::Validation)?;
    }
    if request
        .await_retrieval_ms
        .is_some_and(|ms| ms == 0 || ms > 60000)
        || request.await_retrieval_ms.is_some() && request.method != "GET"
    {
        return Err(Error::Validation(format!(
            "{label}: await_retrieval_ms requires GET and a timeout from 1 to 60000 ms"
        )));
    }
    let body_source_count = usize::from(request.raw_body_hex.is_some())
        + usize::from(request.body.is_some())
        + usize::from(request.raw_body.is_some())
        + usize::from(request.raw_body_crlf_lines.is_some());
    if body_source_count > 1 {
        return Err(Error::Validation(format!(
            "{label} must specify at most one request body source"
        )));
    }
    validate_header_map(&format!("{label} request.headers"), &request.headers)?;
    Ok(())
}

fn is_endpoint_profile_path_template(path: &str) -> bool {
    path == format!("{{{{{ENDPOINT_CAPTURE_RATE_LIMIT_PATH}}}}}")
}

fn validate_expectation(
    label: &str,
    expectation: &ResponseExpectation,
    available_captures: &BTreeSet<String>,
) -> Result<(), Error> {
    if !(100..=599).contains(&expectation.status) {
        return Err(Error::Validation(format!(
            "{label} expect.status must be an HTTP status code"
        )));
    }
    if expectation
        .status_in
        .iter()
        .any(|code| !(100..=599).contains(code))
    {
        return Err(Error::Validation(format!(
            "{label} expect.status_in contains an invalid HTTP status"
        )));
    }
    for check in &expectation.checks {
        check
            .validate()
            .and_then(|()| check.validate_captures(available_captures))
            .map_err(|message| Error::Validation(format!("{label}: {message}")))?;
    }
    for (header_name, header_expectation) in &expectation.headers {
        if header_name.is_empty() {
            return Err(Error::Validation(format!(
                "{label} expected header names must not be empty"
            )));
        }
        validate_header_expectation(label, header_name, header_expectation, available_captures)?;
    }
    if let Some(json) = &expectation.json {
        validate_json_expectation(label, json, available_captures)?;
    }
    if let Some(body) = &expectation.body {
        validate_body_expectation(label, body, available_captures)?;
    }
    validate_capture_specs(label, &expectation.capture)?;
    Ok(())
}

fn validate_header_expectation(
    label: &str,
    header_name: &str,
    expectation: &HeaderExpectation,
    available_captures: &BTreeSet<String>,
) -> Result<(), Error> {
    let condition_count = usize::from(expectation.present.is_some())
        + usize::from(expectation.equals.is_some())
        + usize::from(expectation.contains.is_some())
        + usize::from(expectation.rfc3339.is_some())
        + usize::from(expectation.iso8601.is_some());
    if condition_count != 1 {
        return Err(Error::Validation(format!(
            "{label} header expectation for {header_name} must specify exactly one condition"
        )));
    }
    if let Some(value) = &expectation.equals {
        validate_template_references(
            &format!("{label} header expectation for {header_name} equals"),
            value,
            available_captures,
        )?;
    }
    if let Some(value) = &expectation.contains {
        validate_template_references(
            &format!("{label} header expectation for {header_name} contains"),
            value,
            available_captures,
        )?;
    }
    if expectation.rfc3339 == Some(false) || expectation.iso8601 == Some(false) {
        return Err(Error::Validation(format!(
            "{label} header expectation for {header_name} rfc3339 must be true"
        )));
    }
    Ok(())
}

fn validate_body_expectation(
    label: &str,
    expectation: &BodyExpectation,
    available_captures: &BTreeSet<String>,
) -> Result<(), Error> {
    if expectation.non_empty.is_none() && expectation.contains.is_none() {
        return Err(Error::Validation(format!(
            "{label} body expectation must specify at least one condition"
        )));
    }
    if matches!(expectation.contains.as_deref(), Some("")) {
        return Err(Error::Validation(format!(
            "{label} body.contains must not be empty"
        )));
    }
    if let Some(value) = &expectation.contains {
        validate_template_references(&format!("{label} body.contains"), value, available_captures)?;
    }
    Ok(())
}

fn validate_json_expectation(
    label: &str,
    expectation: &JsonExpectation,
    available_captures: &BTreeSet<String>,
) -> Result<(), Error> {
    if expectation.pointers.is_empty() {
        return Err(Error::Validation(format!(
            "{label} json.pointers must not be empty"
        )));
    }
    for (pointer, pointer_expectation) in &expectation.pointers {
        if !pointer.is_empty() && !pointer.starts_with('/') {
            return Err(Error::Validation(format!(
                "{label} JSON pointer must be empty or start with /: {pointer}"
            )));
        }
        validate_json_pointer_expectation(label, pointer, pointer_expectation, available_captures)?;
    }
    Ok(())
}

fn validate_json_pointer_expectation(
    label: &str,
    pointer: &str,
    expectation: &JsonPointerExpectation,
    available_captures: &BTreeSet<String>,
) -> Result<(), Error> {
    let condition_count = usize::from(expectation.present.is_some())
        + usize::from(expectation.equals.is_some())
        + usize::from(expectation.not_equals.is_some())
        + usize::from(expectation.contains.is_some())
        + usize::from(expectation.not_contains.is_some());
    if condition_count != 1 {
        return Err(Error::Validation(format!(
            "{label} JSON pointer expectation for {pointer} must specify exactly one condition"
        )));
    }
    if let Some(value) = &expectation.equals {
        validate_json_value_templates(
            &format!("{label} JSON pointer expectation for {pointer} equals"),
            value,
            available_captures,
        )?;
    }
    if let Some(value) = &expectation.not_equals {
        validate_json_value_templates(
            &format!("{label} JSON pointer expectation for {pointer} not_equals"),
            value,
            available_captures,
        )?;
    }
    if let Some(value) = &expectation.contains {
        validate_json_value_templates(
            &format!("{label} JSON pointer expectation for {pointer} contains"),
            value,
            available_captures,
        )?;
    }
    if let Some(value) = &expectation.not_contains {
        validate_json_value_templates(
            &format!("{label} JSON pointer expectation for {pointer} not_contains"),
            value,
            available_captures,
        )?;
    }
    Ok(())
}

fn validate_capture_specs(
    label: &str,
    captures: &BTreeMap<String, CaptureSpec>,
) -> Result<(), Error> {
    for (name, capture) in captures {
        if capture.nonmatching_etag
            && (capture.as_json
                || !capture
                    .header
                    .as_deref()
                    .is_some_and(|h| h.eq_ignore_ascii_case("ETag")))
        {
            return Err(Error::Validation(format!(
                "{label}: nonmatching_etag requires an ETag header capture"
            )));
        }
        if capture.as_json && capture.header.is_some() {
            return Err(Error::Validation(format!(
                "{label}: as_json capture requires a JSON pointer"
            )));
        }
        validate_capture_name(label, name)?;
        let source_count =
            usize::from(capture.pointer.is_some()) + usize::from(capture.header.is_some());
        if source_count != 1 {
            return Err(Error::Validation(format!(
                "{label} capture {name} must specify exactly one source"
            )));
        }
        if let Some(pointer) = &capture.pointer
            && !pointer.is_empty()
            && !pointer.starts_with('/')
        {
            return Err(Error::Validation(format!(
                "{label} capture {name} JSON pointer must be empty or start with /: {pointer}"
            )));
        }
        if let Some(header) = &capture.header {
            validate_header_name(&format!("{label} capture {name} header"), header)?;
        }
    }
    Ok(())
}

fn register_captures(
    label: &str,
    captures: &BTreeMap<String, CaptureSpec>,
    available_captures: &mut BTreeSet<String>,
) -> Result<(), Error> {
    for name in captures.keys() {
        if !available_captures.insert(name.clone()) {
            return Err(Error::Validation(format!(
                "{label} capture {name} is already defined"
            )));
        }
    }
    Ok(())
}

fn validate_json_value_templates(
    label: &str,
    value: &Value,
    available_captures: &BTreeSet<String>,
) -> Result<(), Error> {
    match value {
        Value::String(template) => {
            validate_template_references(label, template, available_captures)
        }
        Value::Array(values) => {
            for (index, item) in values.iter().enumerate() {
                validate_json_value_templates(
                    &format!("{label}[{index}]"),
                    item,
                    available_captures,
                )?;
            }
            Ok(())
        }
        Value::Object(values) => {
            for (key, item) in values {
                validate_json_value_templates(&format!("{label}.{key}"), item, available_captures)?;
            }
            Ok(())
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => Ok(()),
    }
}

fn validate_template_references(
    label: &str,
    template: &str,
    available_captures: &BTreeSet<String>,
) -> Result<(), Error> {
    let variables = template_variables(template)
        .map_err(|message| Error::Validation(format!("{label} template is invalid: {message}")))?;
    for variable in variables {
        if !available_captures.contains(&variable) {
            return Err(Error::Validation(format!(
                "{label} references undefined capture {variable}"
            )));
        }
    }
    Ok(())
}

fn template_variables(template: &str) -> Result<BTreeSet<String>, String> {
    let mut variables = BTreeSet::new();
    let mut remaining = template;
    while let Some(start) = remaining.find("{{") {
        // Closing braces outside an opened capture are literal data, including
        // the adjacent braces at the end of nested JSON query parameters.
        let after_open = &remaining[start + 2..];
        let Some(end) = after_open.find("}}") else {
            return Err("unclosed opening delimiter {{".to_owned());
        };
        let name = &after_open[..end];
        validate_capture_name_text(name)?;
        variables.insert(name.to_owned());
        remaining = &after_open[end + 2..];
    }
    Ok(variables)
}

fn validate_capture_name(label: &str, name: &str) -> Result<(), Error> {
    validate_capture_name_text(name)
        .map_err(|message| Error::Validation(format!("{label} capture name is invalid: {message}")))
}

fn validate_capture_name_text(name: &str) -> Result<(), String> {
    let Some(first) = name.chars().next() else {
        return Err("capture name must not be empty".to_owned());
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return Err("capture name must start with ASCII alphabetic or _".to_owned());
    }
    if !name
        .chars()
        .all(|character| character == '_' || character.is_ascii_alphanumeric())
    {
        return Err("capture name must contain only ASCII alphanumeric or _".to_owned());
    }
    Ok(())
}

fn parse_method(method: &str) -> Result<Method, String> {
    Method::from_bytes(method.as_bytes()).map_err(|error| error.to_string())
}

fn validate_header_map(label: &str, headers: &BTreeMap<String, String>) -> Result<(), Error> {
    build_header_map(headers)
        .map(|_headers| ())
        .map_err(|message| {
            Error::Validation(format!("{label} contains invalid HTTP header: {message}"))
        })
}

fn validate_auth_profiles(profiles: &BTreeMap<String, AuthProfile>) -> Result<(), Error> {
    for (name, profile) in profiles {
        validate_auth_profile_name("auth profile", name)?;
        if profile.headers.is_empty() {
            return Err(Error::Validation(format!(
                "auth profile {name} must contain at least one header"
            )));
        }
        validate_header_map(&format!("auth profile {name} headers"), &profile.headers)?;
    }
    Ok(())
}

fn endpoint_profile_capability(
    path: &Path,
    name: &str,
    config: &EndpointProfileConfig,
) -> Result<EndpointProfileCapability, Error> {
    if name != "declared-resource-absence" && !config.case_evidence.is_empty() {
        return Err(Error::Validation(
            "case_evidence is only valid for declared-resource-absence".into(),
        ));
    }
    match name {
        "declared-resource-absence" => {
            if config.case_evidence.is_empty()
                || config
                    .case_evidence
                    .iter()
                    .any(|(case, evidence)| case.trim().is_empty() || evidence.trim().is_empty())
            {
                return Err(Error::Validation("resource absence needs per-case independent evidence, not response-derived absence".into()));
            }
            reject_endpoint_profile_fields(
                path,
                name,
                &[
                    ("contexts", !config.contexts.is_empty()),
                    (
                        "max_request_body_bytes",
                        config.max_request_body_bytes.is_some(),
                    ),
                    ("probe_body_bytes", config.probe_body_bytes.is_some()),
                    ("request_count", config.request_count.is_some()),
                    ("window_seconds", config.window_seconds.is_some()),
                    ("path", config.path.is_some()),
                ],
            )?;
            Ok(EndpointProfileCapability::ResourceAbsence(
                config.case_evidence.clone(),
            ))
        }
        "declared-parameter-vocabulary" => {
            if config.contexts.is_empty()
                || config.contexts.iter().any(|c| {
                    c.evidence.trim().is_empty()
                        || !c.path.starts_with('/')
                        || !matches!(
                            c.method.as_str(),
                            "GET" | "HEAD" | "PUT" | "POST" | "DELETE"
                        )
                        || c.unrecognized.is_empty()
                        || c.unrecognized.iter().any(String::is_empty)
                })
            {
                return Err(Error::Validation("parameter vocabulary needs method, path, independent evidence and nonempty unrecognized names".into()));
            }
            reject_endpoint_profile_fields(
                path,
                name,
                &[
                    (
                        "max_request_body_bytes",
                        config.max_request_body_bytes.is_some(),
                    ),
                    ("probe_body_bytes", config.probe_body_bytes.is_some()),
                    ("request_count", config.request_count.is_some()),
                    ("window_seconds", config.window_seconds.is_some()),
                    ("path", config.path.is_some()),
                ],
            )?;
            Ok(EndpointProfileCapability::ParameterVocabulary(
                config.contexts.clone(),
            ))
        }
        ENDPOINT_PROFILE_TOO_LARGE => endpoint_profile_too_large(path, config),
        ENDPOINT_PROFILE_RATE_LIMIT => endpoint_profile_rate_limit(path, config),
        _ => Err(Error::Validation(format!(
            "{} endpoint profile {name} is not supported",
            path.display()
        ))),
    }
}

fn endpoint_profile_too_large(
    path: &Path,
    config: &EndpointProfileConfig,
) -> Result<EndpointProfileCapability, Error> {
    reject_endpoint_profile_fields(
        path,
        ENDPOINT_PROFILE_TOO_LARGE,
        &[
            ("contexts", !config.contexts.is_empty()),
            ("request_count", config.request_count.is_some()),
            ("window_seconds", config.window_seconds.is_some()),
            ("path", config.path.is_some()),
        ],
    )?;
    let max_request_body_bytes = config.max_request_body_bytes.ok_or_else(|| {
        Error::Validation(format!(
            "{} endpoint profile {ENDPOINT_PROFILE_TOO_LARGE}.max_request_body_bytes is required",
            path.display()
        ))
    })?;
    let probe_body_bytes = config.probe_body_bytes.ok_or_else(|| {
        Error::Validation(format!(
            "{} endpoint profile {ENDPOINT_PROFILE_TOO_LARGE}.probe_body_bytes is required",
            path.display()
        ))
    })?;
    if max_request_body_bytes == 0 {
        return Err(Error::Validation(format!(
            "{} endpoint profile {ENDPOINT_PROFILE_TOO_LARGE}.max_request_body_bytes must be positive",
            path.display()
        )));
    }
    if probe_body_bytes <= max_request_body_bytes {
        return Err(Error::Validation(format!(
            "{} endpoint profile {ENDPOINT_PROFILE_TOO_LARGE}.probe_body_bytes must exceed max_request_body_bytes",
            path.display()
        )));
    }
    if probe_body_bytes > MAX_ENDPOINT_PROFILE_PROBE_BODY_BYTES {
        return Err(Error::Validation(format!(
            "{} endpoint profile {ENDPOINT_PROFILE_TOO_LARGE}.probe_body_bytes must be <= {MAX_ENDPOINT_PROFILE_PROBE_BODY_BYTES}",
            path.display()
        )));
    }
    Ok(EndpointProfileCapability::TooLarge(
        TooLargeEndpointProfile { probe_body_bytes },
    ))
}

fn endpoint_profile_rate_limit(
    path: &Path,
    config: &EndpointProfileConfig,
) -> Result<EndpointProfileCapability, Error> {
    reject_endpoint_profile_fields(
        path,
        ENDPOINT_PROFILE_RATE_LIMIT,
        &[
            ("contexts", !config.contexts.is_empty()),
            (
                "max_request_body_bytes",
                config.max_request_body_bytes.is_some(),
            ),
            ("probe_body_bytes", config.probe_body_bytes.is_some()),
        ],
    )?;
    let request_count = config.request_count.ok_or_else(|| {
        Error::Validation(format!(
            "{} endpoint profile {ENDPOINT_PROFILE_RATE_LIMIT}.request_count is required",
            path.display()
        ))
    })?;
    let window_seconds = config.window_seconds.ok_or_else(|| {
        Error::Validation(format!(
            "{} endpoint profile {ENDPOINT_PROFILE_RATE_LIMIT}.window_seconds is required",
            path.display()
        ))
    })?;
    let path_value = config.path.as_ref().ok_or_else(|| {
        Error::Validation(format!(
            "{} endpoint profile {ENDPOINT_PROFILE_RATE_LIMIT}.path is required",
            path.display()
        ))
    })?;
    if request_count == 0 || request_count > MAX_ENDPOINT_PROFILE_RATE_LIMIT_REQUESTS {
        return Err(Error::Validation(format!(
            "{} endpoint profile {ENDPOINT_PROFILE_RATE_LIMIT}.request_count must be between 1 and {MAX_ENDPOINT_PROFILE_RATE_LIMIT_REQUESTS}",
            path.display()
        )));
    }
    if window_seconds == 0 || window_seconds > MAX_ENDPOINT_PROFILE_RATE_LIMIT_WINDOW_SECONDS {
        return Err(Error::Validation(format!(
            "{} endpoint profile {ENDPOINT_PROFILE_RATE_LIMIT}.window_seconds must be between 1 and {MAX_ENDPOINT_PROFILE_RATE_LIMIT_WINDOW_SECONDS}",
            path.display()
        )));
    }
    if !path_value.starts_with('/') {
        return Err(Error::Validation(format!(
            "{} endpoint profile {ENDPOINT_PROFILE_RATE_LIMIT}.path must start with /",
            path.display()
        )));
    }
    Ok(EndpointProfileCapability::RateLimit(
        RateLimitEndpointProfile {
            request_count,
            window_seconds,
            path: path_value.clone(),
        },
    ))
}

fn reject_endpoint_profile_fields(
    path: &Path,
    name: &str,
    fields: &[(&str, bool)],
) -> Result<(), Error> {
    for (field, present) in fields {
        if *present {
            return Err(Error::Validation(format!(
                "{} endpoint profile {name}.{field} is not allowed",
                path.display()
            )));
        }
    }
    Ok(())
}

fn validate_run_filter(filter: &RunFilter) -> Result<(), Error> {
    for suite in &filter.suites {
        if suite.is_empty() {
            return Err(Error::Validation(
                "suite filter entries must not be empty".to_owned(),
            ));
        }
    }
    for case in &filter.cases {
        if case.is_empty() {
            return Err(Error::Validation(
                "case filter entries must not be empty".to_owned(),
            ));
        }
    }
    for requirement in &filter.requirements {
        if requirement.is_empty() {
            return Err(Error::Validation(
                "requirement filter entries must not be empty".to_owned(),
            ));
        }
    }
    Ok(())
}

fn filter_suites(suites: Vec<LoadedSuite>, filter: &RunFilter) -> Result<Vec<LoadedSuite>, Error> {
    if filter.is_empty() {
        return Ok(suites);
    }

    let mut filtered_suites = Vec::new();
    for mut loaded in suites {
        let suite_id = loaded.document.suite.id.clone();
        loaded
            .document
            .cases
            .retain(|case| filter.matches(&suite_id, case));
        if !loaded.document.cases.is_empty() {
            filtered_suites.push(loaded);
        }
    }

    if filtered_suites.is_empty() {
        return Err(Error::Validation(
            "filters selected no conformance cases".to_owned(),
        ));
    }
    Ok(filtered_suites)
}

fn validate_auth_profile_name(label: &str, name: &str) -> Result<(), Error> {
    validate_auth_profile_name_text(name)
        .map_err(|message| Error::Validation(format!("{label} auth_profile is invalid: {message}")))
}

fn validate_endpoint_profile_name(label: &str, name: &str) -> Result<(), Error> {
    validate_auth_profile_name_text(name).map_err(|message| {
        Error::Validation(format!(
            "{label} endpoint profile name is invalid: {message}"
        ))
    })?;
    if !matches!(
        name,
        ENDPOINT_PROFILE_TOO_LARGE
            | ENDPOINT_PROFILE_RATE_LIMIT
            | "declared-parameter-vocabulary"
            | "signature-validation-context"
            | "declared-resource-absence"
    ) {
        return Err(Error::Validation(format!(
            "{label} endpoint profile {name} is not supported"
        )));
    }
    Ok(())
}

fn validate_auth_profile_name_text(name: &str) -> Result<(), String> {
    let Some(first) = name.chars().next() else {
        return Err("profile name must not be empty".to_owned());
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return Err("profile name must start with ASCII alphabetic or _".to_owned());
    }
    if !name
        .chars()
        .all(|character| character == '_' || character == '-' || character.is_ascii_alphanumeric())
    {
        return Err("profile name must contain only ASCII alphanumeric, _, or -".to_owned());
    }
    Ok(())
}

fn validate_header_name(label: &str, name: &str) -> Result<(), Error> {
    HeaderName::from_bytes(name.as_bytes())
        .map(|_header| ())
        .map_err(|message| {
            Error::Validation(format!(
                "{label} contains invalid HTTP header name: {message}"
            ))
        })
}

fn validation_report(suites: Vec<LoadedSuite>) -> Report {
    let suite_reports = suites
        .into_iter()
        .map(|loaded| SuiteReport {
            id: loaded.document.suite.id,
            path: loaded.path,
            cases: loaded
                .document
                .cases
                .into_iter()
                .map(|case| CaseReport {
                    id: case.id,
                    requirements: case.requirements,
                    safety: case.safety,
                    outcome: CaseOutcome::Validated,
                    failures: Vec::new(),
                    not_run_reasons: Vec::new(),
                    diagnostic_statuses: Vec::new(),
                    retrieval_attempts: Vec::new(),
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    let totals = calculate_totals(&suite_reports);
    Report {
        schema_version: REPORT_SCHEMA_VERSION,
        standard: STANDARD,
        mode: RunMode::ValidateOnly,
        status: ReportStatus::Validated,
        totals,
        suites: suite_reports,
    }
}

async fn execute_suites(
    suites: Vec<LoadedSuite>,
    endpoint: &Url,
    common_headers: &BTreeMap<String, String>,
    auth_profiles: &BTreeMap<String, AuthProfile>,
    endpoint_profile: Option<&EndpointProfile>,
    allow_destructive: bool,
) -> Result<Report, Error> {
    let client = Client::new();
    let mut suite_reports = Vec::new();
    for loaded in suites {
        let mut case_reports = Vec::new();
        for case in loaded.document.cases {
            case_reports.push(
                execute_case(
                    &client,
                    endpoint,
                    common_headers,
                    auth_profiles,
                    endpoint_profile,
                    allow_destructive,
                    case,
                )
                .await,
            );
        }
        suite_reports.push(SuiteReport {
            id: loaded.document.suite.id,
            path: loaded.path,
            cases: case_reports,
        });
    }
    let totals = calculate_totals(&suite_reports);
    let status = if totals.failed > 0 {
        ReportStatus::Failed
    } else if totals.not_run > 0 {
        ReportStatus::Incomplete
    } else {
        ReportStatus::Passed
    };
    Ok(Report {
        schema_version: REPORT_SCHEMA_VERSION,
        standard: STANDARD,
        mode: RunMode::ExecuteHttp,
        status,
        totals,
        suites: suite_reports,
    })
}

async fn execute_case(
    client: &Client,
    endpoint: &Url,
    common_headers: &BTreeMap<String, String>,
    auth_profiles: &BTreeMap<String, AuthProfile>,
    endpoint_profile: Option<&EndpointProfile>,
    allow_destructive: bool,
    case: TestCase,
) -> CaseReport {
    let mut failures = Vec::new();
    if case.safety.destructive && !allow_destructive {
        let mut report = case_report(case, CaseOutcome::NotRun, Vec::new());
        report.not_run_reasons.push(
            "destructive case requires --allow-destructive; no HTTP request was executed"
                .to_owned(),
        );
        return report;
    }

    let mut missing_profiles = missing_endpoint_profiles(&case, endpoint_profile);
    for request in case
        .steps
        .iter()
        .map(|step| &step.request)
        .chain(case.request.iter())
    {
        if let Some(parameter) = &request.unrecognized_parameter {
            let declared = endpoint_profile
                .and_then(|p| p.get("declared-parameter-vocabulary"))
                .is_some_and(|capability| match capability {
                    EndpointProfileCapability::ParameterVocabulary(contexts) => {
                        contexts.iter().any(|c| {
                            c.method == request.method
                                && c.path == request.path
                                && c.unrecognized.contains(parameter)
                        })
                    }
                    _ => false,
                });
            if !request.query.contains_key(parameter) || !declared {
                missing_profiles.push(format!(
                    "independent unrecognized parameter: {} {} {parameter}",
                    request.method, request.path
                ));
            }
        }
        if let Some(profile) = &request.auth_profile
            && !auth_profiles.contains_key(profile)
        {
            missing_profiles.push(format!("auth:{profile}"));
        }
    }
    missing_profiles.sort();
    missing_profiles.dedup();
    if !missing_profiles.is_empty() {
        let reason = format!(
            "required target fixture profile(s) unavailable: {}",
            missing_profiles.join(", ")
        );
        let mut report = case_report(case, CaseOutcome::NotRun, Vec::new());
        report.not_run_reasons.push(reason);
        return report;
    }

    let mut diagnostic_statuses = Vec::new();
    let (request_failures, retrieval_pending, retrieval_attempts) = execute_case_requests(
        client,
        endpoint,
        common_headers,
        auth_profiles,
        endpoint_profile,
        &case,
        &mut diagnostic_statuses,
    )
    .await;
    failures.extend(request_failures);

    let mut report = if retrieval_pending {
        let mut report = case_report(case, CaseOutcome::NotRun, Vec::new());
        report.not_run_reasons.extend(failures);
        report
    } else if failures.is_empty() {
        case_report(case, CaseOutcome::Passed, failures)
    } else {
        case_report(case, CaseOutcome::Failed, failures)
    };
    report.diagnostic_statuses = diagnostic_statuses;
    report.retrieval_attempts = retrieval_attempts;
    report
}

async fn execute_case_requests(
    client: &Client,
    endpoint: &Url,
    common_headers: &BTreeMap<String, String>,
    auth_profiles: &BTreeMap<String, AuthProfile>,
    endpoint_profile: Option<&EndpointProfile>,
    case: &TestCase,
    diagnostic_statuses: &mut Vec<u16>,
) -> (Vec<String>, bool, Vec<RetrievalAttempt>) {
    let captures = match runtime_captures(endpoint_profile, &case.requires_endpoint_profiles) {
        Ok(captures) => captures,
        Err(error) => {
            return (
                vec![format!("failed to create runtime captures: {error}")],
                false,
                Vec::new(),
            );
        }
    };
    let context = HttpRequestContext {
        client,
        endpoint,
        common_headers,
        auth_profiles,
    };
    let mut state = HttpRequestState {
        captures,
        diagnostic_statuses,
        retrieval_attempts: Vec::new(),
        step: None,
        response_status: None,
        retrieval_pending: false,
        capability_unavailable: false,
        visible_statements: BTreeMap::new(),
    };
    if case.steps.is_empty() {
        let failures = match (&case.request, &case.expect) {
            (Some(request), Some(expectation)) => {
                execute_http_request(&context, request, expectation, &mut state).await
            }
            _ => vec!["case has no executable request".to_owned()],
        };
        return (
            failures,
            state.retrieval_pending || state.capability_unavailable,
            state.retrieval_attempts,
        );
    }

    let mut failures = Vec::new();
    let mut step_statuses = BTreeMap::new();
    for step in &case.steps {
        if step
            .when_previous_status
            .is_some_and(|status| state.response_status != Some(status))
            || step.when_step_status.as_ref().is_some_and(|condition| {
                step_statuses.get(&condition.step) != Some(&condition.status)
            })
        {
            state.response_status = None;
            continue;
        }
        state.step = Some(step.id.clone());
        let step_failures =
            execute_http_request(&context, &step.request, &step.expect, &mut state).await;
        if let Some(status) = state.response_status {
            step_statuses.insert(step.id.clone(), status);
        }
        if !step_failures.is_empty() {
            failures.extend(
                step_failures
                    .into_iter()
                    .map(|failure| format!("step {}: {failure}", step.id)),
            );
            break;
        }
    }
    (
        failures,
        state.retrieval_pending || state.capability_unavailable,
        state.retrieval_attempts,
    )
}

struct HttpRequestContext<'a> {
    client: &'a Client,
    endpoint: &'a Url,
    common_headers: &'a BTreeMap<String, String>,
    auth_profiles: &'a BTreeMap<String, AuthProfile>,
}

struct HttpRequestState<'a> {
    captures: Captures,
    diagnostic_statuses: &'a mut Vec<u16>,
    retrieval_attempts: Vec<RetrievalAttempt>,
    step: Option<String>,
    response_status: Option<u16>,
    retrieval_pending: bool,
    capability_unavailable: bool,
    // Key includes endpoint selector and effective credential context. Values
    // are independently returned stored times, not submitted timestamps.
    visible_statements: BTreeMap<String, String>,
}

// Bounded observation window for LRSs that make writes retrievable asynchronously.
// Each attempt uses a private capture set; only a fully checked response commits captures.
async fn execute_http_request(
    context: &HttpRequestContext<'_>,
    request: &RequestSpec,
    expectation: &ResponseExpectation,
    state: &mut HttpRequestState<'_>,
) -> Vec<String> {
    let deadline = tokio::time::Instant::now()
        + std::time::Duration::from_millis(request.await_retrieval_ms.unwrap_or(0));
    loop {
        let mut attempt_diagnostics = Vec::new();
        let mut attempt = HttpRequestState {
            captures: state.captures.clone(),
            diagnostic_statuses: &mut attempt_diagnostics,
            retrieval_attempts: Vec::new(),
            step: state.step.clone(),
            response_status: None,
            retrieval_pending: false,
            capability_unavailable: false,
            visible_statements: state.visible_statements.clone(),
        };
        let failures = execute_http_request_once(context, request, expectation, &mut attempt).await;
        attempt.capability_unavailable |= !failures.is_empty()
            && failures
                .iter()
                .all(|reason| reason.starts_with("ORACLE-UNAVAILABLE:"));
        if request.await_retrieval_ms.is_some() {
            state.retrieval_attempts.push(RetrievalAttempt {
                step: state.step.clone(),
                status: attempt.response_status,
                pending_visibility: attempt.retrieval_pending,
                failures: failures.clone(),
            });
        }
        state.response_status = attempt.response_status;
        state.retrieval_pending = attempt.retrieval_pending;
        state.capability_unavailable = attempt.capability_unavailable;
        if failures.is_empty() {
            state.captures = attempt.captures;
            state.visible_statements = attempt.visible_statements;
            state.diagnostic_statuses.extend(attempt_diagnostics);
            return failures;
        }
        if !attempt.retrieval_pending || tokio::time::Instant::now() >= deadline {
            state.diagnostic_statuses.extend(attempt_diagnostics);
            return failures;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "Preserve one ordered request lifecycle including header failures before unavailable bodies"
)]
async fn execute_http_request_once(
    context: &HttpRequestContext<'_>,
    request: &RequestSpec,
    expectation: &ResponseExpectation,
    state: &mut HttpRequestState<'_>,
) -> Vec<String> {
    let mut failures = Vec::new();
    let request = match render_request(request, &state.captures) {
        Ok(rendered) => rendered,
        Err(error) => {
            failures.push(format!("failed to render request: {error}"));
            return failures;
        }
    };
    if !request.path.starts_with('/') {
        failures.push(format!(
            "rendered request path must start with /: {}",
            request.path
        ));
        return failures;
    }
    let request_url = if let Some(more) = &request.follow_more {
        match context.endpoint.join(more) {
            Ok(url) if url.origin() == context.endpoint.origin() => url,
            Ok(_) => {
                // IEEE imposes no same-origin rule. This runner currently has
                // no separately configured destination/credential capability.
                // Do not forward endpoint credentials or report an IEEE FAIL.
                state.capability_unavailable = true;
                return vec!["ORACLE-UNAVAILABLE: cross-origin continuation requires an explicit destination and credential capability; more IRL origin alone is not an IEEE violation".into()];
            }
            Err(error) => return vec![format!("invalid continuation IRL: {error}")],
        }
    } else {
        build_request_url(context.endpoint, &request)
    };
    let method = match parse_method(&request.method) {
        Ok(method) => method,
        Err(error) => {
            failures.push(format!("invalid HTTP method: {error}"));
            return failures;
        }
    };
    let headers =
        match build_request_headers(context.common_headers, context.auth_profiles, &request) {
            Ok(headers) => headers,
            Err(error) => {
                failures.push(format!("invalid HTTP headers: {error}"));
                return failures;
            }
        };
    let repeat_count = match request_repeat_count(&request) {
        Ok(count) => count,
        Err(error) => {
            failures.push(format!("invalid repeat count: {error}"));
            return failures;
        }
    };

    let mut final_response = None;
    for attempt in 1..=repeat_count {
        let mut request_builder = context
            .client
            .request(method.clone(), request_url.clone())
            .headers(headers.clone());
        if let Some(body) = &request.body {
            request_builder = request_builder.json(body);
        } else if let Some(raw_body) = &request.raw_body {
            request_builder = request_builder.body(raw_body.clone());
        } else if let Some(hex) = &request.raw_body_hex {
            match assertions::decode_hex(hex) {
                Ok(bytes) => request_builder = request_builder.body(bytes),
                Err(error) => return vec![error],
            }
        } else if let Some(lines) = &request.raw_body_crlf_lines {
            request_builder = request_builder.body(lines.join("\r\n"));
        }

        match request_builder.send().await {
            Ok(response) => {
                if attempt < repeat_count {
                    // Warm-up status/body can differ from the final threshold
                    // response; universal header obligations cannot disappear.
                    for (name, expected) in &expectation.headers {
                        if name.eq_ignore_ascii_case("X-Experience-API-Version")
                            || name.eq_ignore_ascii_case("X-Experience-API-Consistent-Through")
                        {
                            evaluate_header(
                                name,
                                expected,
                                response
                                    .headers()
                                    .get(name.as_str())
                                    .and_then(|value| value.to_str().ok()),
                                &state.captures,
                                &mut failures,
                            );
                        }
                    }
                    if failures
                        .iter()
                        .any(|error| !error.starts_with("ORACLE-UNAVAILABLE:"))
                    {
                        return failures;
                    }
                }
                final_response = Some(response);
            }
            Err(error) => {
                failures.push(format!("ORACLE-UNAVAILABLE: request transport did not complete at attempt {attempt}: {error}"));
                return failures;
            }
        }
    }

    let Some(response) = final_response else {
        failures.push("HTTP request was not sent".to_owned());
        return failures;
    };

    let status = response.status();
    state.response_status = Some(status.as_u16());
    if expectation.status_classification == StatusClassification::Diagnostic {
        state.diagnostic_statuses.push(status.as_u16());
    }
    let headers = response.headers().clone();
    let visibility_key =
        statement_visibility_key(&request, &headers_for_visibility(context, &request));
    let pending_retrieval = pending_statement_retrieval(&request, expectation, status);
    let stored = visibility_key
        .as_ref()
        .and_then(|key| state.visible_statements.get(key))
        .cloned();
    let body_bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(error) => {
            if pending_retrieval {
                failures.extend(evaluate_pending_retrieval(
                    expectation,
                    &headers,
                    state,
                    stored.as_deref(),
                    false,
                ));
            } else {
                failures.extend(evaluate_status(expectation, status));
                failures.extend(evaluate_headers(expectation, &headers, &state.captures));
                for check in &expectation.checks {
                    failures.extend(check.evaluate_without_body(&headers));
                }
            }
            failures.push(format!(
                "ORACLE-UNAVAILABLE: response body did not complete: {error}"
            ));
            return failures;
        }
    };
    if let Some(limit) = request.collect_statement_pages {
        return evaluate_statement_pages(
            context,
            &request,
            expectation,
            state,
            status,
            headers,
            body_bytes.to_vec(),
            limit,
        )
        .await;
    }
    if pending_retrieval {
        failures.extend(evaluate_pending_retrieval(
            expectation,
            &headers,
            state,
            stored.as_deref(),
            true,
        ));
        return failures;
    }
    failures.extend(evaluate_response(
        expectation,
        status,
        &headers,
        &body_bytes,
        state,
    ));
    if failures.is_empty() {
        if request.path == "/statements"
            && matches!(request.method.as_str(), "PUT" | "POST" | "DELETE")
        {
            // An intervening mutation can change void/retention state. Do not
            // carry a positive observation through an unmodelled mutation.
            state.visible_statements.clear();
        } else if status == StatusCode::OK
            && let Some(key) = visibility_key
            && let Ok(json) = serde_json::from_slice::<Value>(&body_bytes)
            && let Some(stored) = json.get("stored").and_then(Value::as_str)
            && request
                .query
                .get("statementId")
                .or_else(|| request.query.get("voidedStatementId"))
                .is_some_and(|id| json.get("id").and_then(Value::as_str) == Some(id.as_str()))
        {
            state.visible_statements.insert(key, stored.to_owned());
        }
    }
    failures
}

// Page checks, prefix order and observed exclusions do not depend on completion.
// Required membership and absence success require the terminal union.
#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "Page aggregation carries the already received response and preserves per-page failures before incomplete collection"
)]
async fn evaluate_statement_pages(
    context: &HttpRequestContext<'_>,
    request: &RequestSpec,
    expectation: &ResponseExpectation,
    state: &mut HttpRequestState<'_>,
    mut status: StatusCode,
    mut headers: HeaderMap,
    mut bytes: Vec<u8>,
    limit: usize,
) -> Vec<String> {
    let mut failures = Vec::new();
    let mut statements = Vec::new();
    let mut visited = BTreeSet::new();
    let mut reported_order_failures = BTreeSet::new();
    for page in 0..limit {
        failures.extend(evaluate_status(expectation, status));
        failures.extend(evaluate_headers(expectation, &headers, &state.captures));
        for check in &expectation.checks {
            // Root StatementResult order is checked over the observed union
            // below. Other checks retain their page-local response context.
            if !matches!(check, Check::JsonOrder { pointer, .. } if pointer == "/statements") {
                failures.extend(check.evaluate(status.as_u16(), &headers, &bytes, &state.captures));
            }
        }
        if status != StatusCode::OK {
            failures.push("StatementResult page did not return 200".into());
            return failures;
        }
        let document: Value = match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(error) => {
                failures.push(format!("invalid StatementResult page JSON: {error}"));
                return failures;
            }
        };
        let Some(items) = document.get("statements").and_then(Value::as_array) else {
            failures.push("StatementResult page lacks a statements array".into());
            return failures;
        };
        statements.extend(items.iter().cloned());
        for (index, check) in expectation.checks.iter().enumerate() {
            // A known inversion must survive a later page/transport limit.
            // Recheck the prefix for new defects, but report each check's
            // identical diagnosis only once across subsequent pages.
            for failure in check.evaluate_collected_order(&statements) {
                if reported_order_failures.insert((index, failure.clone())) {
                    failures.push(failure);
                }
            }
        }
        if let Some(predicate) = expectation
            .json
            .as_ref()
            .and_then(|j| j.pointers.get("/statements"))
        {
            if predicate.present == Some(false) {
                failures.push("JSON pointer /statements is present on an observed page".into());
            }
            if let Some(expected) = &predicate.equals {
                match render_json_value(expected, &state.captures) {
                    Ok(Value::Array(expected)) if statements.len() <= expected.len() => {
                        match assertions::all_comparisons(
                            statements
                                .iter()
                                .zip(&expected)
                                .map(|(a, b)| assertions::json_equal(a, b)),
                        ) {
                            Ok(false) => failures.push(
                                "Observed StatementResult prefix cannot equal the expected array"
                                    .into(),
                            ),
                            Err(error) => failures.push(error),
                            Ok(true) => {}
                        }
                    }
                    Ok(_) => failures.push(
                        "Observed StatementResult type/length cannot equal the expected value"
                            .into(),
                    ),
                    Err(error) => failures.push(error),
                }
            }
        }
        if let Some(predicate) = expectation
            .json
            .as_ref()
            .and_then(|j| j.pointers.get("/statements"))
            && let Some(excluded) = &predicate.not_contains
        {
            match render_json_value(excluded, &state.captures)
                .and_then(|value| json_contains(&Value::Array(statements.clone()), &value))
            {
                Ok(true) => failures.push(
                    "JSON pointer /statements not_contains failed on an observed page".into(),
                ),
                // An unknown comparison must not erase a known match on a
                // later page. Keep the unresolved obligation until completion.
                Err(error) => failures.push(error),
                Ok(false) => {}
            }
        }
        let more = match document.get("more") {
            None => "", // End-of-result omission remains in the unselected D-03 set.
            Some(Value::String(value)) => value.as_str(),
            _ => {
                failures.push("StatementResult more is not a string".into());
                return failures;
            }
        };
        if more.is_empty() {
            let aggregate = serde_json::json!({"statements": statements, "more": ""});
            failures.extend(evaluate_body(
                expectation,
                aggregate.to_string().as_bytes(),
                &mut state.captures,
            ));
            return failures;
        }
        if page + 1 == limit {
            failures.push("ORACLE-UNAVAILABLE: continuation exceeds harness page bound; membership/absence not judged".into());
            return failures;
        }
        let url = match context.endpoint.join(more) {
            Ok(url) if url.origin() == context.endpoint.origin() => url,
            _ => {
                failures.push("ORACLE-UNAVAILABLE: continuation destination needs an explicit routing/credential capability".into());
                return failures;
            }
        };
        if !visited.insert(url.to_string()) {
            failures.push("ORACLE-UNAVAILABLE: repeated continuation cannot be completed within this bounded collector".into());
            return failures;
        }
        let request_headers =
            match build_request_headers(context.common_headers, context.auth_profiles, request) {
                Ok(value) => value,
                Err(error) => {
                    failures.push(error);
                    return failures;
                }
            };
        let response = match context
            .client
            .get(url)
            .headers(request_headers)
            .send()
            .await
        {
            Ok(value) => value,
            Err(error) => {
                failures.push(format!(
                    "ORACLE-UNAVAILABLE: continuation transport did not complete: {error}"
                ));
                return failures;
            }
        };
        status = response.status();
        headers = response.headers().clone();
        bytes = match response.bytes().await {
            Ok(value) => value.to_vec(),
            Err(error) => {
                failures.extend(evaluate_status(expectation, status));
                failures.extend(evaluate_headers(expectation, &headers, &state.captures));
                for check in &expectation.checks {
                    failures.extend(check.evaluate_without_body(&headers));
                }
                failures.push(format!(
                    "ORACLE-UNAVAILABLE: continuation body did not complete: {error}"
                ));
                return failures;
            }
        };
    }
    failures
}

fn headers_for_visibility(context: &HttpRequestContext<'_>, request: &RequestSpec) -> HeaderMap {
    // Already validated before sending the request. This private map is never
    // emitted in diagnostics; credential values must not enter report text.
    build_request_headers(context.common_headers, context.auth_profiles, request)
        .unwrap_or_default()
}

fn statement_visibility_key(request: &RequestSpec, headers: &HeaderMap) -> Option<String> {
    if request.path != "/statements" || request.method != "GET" {
        return None;
    }
    let (selector, id) = ["statementId", "voidedStatementId"]
        .into_iter()
        .find_map(|name| request.query.get(name).map(|value| (name, value)))?;
    let identity: BTreeMap<_, _> = headers
        .iter()
        .filter(|(name, _)| {
            !matches!(
                name.as_str(),
                "accept"
                    | "accept-language"
                    | "accept-encoding"
                    | "content-type"
                    | "content-length"
            )
        })
        .map(|(name, value)| (name.as_str(), value.as_bytes()))
        .collect();
    Some(format!("{selector}:{id}:{identity:?}"))
}

fn pending_statement_retrieval(
    request: &RequestSpec,
    expectation: &ResponseExpectation,
    status: StatusCode,
) -> bool {
    request.await_retrieval_ms.is_some()
        && request.method == "GET"
        && request.path == "/statements"
        && (request.query.contains_key("statementId")
            || request.query.contains_key("voidedStatementId"))
        && expectation.status == 200
        && status == StatusCode::NOT_FOUND
}

fn evaluate_pending_retrieval(
    expectation: &ResponseExpectation,
    headers: &HeaderMap,
    state: &mut HttpRequestState<'_>,
    stored: Option<&str>,
    body_complete: bool,
) -> Vec<String> {
    // An observation deadline is a harness limit, not an IEEE deadline.
    // Error headers still apply; success-only JSON and Last-Modified do not.
    let mut failures = Vec::new();
    for name in [
        "X-Experience-API-Version",
        "X-Experience-API-Consistent-Through",
    ] {
        if let Some(expected) = expectation.headers.get(name) {
            evaluate_header(
                name,
                expected,
                headers.get(name).and_then(|v| v.to_str().ok()),
                &state.captures,
                &mut failures,
            );
        }
    }
    if !failures.is_empty() {
        return failures;
    }
    if let Some(stored) = stored {
        // Positive visibility and received headers remain evidence even when
        // this error response's body is unavailable.
        let through = headers
            .get("X-Experience-API-Consistent-Through")
            .and_then(|v| v.to_str().ok());
        if through.map(|v| assertions::compare_timestamps(v, stored))
            == Some(Ok(std::cmp::Ordering::Greater))
        {
            failures.push("previously visible Statement disappeared with Consistent-Through strictly after its stored time in the same credential/selector context".into());
        } else {
            state.capability_unavailable = true;
            failures.push("ORACLE-UNAVAILABLE: previously visible Statement is absent but its consistency watermark is not established; no initial-delay retry or PASS".into());
        }
    } else if body_complete {
        state.retrieval_pending = true;
        failures.push("Statement not yet retrievable within the configured observation window; no normative availability deadline is specified".to_owned());
    }
    failures
}

fn evaluate_response(
    expectation: &ResponseExpectation,
    status: StatusCode,
    headers: &HeaderMap,
    body_bytes: &[u8],
    state: &mut HttpRequestState<'_>,
) -> Vec<String> {
    let mut failures = Vec::new();
    failures.extend(evaluate_status(expectation, status));
    failures.extend(evaluate_headers(expectation, headers, &state.captures));
    failures.extend(evaluate_body(expectation, body_bytes, &mut state.captures));
    for check in &expectation.checks {
        failures.extend(check.evaluate(status.as_u16(), headers, body_bytes, &state.captures));
    }
    failures.extend(capture_header_values(
        expectation,
        headers,
        &mut state.captures,
    ));
    failures
}

fn request_repeat_count(request: &RequestSpec) -> Result<usize, String> {
    let Some(repeat) = &request.repeat else {
        return Ok(1);
    };
    let count = repeat
        .count
        .parse::<usize>()
        .map_err(|error| error.to_string())?;
    if count == 0 || count > MAX_ENDPOINT_PROFILE_RATE_LIMIT_REQUESTS {
        return Err(format!(
            "repeat.count must be between 1 and {MAX_ENDPOINT_PROFILE_RATE_LIMIT_REQUESTS}"
        ));
    }
    Ok(count)
}

fn render_request(request: &RequestSpec, captures: &Captures) -> Result<RequestSpec, String> {
    Ok(RequestSpec {
        auth_profile: request.auth_profile.clone(),
        method: render_template(&request.method, captures)?,
        path: render_template(&request.path, captures)?,
        follow_more: request
            .follow_more
            .as_ref()
            .map(|more| render_template(more, captures))
            .transpose()?,
        collect_statement_pages: request.collect_statement_pages,
        query: render_string_map(&request.query, captures)?,
        headers: render_string_map(&request.headers, captures)?,
        body: request
            .body
            .as_ref()
            .map(|body| render_json_value(body, captures))
            .transpose()?,
        raw_body_hex: request.raw_body_hex.clone(),
        await_retrieval_ms: request.await_retrieval_ms,
        unrecognized_parameter: request.unrecognized_parameter.clone(),
        raw_body: request
            .raw_body
            .as_ref()
            .map(|raw_body| render_template(raw_body, captures))
            .transpose()?,
        raw_body_crlf_lines: request
            .raw_body_crlf_lines
            .as_ref()
            .map(|lines| render_string_list(lines, captures))
            .transpose()?,
        repeat: request
            .repeat
            .as_ref()
            .map(|repeat| -> Result<RepeatSpec, String> {
                Ok(RepeatSpec {
                    count: render_template(&repeat.count, captures)?,
                })
            })
            .transpose()?,
    })
}

fn render_string_map(
    values: &BTreeMap<String, String>,
    captures: &Captures,
) -> Result<BTreeMap<String, String>, String> {
    values
        .iter()
        .map(|(name, value)| Ok((name.clone(), render_template(value, captures)?)))
        .collect()
}

fn render_string_list(values: &[String], captures: &Captures) -> Result<Vec<String>, String> {
    values
        .iter()
        .map(|value| render_template(value, captures))
        .collect()
}

fn render_json_value(value: &Value, captures: &Captures) -> Result<Value, String> {
    match value {
        Value::String(template) => Ok(Value::String(render_template(template, captures)?)),
        Value::Array(values) => values
            .iter()
            .map(|item| render_json_value(item, captures))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Object(values) => values
            .iter()
            .map(|(key, item)| Ok((key.clone(), render_json_value(item, captures)?)))
            .collect::<Result<serde_json::Map<_, _>, _>>()
            .map(Value::Object),
        Value::Null | Value::Bool(_) | Value::Number(_) => Ok(value.clone()),
    }
}

fn render_template(template: &str, captures: &Captures) -> Result<String, String> {
    let mut rendered = String::new();
    let mut remaining = template;
    while let Some(start) = remaining.find("{{") {
        rendered.push_str(&remaining[..start]);
        let after_open = &remaining[start + 2..];
        let Some(end) = after_open.find("}}") else {
            return Err("unclosed opening delimiter {{".to_owned());
        };
        let name = &after_open[..end];
        validate_capture_name_text(name)?;
        let value = captures
            .get(name)
            .ok_or_else(|| format!("undefined capture {name}"))?;
        rendered.push_str(value);
        remaining = &after_open[end + 2..];
    }
    rendered.push_str(remaining);
    Ok(rendered)
}

fn runtime_capture_names(required_endpoint_profiles: &[String]) -> BTreeSet<String> {
    let mut names = BTreeSet::from([RUNTIME_CAPTURE_NOW_PLUS_30S.to_owned()]);
    for profile in required_endpoint_profiles {
        match profile.as_str() {
            ENDPOINT_PROFILE_TOO_LARGE => {
                names.insert(ENDPOINT_CAPTURE_TOO_LARGE_BODY.to_owned());
            }
            ENDPOINT_PROFILE_RATE_LIMIT => {
                names.insert(ENDPOINT_CAPTURE_RATE_LIMIT_REQUEST_COUNT.to_owned());
                names.insert(ENDPOINT_CAPTURE_RATE_LIMIT_WINDOW_SECONDS.to_owned());
                names.insert(ENDPOINT_CAPTURE_RATE_LIMIT_PATH.to_owned());
            }
            _ => {}
        }
    }
    names
}

fn runtime_captures(
    endpoint_profile: Option<&EndpointProfile>,
    required_endpoint_profiles: &[String],
) -> Result<Captures, String> {
    let timestamp = OffsetDateTime::now_utc()
        .saturating_add(Duration::seconds(30))
        .format(&Rfc3339)
        .map_err(|error| error.to_string())?;
    let mut captures = Captures::from([(RUNTIME_CAPTURE_NOW_PLUS_30S.to_owned(), timestamp)]);
    add_endpoint_profile_captures(endpoint_profile, required_endpoint_profiles, &mut captures)?;
    Ok(captures)
}

fn missing_endpoint_profiles(
    case: &TestCase,
    endpoint_profile: Option<&EndpointProfile>,
) -> Vec<String> {
    case.requires_endpoint_profiles
        .iter()
        .filter(|profile| {
            if profile.as_str() == "declared-resource-absence" {
                return !matches!(endpoint_profile.and_then(|p| p.get(profile)), Some(EndpointProfileCapability::ResourceAbsence(evidence)) if evidence.contains_key(&case.id));
            }
            endpoint_profile
                .as_ref()
                .is_none_or(|endpoint_profile| !endpoint_profile.has_profile(profile))
        })
        .cloned()
        .collect()
}

fn add_endpoint_profile_captures(
    endpoint_profile: Option<&EndpointProfile>,
    required_endpoint_profiles: &[String],
    captures: &mut Captures,
) -> Result<(), String> {
    let Some(endpoint_profile) = endpoint_profile else {
        if required_endpoint_profiles.is_empty() {
            return Ok(());
        }
        return Err("endpoint profile was not supplied".to_owned());
    };
    for profile in required_endpoint_profiles {
        let capability = endpoint_profile
            .get(profile)
            .ok_or_else(|| format!("endpoint profile {profile} was not supplied"))?;
        match capability {
            EndpointProfileCapability::ParameterVocabulary(_)
            | EndpointProfileCapability::ResourceAbsence(_) => {}
            EndpointProfileCapability::TooLarge(profile) => {
                captures.insert(
                    ENDPOINT_CAPTURE_TOO_LARGE_BODY.to_owned(),
                    too_large_statement_body(profile.probe_body_bytes)?,
                );
            }
            EndpointProfileCapability::RateLimit(profile) => {
                captures.insert(
                    ENDPOINT_CAPTURE_RATE_LIMIT_REQUEST_COUNT.to_owned(),
                    profile.request_count.to_string(),
                );
                captures.insert(
                    ENDPOINT_CAPTURE_RATE_LIMIT_WINDOW_SECONDS.to_owned(),
                    profile.window_seconds.to_string(),
                );
                captures.insert(
                    ENDPOINT_CAPTURE_RATE_LIMIT_PATH.to_owned(),
                    profile.path.clone(),
                );
            }
        }
    }
    Ok(())
}

fn too_large_statement_body(min_bytes: usize) -> Result<String, String> {
    let mut padding = String::new();
    loop {
        let body = serde_json::to_string(&serde_json::json!({
            "id": "00000000-0000-4000-8000-000000000901",
            "actor": {
                "objectType": "Agent",
                "mbox": "mailto:kiva-conformance-too-large@example.org"
            },
            "verb": {
                "id": "https://adlnet.gov/expapi/verbs/experienced",
                "display": {
                    "en-US": "experienced"
                }
            },
            "object": {
                "objectType": "Activity",
                "id": "https://example.org/kiva/conformance/activity/too-large"
            },
            "context": {
                "extensions": {
                    "https://example.org/kiva/conformance/extensions/too-large-padding": padding.as_str()
                }
            },
            "version": "2.0.0"
        }))
        .map_err(|error| error.to_string())?;
        if body.len() >= min_bytes {
            return Ok(body);
        }
        padding.push_str(&"x".repeat(min_bytes - body.len()));
    }
}

fn build_request_headers(
    common_headers: &BTreeMap<String, String>,
    auth_profiles: &BTreeMap<String, AuthProfile>,
    request: &RequestSpec,
) -> Result<HeaderMap, String> {
    let mut headers = build_header_map(common_headers)?;
    if let Some(profile_name) = &request.auth_profile {
        let profile = auth_profiles
            .get(profile_name)
            .ok_or_else(|| format!("auth profile {profile_name} was not supplied"))?;
        for (name, value) in &profile.headers {
            insert_header(&mut headers, name, value)?;
        }
    }
    for (name, value) in &request.headers {
        insert_header(&mut headers, name, value)?;
    }
    Ok(headers)
}

fn build_header_map(headers: &BTreeMap<String, String>) -> Result<HeaderMap, String> {
    let mut header_map = HeaderMap::new();
    for (name, value) in headers {
        insert_header(&mut header_map, name, value)?;
    }
    Ok(header_map)
}

fn insert_header(headers: &mut HeaderMap, name: &str, value: &str) -> Result<(), String> {
    let header_name = HeaderName::from_bytes(name.as_bytes()).map_err(|error| error.to_string())?;
    let header_value = HeaderValue::from_str(value).map_err(|error| error.to_string())?;
    headers.insert(header_name, header_value);
    Ok(())
}

fn build_request_url(endpoint: &Url, request: &RequestSpec) -> Url {
    let mut url = endpoint.clone();
    let base_path = endpoint.path().trim_end_matches('/');
    let request_path = request.path.trim_start_matches('/');
    let joined_path = if base_path.is_empty() || base_path == "/" {
        format!("/{request_path}")
    } else {
        format!("{base_path}/{request_path}")
    };
    url.set_path(&joined_path);
    url.set_query(None);
    for (name, value) in &request.query {
        url.query_pairs_mut().append_pair(name, value);
    }
    url
}

fn evaluate_status(expectation: &ResponseExpectation, actual: StatusCode) -> Vec<String> {
    if expectation.status_classification == StatusClassification::Diagnostic
        || actual.as_u16() == expectation.status
        || expectation.status_in.contains(&actual.as_u16())
    {
        Vec::new()
    } else {
        vec![format!(
            "expected status {}, received {}",
            expectation.status,
            actual.as_u16()
        )]
    }
}

fn evaluate_headers(
    expectation: &ResponseExpectation,
    headers: &HeaderMap,
    captures: &Captures,
) -> Vec<String> {
    let mut failures = Vec::new();
    for (name, header_expectation) in &expectation.headers {
        let actual = headers
            .get(name.as_str())
            .and_then(|value| value.to_str().ok());
        evaluate_header(name, header_expectation, actual, captures, &mut failures);
    }
    failures
}

fn evaluate_header(
    name: &str,
    expectation: &HeaderExpectation,
    actual: Option<&str>,
    captures: &Captures,
    failures: &mut Vec<String>,
) {
    if let Some(present) = expectation.present
        && actual.is_some() != present
    {
        failures.push(format!("expected header {name} present={present}"));
    }
    if let Some(expected) = &expectation.equals {
        match render_template(expected, captures) {
            Ok(rendered) if actual == Some(rendered.as_str()) => {}
            Ok(rendered) => failures.push(format!(
                "expected header {name} to equal {rendered:?}, received {actual:?}"
            )),
            Err(error) => failures.push(format!("failed to render header {name}: {error}")),
        }
    }
    if let Some(expected) = &expectation.contains {
        match render_template(expected, captures) {
            Ok(rendered)
                if actual
                    .is_some_and(|actual_value| header_contains(name, actual_value, &rendered)) => {
            }
            Ok(rendered) => failures.push(format!(
                "expected header {name} to contain {rendered:?}, received {actual:?}"
            )),
            Err(error) => failures.push(format!("failed to render header {name}: {error}")),
        }
    }
    if expectation.rfc3339 == Some(true)
        && actual.is_none_or(|value| {
            time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
                .is_err()
        })
    {
        failures.push(format!(
            "expected header {name} to contain an RFC3339 timestamp, received {actual:?}"
        ));
    }
    if expectation.iso8601 == Some(true) {
        match actual {
            Some(value) => {
                if let Err(error) = assertions::parse_time(value) {
                    failures.push(error);
                }
            }
            None => failures.push(format!("missing required ISO timestamp header {name}")),
        }
    }
}

// Content-Type assertions name a media type, not a substring. Parameters in the
// assertion constrain the response; additional response parameters are allowed.
fn header_contains(name: &str, actual: &str, expected: &str) -> bool {
    if !name.eq_ignore_ascii_case("content-type") {
        return actual.contains(expected);
    }
    let (Ok(actual), Ok(expected)) = (
        media_type::parse_http(actual),
        media_type::parse_http(expected),
    ) else {
        return false;
    };
    actual.essence == expected.essence
        && expected.ambiguous.is_empty()
        && expected
            .parameters
            .iter()
            .all(|(name, value)| actual.parameters.get(name) == Some(value))
}

fn evaluate_body(
    expectation: &ResponseExpectation,
    body_bytes: &[u8],
    captures: &mut Captures,
) -> Vec<String> {
    let mut failures = evaluate_plain_body(expectation, body_bytes, captures);
    if expectation.json.is_none() && !captures_json_body(expectation) {
        return failures;
    }
    let body = match serde_json::from_slice::<Value>(body_bytes) {
        Ok(body) => body,
        Err(error) => {
            failures.push(format!("expected JSON response body: {error}"));
            return failures;
        }
    };

    if let Some(json_expectation) = &expectation.json {
        for (pointer, pointer_expectation) in &json_expectation.pointers {
            let actual = body.pointer(pointer);
            evaluate_json_pointer(
                pointer,
                pointer_expectation,
                actual,
                captures,
                &mut failures,
            );
        }
    }
    failures.extend(capture_json_values(expectation, &body, captures));
    failures
}

fn evaluate_plain_body(
    expectation: &ResponseExpectation,
    body_bytes: &[u8],
    captures: &Captures,
) -> Vec<String> {
    let Some(body_expectation) = &expectation.body else {
        return Vec::new();
    };
    let mut failures = Vec::new();
    if let Some(non_empty) = body_expectation.non_empty
        && (body_bytes.is_empty() == non_empty)
    {
        failures.push(format!("expected response body non_empty={non_empty}"));
    }
    if let Some(expected) = &body_expectation.contains {
        let Ok(body_text) = std::str::from_utf8(body_bytes) else {
            failures.push("text response body is not valid UTF-8".to_owned());
            return failures;
        };
        match render_template(expected, captures) {
            Ok(rendered) if body_text.contains(&rendered) => {}
            Ok(rendered) => {
                failures.push(format!("expected response body to contain {rendered:?}"));
            }
            Err(error) => failures.push(format!("failed to render body expectation: {error}")),
        }
    }
    failures
}

fn evaluate_json_pointer(
    pointer: &str,
    expectation: &JsonPointerExpectation,
    actual: Option<&Value>,
    captures: &Captures,
    failures: &mut Vec<String>,
) {
    if let Some(present) = expectation.present
        && actual.is_some() != present
    {
        failures.push(format!("expected JSON pointer {pointer} present={present}"));
    }
    for (expected, operation) in [
        (&expectation.equals, "equals"),
        (&expectation.not_equals, "not_equals"),
        (&expectation.contains, "contains"),
        (&expectation.not_contains, "not_contains"),
    ] {
        let Some(expected) = expected else {
            continue;
        };
        let rendered = match render_json_value(expected, captures) {
            Ok(value) => value,
            Err(error) => {
                failures.push(format!("failed to render JSON pointer {pointer}: {error}"));
                continue;
            }
        };
        let Some(actual) = actual else {
            failures.push(format!("missing JSON pointer {pointer} for {operation}"));
            continue;
        };
        let result = if matches!(operation, "contains" | "not_contains") {
            json_contains(actual, &rendered)
        } else {
            assertions::json_equal(actual, &rendered)
        };
        let negated = matches!(operation, "not_equals" | "not_contains");
        match result {
            Ok(matched) if matched != negated => {}
            Ok(_) => failures.push(format!(
                "JSON pointer {pointer} {operation} failed for {rendered}"
            )),
            Err(error) => failures.push(format!("{error}; JSON pointer {pointer} {operation}")),
        }
    }
}

fn capture_json_values(
    expectation: &ResponseExpectation,
    body: &Value,
    captures: &mut Captures,
) -> Vec<String> {
    let mut failures = Vec::new();
    for (name, capture) in &expectation.capture {
        let Some(pointer) = &capture.pointer else {
            continue;
        };
        let Some(actual) = body.pointer(pointer) else {
            failures.push(format!(
                "expected JSON pointer {pointer} for capture {name}"
            ));
            continue;
        };
        if let Err(error) = assertions::require_exact_numbers(actual) {
            failures.push(format!("{error}; capture {name} at {pointer}"));
            continue;
        }
        let serialized_value = if capture.as_json {
            Some(actual.to_string())
        } else {
            capture_scalar_value(actual)
        };
        let Some(value) = serialized_value else {
            failures.push(format!(
                "capture {name} at JSON pointer {pointer} must be a scalar"
            ));
            continue;
        };
        captures.insert(name.clone(), value);
    }
    failures
}

fn capture_header_values(
    expectation: &ResponseExpectation,
    headers: &HeaderMap,
    captures: &mut Captures,
) -> Vec<String> {
    let mut failures = Vec::new();
    for (name, capture) in &expectation.capture {
        let Some(header) = &capture.header else {
            continue;
        };
        let actual = headers
            .get(header.as_str())
            .and_then(|value| value.to_str().ok());
        match actual {
            Some(value) => {
                if capture.nonmatching_etag {
                    let opaque = value.strip_prefix("W/").unwrap_or(value);
                    if !opaque.starts_with('"') || !opaque.ends_with('"') || opaque.len() < 2 {
                        failures.push("ETag capture is not a quoted entity-tag".into());
                        continue;
                    }
                    // Append within the quoted opaque tag, preserving every
                    // existing byte. This strong tag cannot equal the source.
                    let different = format!("{}!\"", &opaque[..opaque.len() - 1]);
                    captures.insert(name.clone(), different);
                } else {
                    captures.insert(name.clone(), value.to_owned());
                }
            }
            None => failures.push(format!("expected header {header} for capture {name}")),
        }
    }
    failures
}

fn captures_json_body(expectation: &ResponseExpectation) -> bool {
    expectation
        .capture
        .values()
        .any(|capture| capture.pointer.is_some())
}

fn capture_scalar_value(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
    }
}

fn json_contains(actual: &Value, expected: &Value) -> Result<bool, String> {
    match (actual, expected) {
        (Value::Array(values), expected_value) => assertions::any_comparison(
            values
                .iter()
                .map(|value| json_matches_subset(value, expected_value)),
        ),
        (Value::String(actual_value), Value::String(expected_value)) => {
            Ok(actual_value.contains(expected_value))
        }
        (Value::Object(_), Value::Object(_)) => json_matches_subset(actual, expected),
        (actual_value, expected_value) => assertions::json_equal(actual_value, expected_value),
    }
}

fn json_matches_subset(actual: &Value, expected: &Value) -> Result<bool, String> {
    if let (Value::Object(values), Value::Object(subset)) = (actual, expected) {
        return assertions::all_comparisons(subset.iter().map(|(key, value)| {
            values
                .get(key)
                .map_or(Ok(false), |actual| json_matches_subset(actual, value))
        }));
    }
    assertions::json_equal(actual, expected)
}

fn case_report(case: TestCase, outcome: CaseOutcome, failures: Vec<String>) -> CaseReport {
    CaseReport {
        id: case.id,
        requirements: case.requirements,
        safety: case.safety,
        outcome,
        failures,
        not_run_reasons: Vec::new(),
        diagnostic_statuses: Vec::new(),
        retrieval_attempts: Vec::new(),
    }
}

fn calculate_totals(suites: &[SuiteReport]) -> Totals {
    let mut totals = Totals::default();
    for suite in suites {
        for case in &suite.cases {
            totals.total += 1;
            match case.outcome {
                CaseOutcome::Validated => totals.validated += 1,
                CaseOutcome::Passed => totals.passed += 1,
                CaseOutcome::Failed => totals.failed += 1,
                CaseOutcome::NotRun => totals.not_run += 1,
            }
        }
    }
    totals
}

#[cfg(test)]
mod tests {
    async fn response_control(
        request: serde_json::Value,
        expect: serde_json::Value,
        responses: Vec<String>,
    ) -> super::CaseReport {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("local fixture");
        let address = listener.local_addr().expect("fixture address");
        let responder = std::thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().expect("fixture request");
                let mut request = [0_u8; 4096];
                let _ = stream.read(&mut request).expect("request bytes");
                stream
                    .write_all(response.as_bytes())
                    .expect("fixture response");
            }
        });
        let case = serde_json::from_value(serde_json::json!({
            "id":"response-control", "requirements":["fixture"],
            "safety":{"destructive":false,"requires_isolated_lrs":true},
            "request":request,"expect":expect
        }))
        .expect("fixture case");
        super::validate_case_steps("response-control", &case).expect("valid fixture definition");
        let report = super::execute_case(
            &reqwest::Client::new(),
            &url::Url::parse(&format!("http://{address}")).expect("fixture URL"),
            &BTreeMap::new(),
            &BTreeMap::new(),
            None,
            false,
            case,
        )
        .await;
        responder.join().expect("fixture thread");
        report
    }

    fn control_response(status: u16, headers: &str, body: &str, truncated: bool) -> String {
        let length = body.len() + usize::from(truncated);
        format!(
            "HTTP/1.1 {status} Response\r\n{headers}Content-Length: {length}\r\nConnection: close\r\n\r\n{body}"
        )
    }

    #[test]
    fn utc_offset_diagnostic_requires_a_clock() {
        for text in [
            "2026-01-01",
            "2026-01",
            "2026-01-01+09:00",
            "2026-01-01T+09",
            "2026-01-01 -01",
        ] {
            assert!(!super::assertions::has_explicit_non_utc_offset(text));
            let submitted = serde_json::json!({"actor":{"mbox":"mailto:fixture@example.org"},"verb":{"id":"urn:verb"},"object":{"id":"urn:activity"},"timestamp":"2026-01-01T00:00:00Z"});
            let mut returned = submitted.clone();
            returned["timestamp"] = text.into();
            for definition in [
                serde_json::json!({"kind":"json-time","pointer":"/timestamp","utc":true}),
                serde_json::json!({"kind":"json-preservation","expected":submitted}),
            ] {
                let check: super::Check =
                    serde_json::from_value(definition).expect("fixture check");
                let failures = check.evaluate(
                    200,
                    &reqwest::header::HeaderMap::new(),
                    returned.to_string().as_bytes(),
                    &BTreeMap::new(),
                );
                assert_eq!(failures.len(), 1, "{text}: {failures:?}");
                assert!(
                    failures[0].contains("requires explicit combined date and time"),
                    "{text}: {failures:?}"
                );
            }
        }
    }

    #[test]
    fn timestamp_preservation_keeps_known_defects_in_either_operand() {
        for unknown in ["2026-01-01T24:00:00Z", "2016-12-31T23:59:60Z"] {
            for invalid in [
                "2026-02-30T00:00:00Z",
                "2026-01-01T25:00:00Z",
                "2026-01-01T00:61:00Z",
                "2026-01-01T00:00:00+24:00",
                "2026-01-01",
                "invalid",
            ] {
                for (sent, got) in [(unknown, invalid), (invalid, unknown)] {
                    for definition in [
                        serde_json::json!({"kind":"json-time","pointer":"/timestamp","expected":sent,"utc":true}),
                        serde_json::json!({"kind":"json-preservation","expected":{"timestamp":sent}}),
                    ] {
                        let check: super::Check =
                            serde_json::from_value(definition).expect("fixture check");
                        let failures = check.evaluate(
                            200,
                            &reqwest::header::HeaderMap::new(),
                            serde_json::json!({"timestamp":got}).to_string().as_bytes(),
                            &BTreeMap::new(),
                        );
                        assert!(
                            failures
                                .iter()
                                .any(|error| !error.starts_with("ORACLE-UNAVAILABLE:")),
                            "{sent} / {got}: {failures:?}"
                        );
                    }
                }
            }
            for (sent, got) in [
                (unknown, "2026-01-02T00:00:00Z"),
                ("2026-01-02T00:00:00Z", unknown),
            ] {
                assert!(
                    super::assertions::timestamp_preserved(got, sent, true)
                        .expect_err("provider still required")
                        .starts_with("ORACLE-UNAVAILABLE:")
                );
            }
        }
    }

    #[tokio::test]
    async fn explicit_non_utc_survives_unavailable_instant_conversion() {
        use super::CaseOutcome::{Failed, NotRun};
        for clock in [
            "2026-01-01T24:00:00",
            "20260101T240000",
            "2016-12-31T23:59:60",
        ] {
            for offset in [
                "+09:00", "+0900", "+09", "-09:00", "-0900", "-09", "-00:00", "-0000", "-00",
                "+00:00", "+0000", "+00", "Z",
            ] {
                let timestamp = format!("{clock}{offset}");
                let expected_outcome = if offset.starts_with('-') || offset.contains('9') {
                    Failed
                } else {
                    NotRun
                };
                let submitted = serde_json::json!({"actor":{"mbox":"mailto:fixture@example.org"},"verb":{"id":"urn:verb"},"object":{"id":"urn:activity"},"timestamp":format!("{clock}Z")});
                let mut returned = submitted.clone();
                returned["timestamp"] = serde_json::json!(timestamp);
                for check in [
                    serde_json::json!({"kind":"json-time","pointer":"/timestamp","utc":true}),
                    serde_json::json!({"kind":"json-preservation","expected":submitted}),
                ] {
                    let report = response_control(
                        serde_json::json!({"method":"GET","path":"/fixture"}),
                        serde_json::json!({"status":200,"checks":[check]}),
                        vec![control_response(200, "", &returned.to_string(), false)],
                    )
                    .await;
                    assert_eq!(report.outcome, expected_outcome, "{timestamp}: {report:?}");
                    if expected_outcome == Failed {
                        assert!(
                            report
                                .failures
                                .iter()
                                .any(|failure| failure.contains("UTC")),
                            "{report:?}"
                        );
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn interrupted_header_time_keeps_presence_and_syntax_obligations() {
        use super::CaseOutcome::{Failed, NotRun};
        for context in ["single", "initial-page", "continuation-page", "pending"] {
            for optional in [false, true] {
                for header in [
                    "",
                    "Last-Modified: invalid\r\n",
                    "Last-Modified: Thu, 01 Jan 2026 00:00:00 GMT\r\n",
                ] {
                    let collection = context.ends_with("page");
                    let mut request = serde_json::json!({"method":"GET","path":"/statements"});
                    let mut check = serde_json::json!({"kind":"header-time","header":"Last-Modified","optional_if_empty":optional});
                    if collection {
                        request["collect_statement_pages"] = 3.into();
                        check["max_pointer"] = "/statements".into();
                    } else {
                        request["query"] = serde_json::json!({"statementId":"01000000-0000-4000-8000-000000000001"});
                        check["equals_pointer"] = "/stored".into();
                    }
                    let mut responses = Vec::new();
                    if context == "continuation-page" {
                        responses.push(control_response(
                            200,
                            "Last-Modified: Thu, 01 Jan 2026 00:00:00 GMT\r\n",
                            r#"{"statements":[{"stored":"2026-01-01T00:00:00Z"}],"more":"/next"}"#,
                            false,
                        ));
                    }
                    if context == "pending" {
                        request["await_retrieval_ms"] = 1.into();
                    }
                    responses.push(control_response(
                        if context == "pending" { 404 } else { 200 },
                        header,
                        "x",
                        true,
                    ));
                    let report = response_control(
                        request,
                        serde_json::json!({"status":200,"checks":[check]}),
                        responses,
                    )
                    .await;
                    let defective = context != "pending"
                        && (header.contains("invalid")
                            || (header.is_empty() && !(collection && optional)));
                    assert_eq!(
                        report.outcome,
                        if defective { Failed } else { NotRun },
                        "{context}, optional={optional}, header={header:?}: {report:?}"
                    );
                    assert_eq!(
                        report.failures.iter().any(|e| e.contains("Last-Modified")),
                        defective
                    );
                    assert!(
                        report
                            .failures
                            .iter()
                            .chain(&report.not_run_reasons)
                            .any(|e| e.contains("body did not complete"))
                    );
                    assert!(report.failures.iter().all(|e| !e.contains("requires JSON")));
                }
            }
        }
    }

    #[tokio::test]
    async fn collected_statement_order_spans_pages_and_incomplete_prefixes() {
        use super::CaseOutcome::{Failed, NotRun, Passed};
        for descending in [false, true] {
            let (first, last) = if descending {
                ("2026-01-01T00:00:02Z", "2026-01-01T00:00:01Z")
            } else {
                ("2026-01-01T00:00:01Z", "2026-01-01T00:00:02Z")
            };
            for (pages, bound, truncated, expected) in [
                (vec![vec![first], vec![last]], false, false, Passed),
                (vec![vec![last], vec![first]], false, false, Failed),
                (vec![vec![first], vec![first]], false, false, Passed),
                (
                    vec![
                        vec!["2026-01-01T00:00:00Z"],
                        vec!["2026-01-01T01:00:00+01:00"],
                    ],
                    false,
                    false,
                    Passed,
                ),
                (vec![vec![first], vec![], vec![last]], false, false, Passed),
                (vec![vec![last], vec![], vec![first]], false, false, Failed),
                (
                    vec![vec![first], vec!["2026-01-01T24:00:00Z"]],
                    false,
                    false,
                    NotRun,
                ),
                (
                    vec![vec![last], vec!["2026-01-01T24:00:00Z"], vec![first]],
                    false,
                    false,
                    Failed,
                ),
                (vec![vec![last], vec![first]], true, false, Failed),
                (vec![vec![first], vec![last]], true, false, NotRun),
                (vec![vec![last], vec![first]], false, true, Failed),
                (vec![vec![first], vec![last]], false, true, NotRun),
                (
                    vec![vec![last], vec![first], vec![], vec![first], vec![first]],
                    false,
                    false,
                    Failed,
                ),
                (
                    vec![vec![last], vec![first], vec![first]],
                    true,
                    false,
                    Failed,
                ),
                (
                    vec![vec![last], vec![first], vec![first]],
                    false,
                    true,
                    Failed,
                ),
            ] {
                let mut responses = Vec::new();
                for (index, page) in pages.iter().enumerate() {
                    let more = if index + 1 < pages.len() || bound || truncated {
                        format!("/next/{}", index + 1)
                    } else {
                        String::new()
                    };
                    let statements: Vec<_> = page.iter().enumerate().map(|(item, stored)| {
                        serde_json::json!({"id":format!("{index}-{item}"),"stored":stored})
                    }).collect();
                    let body = serde_json::json!({"statements":statements,"more":more}).to_string();
                    responses.push(control_response(200, "", &body, false));
                }
                if truncated {
                    responses.push(control_response(200, "", "x", true));
                }
                let limit = if bound { pages.len() } else { pages.len() + 1 };
                let request = serde_json::json!({"method":"GET","path":"/statements","collect_statement_pages":limit});
                let expect = serde_json::json!({"status":200,
                    "json":{"pointers":{"/statements":{"contains":{"id":"0-0"}}}},
                    "checks":[{"kind":"json-order","pointer":"/statements","by":"stored","descending":descending}]});
                let report = response_control(request, expect, responses).await;
                assert_eq!(
                    report.outcome, expected,
                    "descending={descending}, {pages:?}, bound={bound}, truncated={truncated}: {report:?}"
                );
                assert_eq!(
                    report
                        .failures
                        .iter()
                        .filter(|e| e.contains("not ordered by stored"))
                        .count(),
                    usize::from(expected == Failed)
                );
            }
        }
    }

    #[tokio::test]
    async fn collected_order_keeps_distinct_later_defects() {
        let pages = [
            serde_json::json!([{"stored":"2026-01-01T00:00:01Z"}]),
            serde_json::json!([{"stored":"2026-01-01T24:00:00Z"}]),
            serde_json::json!([{"stored":"2026-01-01T00:00:02Z"}]),
            serde_json::json!([{"stored":42}]),
            serde_json::json!([]),
        ];
        let responses = pages
            .iter()
            .enumerate()
            .map(|(index, items)| {
                let more = if index + 1 == pages.len() {
                    String::new()
                } else {
                    format!("/next/{index}")
                };
                control_response(
                    200,
                    "",
                    &serde_json::json!({"statements":items,"more":more}).to_string(),
                    false,
                )
            })
            .collect();
        let report = response_control(
            serde_json::json!({"method":"GET","path":"/statements","collect_statement_pages":5}),
            serde_json::json!({"status":200,"checks":[{"kind":"json-order","pointer":"/statements","by":"stored","descending":true}]}),
            responses,
        ).await;
        assert_eq!(report.outcome, super::CaseOutcome::Failed);
        for reason in [
            "ORACLE-UNAVAILABLE:",
            "not ordered by stored",
            "missing JSON timestamp string",
        ] {
            assert_eq!(
                report
                    .failures
                    .iter()
                    .filter(|e| e.contains(reason))
                    .count(),
                1,
                "{report:?}"
            );
        }
    }

    #[test]
    fn definite_timestamp_defects_fail_even_on_expected_error_responses()
    -> Result<(), serde_yaml::Error> {
        let expectation: super::ResponseExpectation = serde_yaml::from_str(
            "status: 400\nheaders:\n  X-Experience-API-Consistent-Through: {iso8601: true}",
        )?;
        for timestamp in [
            "2026-02-30T00:00:00Z",
            "2025-02-29T00:00:00Z",
            "2026-00-01T00:00:00Z",
            "2026-01-00T00:00:00Z",
            "2026-10-01T25:00:00Z",
            "2026-10-01T24:00:01Z",
            "2026-10-01T24:00:00.1Z",
            "2026-10-01T00:61:00Z",
            "2026-10-01T00:00:61Z",
            "2026-10-01T00:00:00+24:00",
            "2026-10-01T00:00:00+09:60",
            // Known violations must take precedence over leap-second delegation.
            "2026-02-30T23:59:60Z",
            "2026-06-30T25:59:60Z",
            "2026-06-30T23:59:60+09:60",
            "20260230T000000Z",
            "2026-10-01T25Z",
            "2026-10-01T25:00Z",
            "2026-10-01T00:61Z",
            "2026-10-01T25.5Z",
            "2026-10-01T24.1Z",
            "2026-10-01T09.5+24:00",
        ] {
            let mut failures =
                super::evaluate_status(&expectation, reqwest::StatusCode::BAD_REQUEST);
            assert!(failures.is_empty());
            super::evaluate_header(
                "X-Experience-API-Consistent-Through",
                &expectation.headers["X-Experience-API-Consistent-Through"],
                Some(timestamp),
                &BTreeMap::new(),
                &mut failures,
            );
            assert!(
                failures
                    .iter()
                    .any(|error| !error.starts_with("ORACLE-UNAVAILABLE:")),
                "invalid watermark must be a definite failure: {timestamp}"
            );
        }
        for timestamp in ["2024-02-29T00:00:00Z", "2026-10-01T23:59:59.123456789012Z"] {
            assert!(
                super::assertions::parse_time(timestamp).is_ok(),
                "{timestamp}"
            );
        }
        for timestamp in [
            "2026-10-01T24:00:00Z",
            "2026-06-30T23:59:60Z",
            "2026-10-01T09.5Z",
            "2026-10-01T09:30.5Z",
        ] {
            assert!(
                matches!(super::assertions::parse_time(timestamp), Err(error)
                if error.starts_with("ORACLE-UNAVAILABLE:")),
                "{timestamp}"
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn observation_deadline_is_incomplete_but_error_header_defects_fail() {
        use std::io::{Read, Write};
        for valid_headers in [true, false] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("local fixture");
            let address = listener.local_addr().expect("fixture address");
            let responder = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().expect("fixture request");
                let mut request = [0_u8; 4096];
                let _ = stream.read(&mut request).expect("request bytes");
                std::thread::sleep(std::time::Duration::from_millis(10));
                let version = if valid_headers {
                    "X-Experience-API-Version: 2.0.0\r\n"
                } else {
                    ""
                };
                write!(stream, "HTTP/1.1 404 Not Found\r\n{version}X-Experience-API-Consistent-Through: 2020-01-01T00:00:00Z\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").expect("fixture response");
            });
            let case = serde_yaml::from_str(
                r"
id: delayed-retrieval
requirements: [xapi-lrs-4.1.6.1-statement-put]
safety: {destructive: false, requires_isolated_lrs: false}
request:
  method: GET
  path: /statements
  query: {statementId: '01000000-0000-4000-8000-000000000001'}
  await_retrieval_ms: 1
expect:
  status: 200
  headers:
    X-Experience-API-Version: {equals: '2.0.0'}
    X-Experience-API-Consistent-Through: {iso8601: true}
  json: {pointers: {'/id': {present: true}}}
",
            )
            .expect("fixture case");
            let report = super::execute_case(
                &reqwest::Client::new(),
                &url::Url::parse(&format!("http://{address}")).expect("fixture URL"),
                &BTreeMap::new(),
                &BTreeMap::new(),
                None,
                false,
                case,
            )
            .await;
            assert_eq!(
                report.outcome,
                if valid_headers {
                    super::CaseOutcome::NotRun
                } else {
                    super::CaseOutcome::Failed
                }
            );
            responder.join().expect("fixture thread");
        }
    }
    #[test]
    fn media_type_assertions_reject_substrings_and_accept_http_case_rules() {
        for actual in [
            "application/json",
            "APPLICATION/JSON",
            "application/json; charset=UTF-8",
        ] {
            assert!(super::header_contains(
                "Content-Type",
                actual,
                "application/json"
            ));
        }
        for actual in [
            "application/jsonp",
            "application/json+other",
            "text/plain; note=application/json",
            "application/json; broken",
            "not application/json",
        ] {
            assert!(!super::header_contains(
                "Content-Type",
                actual,
                "application/json"
            ));
        }
        assert!(super::header_contains(
            "content-type",
            "text/plain; charset=UTF-8",
            "TEXT/PLAIN; charset=utf-8"
        ));
        assert!(!super::header_contains(
            "content-type",
            "text/plain; charset=iso-8859-1",
            "text/plain; charset=utf-8"
        ));
        assert!(super::header_contains("x-custom", "prefix-value", "value"));
    }

    #[test]
    fn content_type_linear_whitespace_preserves_tokens_and_quoted_values() {
        for (actual, expected) in [
            ("application/json ; charset=utf-8", "application/json"),
            (
                "application/json\t;\tcharset=UTF-8",
                "application/json; charset=utf-8",
            ),
            (
                " APPLICATION/JSON ; charset=\"utf-8\" ",
                "application/json ; charset=utf-8",
            ),
            (
                "multipart/mixed ; boundary=\"a ; b\"",
                "multipart/mixed; boundary=\"a ; b\"",
            ),
            (
                "application/octet-stream ; name=\"a b\"",
                "application/octet-stream",
            ),
        ] {
            assert!(
                super::header_contains("Content-Type", actual, expected),
                "{actual}"
            );
        }
        for (actual, expected) in [
            ("application /json", "application/json"),
            ("application/ json", "application/json"),
            ("application/json; charset =utf-8", "application/json"),
            ("application/json; charset= utf-8", "application/json"),
            ("app lication/json ; charset=utf-8", "application/json"),
            ("application/j son", "application/json"),
            ("application/json; char set=utf-8", "application/json"),
            ("application/json; charset=utf -8", "application/json"),
            ("application/json; broken", "application/json"),
            ("application/json (comment)", "application/json"),
            (
                "multipart/mixed; boundary=\"a ; b\"",
                "multipart/mixed; boundary=\"a;b\"",
            ),
            (
                "multipart/mixed; boundary=\"AbC\"",
                "multipart/mixed; boundary=\"abc\"",
            ),
        ] {
            assert!(
                !super::header_contains("Content-Type", actual, expected),
                "{actual}"
            );
        }
    }

    #[tokio::test]
    async fn content_type_quoted_parameters_follow_http_grammar() {
        for (actual, expected, valid) in [
            (r#"application/json; note="a\"b""#, "application/json", true),
            (r#"application/json; note="""#, "application/json", true),
            (
                r#"application/json; note="""#,
                r#"application/json; note="""#,
                true,
            ),
            (
                r#"application/json; note="a\"b""#,
                r#"application/json; note="a\"b""#,
                true,
            ),
            (
                r#"multipart/mixed; boundary="a\b""#,
                "multipart/mixed; boundary=ab",
                true,
            ),
            (
                r#"application/json; note="a\\b""#,
                r#"application/json; note="a\\b""#,
                true,
            ),
            (
                r#"application/json; note="a;=b""#,
                r#"application/json; note="a;=b""#,
                true,
            ),
            (
                r#"application/json; charset="UT\F-8""#,
                "application/json; charset=utf-8",
                true,
            ),
            (
                r#"application/json; note="a\"b""#,
                r#"application/json; note="ab""#,
                false,
            ),
            (
                r#"multipart/mixed; boundary="A\b""#,
                "multipart/mixed; boundary=ab",
                false,
            ),
            (r#"application/json; note=""x"#, "application/json", false),
            (r#"application/json; note="a\""#, "application/json", false),
            (r#"application/json; note="a\"#, "application/json", false),
            (r#"application/json; note="a"b""#, "application/json", false),
            ("application/json; note=", "application/json", false),
            ("application/json; note={x}", "application/json", false),
            (
                "application/json; note=x(comment)",
                "application/json",
                false,
            ),
        ] {
            let report = response_control(
                serde_json::json!({"method":"GET","path":"/statements"}),
                serde_json::json!({"status":200,"headers":{"Content-Type":{"contains":expected}}}),
                vec![control_response(
                    200,
                    &format!("Content-Type: {actual}\r\n"),
                    "{}",
                    false,
                )],
            )
            .await;
            assert_eq!(
                report.outcome,
                if valid {
                    super::CaseOutcome::Passed
                } else {
                    super::CaseOutcome::Failed
                },
                "{actual} / {expected}: {report:?}"
            );
        }
    }

    #[tokio::test]
    #[expect(
        clippy::too_many_lines,
        reason = "Keep complete/interrupted response pairs in one visible classification matrix"
    )]
    async fn retrieval_preserves_visibility_and_header_classification() {
        use super::CaseOutcome::{Failed, NotRun};
        use std::io::{Read, Write};
        // Initial delay, error headers, ordinary 404/500, and previously visible
        // Statements must remain distinguishable when the body is truncated.
        for ((status, wait, seen, version, through, outcome), truncated) in [
            (404, true, false, "2.0.0", "2026-01-02T00:00:00Z", NotRun),
            (404, true, false, "", "2026-01-02T00:00:00Z", Failed),
            (404, true, false, "2.0.0", "2026-02-30T00:00:00Z", Failed),
            (404, false, false, "2.0.0", "2026-01-02T00:00:00Z", Failed),
            (500, true, false, "2.0.0", "2026-01-02T00:00:00Z", Failed),
            (200, true, false, "2.0.0", "2026-01-02T00:00:00Z", NotRun),
            (404, true, true, "2.0.0", "2026-01-02T00:00:00Z", Failed),
            (404, true, true, "2.0.0", "2025-12-31T00:00:00Z", NotRun),
            (404, true, true, "2.0.0", "2026-01-01T00:00:00Z", NotRun),
            (404, true, true, "2.0.0", "2026-01-01T00:00:00.000Z", NotRun),
            (
                404,
                true,
                true,
                "2.0.0",
                "2026-01-01T09:00:00+09:00",
                NotRun,
            ),
            (
                404,
                true,
                true,
                "2.0.0",
                "2026-01-01T00:00:00.0000000001Z",
                Failed,
            ),
        ]
        .into_iter()
        .flat_map(|scenario| [true, false].map(|truncated| (scenario, truncated)))
        .filter(|(scenario, truncated)| *truncated || scenario.0 == 404)
        {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("local fixture");
            let address = listener.local_addr().expect("fixture address");
            let responder = std::thread::spawn(move || {
                for first in (0..=usize::from(seen)).map(|i| seen && i == 0) {
                    let (mut stream, _) = listener.accept().expect("fixture request");
                    let mut request = [0_u8; 4096];
                    let _ = stream.read(&mut request).expect("request bytes");
                    let body = r#"{"id":"01000000-0000-4000-8000-000000000001","stored":"2026-01-01T00:00:00Z"}"#;
                    if first {
                        write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).expect("visible fixture");
                    } else {
                        // Exceed the 1ms observation window without a retry race.
                        std::thread::sleep(std::time::Duration::from_millis(10));
                        let length = if truncated { 100 } else { 1 };
                        write!(stream, "HTTP/1.1 {status} Response\r\nX-Experience-API-Version: {version}\r\nX-Experience-API-Consistent-Through: {through}\r\nContent-Length: {length}\r\nConnection: close\r\n\r\nx").expect("retrieval fixture");
                    }
                }
            });
            let request = serde_json::json!({"method":"GET", "path":"/statements", "query":{"statementId":"01000000-0000-4000-8000-000000000001"}});
            let mut read_request = request.clone();
            if wait {
                read_request["await_retrieval_ms"] = 1.into();
            }
            let mut steps = Vec::new();
            if seen {
                steps.push(
                    serde_json::json!({"id":"visible", "request":request, "expect":{"status":200}}),
                );
            }
            steps.push(serde_json::json!({"id":"read", "request":read_request, "expect":{"status":200,"headers":{
                "X-Experience-API-Version":{"equals":"2.0.0"},
                "X-Experience-API-Consistent-Through":{"iso8601":true},
                "Content-Type":{"contains":"application/json"},
                "Last-Modified":{"present":true}
            }}}));
            // Success-only headers must not be required on the initial 404.
            if status == 200 {
                steps.last_mut().expect("read step")["expect"]["headers"]
                    .as_object_mut()
                    .expect("headers")
                    .retain(|name, _| name.starts_with("X-Experience"));
            }
            let case = serde_json::from_value(serde_json::json!({"id":"interrupted-retrieval", "requirements":["xapi-lrs-4.1.6.1-statement-put"],"safety":{"destructive":false,"requires_isolated_lrs":false},"steps":steps})).expect("fixture case");
            let report = super::execute_case(
                &reqwest::Client::new(),
                &url::Url::parse(&format!("http://{address}")).expect("fixture URL"),
                &BTreeMap::new(),
                &BTreeMap::new(),
                None,
                false,
                case,
            )
            .await;
            responder.join().expect("fixture thread");
            assert_eq!(
                report.outcome, outcome,
                "status={status}, wait={wait}, seen={seen}, version={version}, through={through}: {report:?}"
            );
            if wait && status == 404 {
                assert!(report.failures.iter().all(|reason| {
                    !reason.contains("expected status")
                        && !reason.contains("Content-Type")
                        && !reason.contains("Last-Modified")
                }));
            }
            assert_eq!(
                report
                    .failures
                    .iter()
                    .chain(&report.not_run_reasons)
                    .any(|reason| reason.contains("response body did not complete")),
                truncated
            );
        }
    }

    #[test]
    fn json_oracles_reject_invalid_utf8_without_rejecting_binary_documents() {
        let expectation: super::ResponseExpectation = serde_yaml::from_str(
            "status: 200\njson: {pointers: {'/version/0': {equals: '2.0.0'}}}\ncapture: {text: {pointer: /extra}}",
        ).expect("JSON expectation");
        let valid = b"{\"version\":[\"2.0.0\"],\"extra\":\"valid\"}";
        let invalid = b"{\"version\":[\"2.0.0\"],\"extra\":\"\xff\"}";
        assert!(super::evaluate_body(&expectation, valid, &mut BTreeMap::new()).is_empty());
        let mut captures = BTreeMap::new();
        assert!(!super::evaluate_body(&expectation, invalid, &mut captures).is_empty());
        assert!(captures.is_empty());
        let binary: super::ResponseExpectation =
            serde_yaml::from_str("status: 200\nbody: {non_empty: true}")
                .expect("binary expectation");
        assert!(super::evaluate_body(&binary, &[0, 255, 128], &mut BTreeMap::new()).is_empty());
    }
    #[tokio::test]
    async fn unavailable_target_fixtures_are_not_run_and_never_pass() {
        let document: super::SuiteDocument = serde_yaml::from_str(
            r"
suite:
  id: unavailable-fixtures
  standard: IEEE/ISO/IEC 39274-1-1-2025
cases:
- id: missing-size-limit
  requirements: [xapi-lrs-4.1.5-request-too-large]
  safety: {destructive: false, requires_isolated_lrs: true}
  requires_endpoint_profiles: [deterministic-too-large-threshold]
  request: {method: POST, path: /statements}
  expect: {status: 413}
- id: missing-refused-credential
  requirements: [xapi-lrs-4.1.5-unauthorized]
  safety: {destructive: false, requires_isolated_lrs: false}
  steps:
  - id: first
    request: {method: GET, path: /statements, auth_profile: invalid_credential}
    expect: {status: 401}
  - id: second
    request: {method: GET, path: /statements, auth_profile: invalid_credential}
    expect: {status: 401}
",
        )
        .expect("fixture case YAML");
        let report = super::execute_suites(
            vec![super::LoadedSuite {
                path: "fixture.yaml".into(),
                document,
            }],
            &url::Url::parse("http://127.0.0.1:9").expect("URL"),
            &std::collections::BTreeMap::new(),
            &std::collections::BTreeMap::new(),
            None,
            false,
        )
        .await
        .expect("no HTTP requests are eligible");
        assert_eq!(report.status, super::ReportStatus::Incomplete);
        assert!(!report.is_success());
        assert_eq!(report.totals.not_run, 2);
        assert_eq!(report.totals.failed, 0);
        assert_eq!(report.totals.passed, 0);
        assert_eq!(
            report.suites[0].cases[1].not_run_reasons[0]
                .matches("auth:invalid_credential")
                .count(),
            1
        );
        for case in &report.suites[0].cases {
            assert_eq!(case.outcome, super::CaseOutcome::NotRun);
            assert!(case.failures.is_empty());
            assert!(!case.not_run_reasons.is_empty());
        }
    }

    #[test]
    fn conditional_steps_require_a_reachable_previous_response() {
        let document: super::SuiteDocument = serde_yaml::from_str(
            r"
suite: {id: conditional, standard: IEEE/ISO/IEC 39274-1-1-2025}
cases:
- id: conditional-read
  requirements: [xapi-lrs-4.2.7-timestamps-utc]
  safety: {destructive: true, requires_isolated_lrs: true}
  steps:
  - id: write
    request: {method: POST, path: /statements}
    expect: {status: 200, status_in: [400]}
  - id: read
    when_previous_status: 200
    request: {method: GET, path: /statements}
    expect: {status: 200}
",
        )
        .expect("conditional fixture");
        let mut case = document.cases.into_iter().next().expect("fixture case");
        assert!(super::validate_case_steps("conditional", &case).is_ok());
        case.steps[1].when_previous_status = Some(204);
        assert!(super::validate_case_steps("unreachable", &case).is_err());
        case.steps[1].when_previous_status = Some(200);
        case.steps[0].when_previous_status = Some(200);
        assert!(super::validate_case_steps("missing previous", &case).is_err());
    }

    #[test]
    fn diagnostic_status_does_not_exempt_independent_headers() {
        let mut expectation: super::ResponseExpectation = serde_yaml::from_str(
            r"
status: 204
status_classification: diagnostic
headers:
  X-Experience-API-Version: {equals: 2.0.0}
",
        )
        .expect("leaf-level diagnostic expectation");
        assert!(super::evaluate_status(&expectation, reqwest::StatusCode::BAD_REQUEST).is_empty());
        assert!(
            !super::evaluate_headers(
                &expectation,
                &reqwest::header::HeaderMap::new(),
                &std::collections::BTreeMap::new()
            )
            .is_empty()
        );
        expectation.status_classification = super::StatusClassification::Required;
        assert!(!super::evaluate_status(&expectation, reqwest::StatusCode::BAD_REQUEST).is_empty());
    }
    use std::collections::BTreeMap;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    use serde_json::json;

    use super::{CaseSafety, LoadedSuite, RunOptions, SuiteDocument, run, validate_suites};

    #[test]
    fn explicit_json_null_is_preserved_in_requests_and_expectations() -> Result<(), super::Error> {
        let request: super::RequestSpec = serde_yaml::from_str(
            "method: POST\npath: /activities/state\nbody: null",
        )
        .map_err(|source| super::Error::ParseYaml {
            path: PathBuf::from("null-fixture"),
            source,
        })?;
        assert_eq!(request.body, Some(serde_json::Value::Null));
        let expectation: super::ResponseExpectation =
            serde_yaml::from_str("status: 200\njson:\n  pointers:\n    '': {equals: null}")
                .map_err(|source| super::Error::ParseYaml {
                    path: PathBuf::from("null-fixture"),
                    source,
                })?;
        super::validate_expectation("null", &expectation, &BTreeSet::new())?;
        assert!(super::evaluate_body(&expectation, b"null", &mut BTreeMap::new()).is_empty());
        assert!(!super::evaluate_body(&expectation, b"{}", &mut BTreeMap::new()).is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn validates_bundled_http_cases() -> Result<(), super::Error> {
        let case_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/http");
        let report = run(RunOptions {
            case_roots: vec![case_path],
            endpoint: None,
            common_headers: BTreeMap::default(),
            auth_profiles: BTreeMap::default(),
            endpoint_profile: None,
            validate_only: true,
            allow_destructive: false,
            filter: super::RunFilter::default(),
        })
        .await?;

        assert!(report.is_success());
        assert!(report.totals.total >= 127);
        assert_eq!(report.totals.total, report.totals.validated);
        Ok(())
    }

    #[tokio::test]
    async fn filters_bundled_cases_by_requirement() -> Result<(), super::Error> {
        let case_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/http");
        let report = run(RunOptions {
            case_roots: vec![case_path],
            endpoint: None,
            common_headers: BTreeMap::default(),
            auth_profiles: BTreeMap::default(),
            endpoint_profile: None,
            validate_only: true,
            allow_destructive: false,
            filter: super::RunFilter {
                suites: BTreeSet::default(),
                cases: BTreeSet::default(),
                requirements: BTreeSet::from(["xapi-lrs-4.1.6.7-about-version".to_owned()]),
            },
        })
        .await?;

        assert!(report.is_success());
        assert!(report.totals.total >= 2);
        assert!(
            report
                .suites
                .iter()
                .all(|suite| suite.cases.iter().all(|case| case
                    .requirements
                    .contains(&"xapi-lrs-4.1.6.7-about-version".to_owned())))
        );
        Ok(())
    }

    #[tokio::test]
    async fn rejects_filters_that_select_no_cases() {
        let case_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/http");
        let result = run(RunOptions {
            case_roots: vec![case_path],
            endpoint: None,
            common_headers: BTreeMap::default(),
            auth_profiles: BTreeMap::default(),
            endpoint_profile: None,
            validate_only: true,
            allow_destructive: false,
            filter: super::RunFilter {
                suites: BTreeSet::default(),
                cases: BTreeSet::from(["does-not-exist".to_owned()]),
                requirements: BTreeSet::default(),
            },
        })
        .await;

        assert!(result.is_err());
    }

    #[test]
    fn rejects_destructive_cases_without_isolation() {
        let document = SuiteDocument {
            suite: super::SuiteMetadata {
                id: "inline".to_owned(),
                standard: super::STANDARD.to_owned(),
                description: None,
            },
            cases: vec![super::TestCase {
                id: "destructive-without-isolation".to_owned(),
                requirements: vec!["xapi-lrs-inline".to_owned()],
                safety: CaseSafety {
                    destructive: true,
                    requires_isolated_lrs: false,
                },
                requires_endpoint_profiles: Vec::new(),
                request: Some(super::RequestSpec {
                    auth_profile: None,
                    method: "GET".to_owned(),
                    path: "/about".to_owned(),
                    follow_more: None,
                    collect_statement_pages: None,
                    query: BTreeMap::default(),
                    headers: BTreeMap::default(),
                    body: None,
                    raw_body: None,
                    raw_body_hex: None,
                    await_retrieval_ms: None,
                    unrecognized_parameter: None,
                    raw_body_crlf_lines: None,
                    repeat: None,
                }),
                expect: Some(super::ResponseExpectation {
                    status: 200,
                    status_classification: super::StatusClassification::Required,
                    status_in: Vec::new(),
                    checks: Vec::new(),
                    headers: BTreeMap::default(),
                    body: None,
                    json: None,
                    capture: BTreeMap::default(),
                }),
                steps: Vec::new(),
            }],
        };
        let suites = [LoadedSuite {
            path: PathBuf::from("inline.yaml"),
            document,
        }];

        assert!(validate_suites(&suites).is_err());
    }

    #[test]
    fn accepts_auth_profile_names_on_requests() {
        let document = SuiteDocument {
            suite: super::SuiteMetadata {
                id: "inline".to_owned(),
                standard: super::STANDARD.to_owned(),
                description: None,
            },
            cases: vec![super::TestCase {
                id: "auth-profile-reference".to_owned(),
                requirements: vec!["xapi-lrs-inline".to_owned()],
                safety: CaseSafety {
                    destructive: false,
                    requires_isolated_lrs: false,
                },
                requires_endpoint_profiles: Vec::new(),
                request: Some(super::RequestSpec {
                    auth_profile: Some("statement_writer".to_owned()),
                    method: "GET".to_owned(),
                    path: "/about".to_owned(),
                    follow_more: None,
                    collect_statement_pages: None,
                    query: BTreeMap::default(),
                    headers: BTreeMap::default(),
                    body: None,
                    raw_body: None,
                    raw_body_hex: None,
                    await_retrieval_ms: None,
                    unrecognized_parameter: None,
                    raw_body_crlf_lines: None,
                    repeat: None,
                }),
                expect: Some(super::ResponseExpectation {
                    status: 200,
                    status_classification: super::StatusClassification::Required,
                    status_in: Vec::new(),
                    checks: Vec::new(),
                    headers: BTreeMap::default(),
                    body: None,
                    json: None,
                    capture: BTreeMap::default(),
                }),
                steps: Vec::new(),
            }],
        };
        let suites = [LoadedSuite {
            path: PathBuf::from("inline.yaml"),
            document,
        }];

        assert!(validate_suites(&suites).is_ok());
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "The two-step fixture makes the previous-response capture dependency explicit"
    )]
    fn accepts_capture_reference_from_previous_step() {
        let mut capture = BTreeMap::new();
        capture.insert(
            "captured_version".to_owned(),
            super::CaptureSpec {
                nonmatching_etag: false,
                as_json: false,
                pointer: Some("/version".to_owned()),
                header: None,
            },
        );

        let mut query = BTreeMap::new();
        query.insert("version".to_owned(), "{{captured_version}}".to_owned());

        let document = SuiteDocument {
            suite: super::SuiteMetadata {
                id: "inline".to_owned(),
                standard: super::STANDARD.to_owned(),
                description: None,
            },
            cases: vec![super::TestCase {
                id: "capture-reference".to_owned(),
                requirements: vec!["xapi-lrs-inline".to_owned()],
                safety: CaseSafety {
                    destructive: false,
                    requires_isolated_lrs: false,
                },
                requires_endpoint_profiles: Vec::new(),
                request: None,
                expect: None,
                steps: vec![
                    super::TestStep {
                        when_previous_status: None,
                        when_step_status: None,
                        id: "capture".to_owned(),
                        request: super::RequestSpec {
                            auth_profile: None,
                            method: "GET".to_owned(),
                            path: "/about".to_owned(),
                            follow_more: None,
                            collect_statement_pages: None,
                            query: BTreeMap::default(),
                            headers: BTreeMap::default(),
                            body: None,
                            raw_body: None,
                            raw_body_hex: None,
                            await_retrieval_ms: None,
                            unrecognized_parameter: None,
                            raw_body_crlf_lines: None,
                            repeat: None,
                        },
                        expect: super::ResponseExpectation {
                            status: 200,
                            status_classification: super::StatusClassification::Required,
                            status_in: Vec::new(),
                            checks: Vec::new(),
                            headers: BTreeMap::default(),
                            body: None,
                            json: None,
                            capture,
                        },
                    },
                    super::TestStep {
                        when_previous_status: None,
                        when_step_status: None,
                        id: "use-capture".to_owned(),
                        request: super::RequestSpec {
                            auth_profile: None,
                            method: "GET".to_owned(),
                            path: "/about".to_owned(),
                            follow_more: None,
                            collect_statement_pages: None,
                            query,
                            headers: BTreeMap::default(),
                            body: None,
                            raw_body: None,
                            raw_body_hex: None,
                            await_retrieval_ms: None,
                            unrecognized_parameter: None,
                            raw_body_crlf_lines: None,
                            repeat: None,
                        },
                        expect: super::ResponseExpectation {
                            status: 200,
                            status_classification: super::StatusClassification::Required,
                            status_in: Vec::new(),
                            checks: Vec::new(),
                            headers: BTreeMap::default(),
                            body: Some(super::BodyExpectation {
                                non_empty: None,
                                contains: Some("{{captured_version}}".to_owned()),
                            }),
                            json: None,
                            capture: BTreeMap::default(),
                        },
                    },
                ],
            }],
        };
        let suites = [LoadedSuite {
            path: PathBuf::from("inline.yaml"),
            document,
        }];

        assert!(validate_suites(&suites).is_ok());
    }

    #[test]
    fn accepts_header_capture_reference_from_previous_step() {
        let mut capture = BTreeMap::new();
        capture.insert(
            "captured_etag".to_owned(),
            super::CaptureSpec {
                nonmatching_etag: false,
                as_json: false,
                pointer: None,
                header: Some("ETag".to_owned()),
            },
        );

        let mut headers = BTreeMap::new();
        headers.insert("If-Match".to_owned(), "{{captured_etag}}".to_owned());

        let document = SuiteDocument {
            suite: super::SuiteMetadata {
                id: "inline".to_owned(),
                standard: super::STANDARD.to_owned(),
                description: None,
            },
            cases: vec![super::TestCase {
                id: "header-capture-reference".to_owned(),
                requirements: vec!["xapi-lrs-inline".to_owned()],
                safety: CaseSafety {
                    destructive: false,
                    requires_isolated_lrs: false,
                },
                requires_endpoint_profiles: Vec::new(),
                request: None,
                expect: None,
                steps: vec![
                    super::TestStep {
                        when_previous_status: None,
                        when_step_status: None,
                        id: "capture".to_owned(),
                        request: super::RequestSpec {
                            auth_profile: None,
                            method: "GET".to_owned(),
                            path: "/activities/state".to_owned(),
                            follow_more: None,
                            collect_statement_pages: None,
                            query: BTreeMap::default(),
                            headers: BTreeMap::default(),
                            body: None,
                            raw_body: None,
                            raw_body_hex: None,
                            await_retrieval_ms: None,
                            unrecognized_parameter: None,
                            raw_body_crlf_lines: None,
                            repeat: None,
                        },
                        expect: super::ResponseExpectation {
                            status: 200,
                            status_classification: super::StatusClassification::Required,
                            status_in: Vec::new(),
                            checks: Vec::new(),
                            headers: BTreeMap::default(),
                            body: None,
                            json: None,
                            capture,
                        },
                    },
                    super::TestStep {
                        when_previous_status: None,
                        when_step_status: None,
                        id: "use-capture".to_owned(),
                        request: super::RequestSpec {
                            auth_profile: None,
                            method: "PUT".to_owned(),
                            path: "/activities/state".to_owned(),
                            follow_more: None,
                            collect_statement_pages: None,
                            query: BTreeMap::default(),
                            headers,
                            body: None,
                            raw_body: None,
                            raw_body_hex: None,
                            await_retrieval_ms: None,
                            unrecognized_parameter: None,
                            raw_body_crlf_lines: None,
                            repeat: None,
                        },
                        expect: super::ResponseExpectation {
                            status: 204,
                            status_classification: super::StatusClassification::Required,
                            status_in: Vec::new(),
                            checks: Vec::new(),
                            headers: BTreeMap::default(),
                            body: None,
                            json: None,
                            capture: BTreeMap::default(),
                        },
                    },
                ],
            }],
        };
        let suites = [LoadedSuite {
            path: PathBuf::from("inline.yaml"),
            document,
        }];

        assert!(validate_suites(&suites).is_ok());
    }

    #[test]
    fn rejects_forward_capture_reference() {
        let mut query = BTreeMap::new();
        query.insert("version".to_owned(), "{{later_version}}".to_owned());

        let document = SuiteDocument {
            suite: super::SuiteMetadata {
                id: "inline".to_owned(),
                standard: super::STANDARD.to_owned(),
                description: None,
            },
            cases: vec![super::TestCase {
                id: "forward-reference".to_owned(),
                requirements: vec!["xapi-lrs-inline".to_owned()],
                safety: CaseSafety {
                    destructive: false,
                    requires_isolated_lrs: false,
                },
                requires_endpoint_profiles: Vec::new(),
                request: Some(super::RequestSpec {
                    auth_profile: None,
                    method: "GET".to_owned(),
                    path: "/about".to_owned(),
                    follow_more: None,
                    collect_statement_pages: None,
                    query,
                    headers: BTreeMap::default(),
                    body: None,
                    raw_body: None,
                    raw_body_hex: None,
                    await_retrieval_ms: None,
                    unrecognized_parameter: None,
                    raw_body_crlf_lines: None,
                    repeat: None,
                }),
                expect: Some(super::ResponseExpectation {
                    status: 200,
                    status_classification: super::StatusClassification::Required,
                    status_in: Vec::new(),
                    checks: Vec::new(),
                    headers: BTreeMap::default(),
                    body: None,
                    json: None,
                    capture: BTreeMap::default(),
                }),
                steps: Vec::new(),
            }],
        };
        let suites = [LoadedSuite {
            path: PathBuf::from("inline.yaml"),
            document,
        }];

        assert!(validate_suites(&suites).is_err());
    }

    #[test]
    fn accepts_endpoint_profile_capture_when_profile_is_required() {
        let document = SuiteDocument {
            suite: super::SuiteMetadata {
                id: "inline".to_owned(),
                standard: super::STANDARD.to_owned(),
                description: None,
            },
            cases: vec![super::TestCase {
                id: "endpoint-profile-capture".to_owned(),
                requirements: vec!["xapi-lrs-inline".to_owned()],
                safety: CaseSafety {
                    destructive: false,
                    requires_isolated_lrs: true,
                },
                requires_endpoint_profiles: vec![super::ENDPOINT_PROFILE_TOO_LARGE.to_owned()],
                request: Some(super::RequestSpec {
                    auth_profile: None,
                    method: "PUT".to_owned(),
                    path: "/statements".to_owned(),
                    follow_more: None,
                    collect_statement_pages: None,
                    query: BTreeMap::default(),
                    headers: BTreeMap::default(),
                    body: None,
                    raw_body: Some(format!(
                        "{{{{{}}}}}",
                        super::ENDPOINT_CAPTURE_TOO_LARGE_BODY
                    )),
                    raw_body_hex: None,
                    await_retrieval_ms: None,
                    unrecognized_parameter: None,
                    raw_body_crlf_lines: None,
                    repeat: None,
                }),
                expect: Some(super::ResponseExpectation {
                    status: 413,
                    status_classification: super::StatusClassification::Required,
                    status_in: Vec::new(),
                    checks: Vec::new(),
                    headers: BTreeMap::default(),
                    body: None,
                    json: None,
                    capture: BTreeMap::default(),
                }),
                steps: Vec::new(),
            }],
        };
        let suites = [LoadedSuite {
            path: PathBuf::from("inline.yaml"),
            document,
        }];

        assert!(validate_suites(&suites).is_ok());
    }

    #[test]
    fn rejects_endpoint_profile_capture_without_profile_requirement() {
        let document = SuiteDocument {
            suite: super::SuiteMetadata {
                id: "inline".to_owned(),
                standard: super::STANDARD.to_owned(),
                description: None,
            },
            cases: vec![super::TestCase {
                id: "endpoint-profile-capture-without-requirement".to_owned(),
                requirements: vec!["xapi-lrs-inline".to_owned()],
                safety: CaseSafety {
                    destructive: false,
                    requires_isolated_lrs: true,
                },
                requires_endpoint_profiles: Vec::new(),
                request: Some(super::RequestSpec {
                    auth_profile: None,
                    method: "PUT".to_owned(),
                    path: "/statements".to_owned(),
                    follow_more: None,
                    collect_statement_pages: None,
                    query: BTreeMap::default(),
                    headers: BTreeMap::default(),
                    body: None,
                    raw_body: Some(format!(
                        "{{{{{}}}}}",
                        super::ENDPOINT_CAPTURE_TOO_LARGE_BODY
                    )),
                    raw_body_hex: None,
                    await_retrieval_ms: None,
                    unrecognized_parameter: None,
                    raw_body_crlf_lines: None,
                    repeat: None,
                }),
                expect: Some(super::ResponseExpectation {
                    status: 413,
                    status_classification: super::StatusClassification::Required,
                    status_in: Vec::new(),
                    checks: Vec::new(),
                    headers: BTreeMap::default(),
                    body: None,
                    json: None,
                    capture: BTreeMap::default(),
                }),
                steps: Vec::new(),
            }],
        };
        let suites = [LoadedSuite {
            path: PathBuf::from("inline.yaml"),
            document,
        }];

        assert!(validate_suites(&suites).is_err());
    }

    #[test]
    fn generated_too_large_body_is_valid_statement_json() {
        let body = super::too_large_statement_body(2_048).expect("generated body");
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");

        assert!(body.len() >= 2_048);
        assert_eq!(
            parsed.pointer("/id"),
            Some(&json!("00000000-0000-4000-8000-000000000901"))
        );
    }

    #[test]
    fn json_contains_matches_nested_partial_objects() {
        let actual = json!([
            {
                "id": "00000000-0000-4000-8000-000000000306",
                "object": {
                    "objectType": "Activity",
                    "id": "http://example.org:80/kiva/conformance/activity/iri-default-port"
                }
            }
        ]);
        let expected = json!({
            "object": {
                "id": "http://example.org:80/kiva/conformance/activity/iri-default-port"
            }
        });

        assert_eq!(super::json_contains(&actual, &expected), Ok(true));
    }

    #[test]
    fn json_contains_rejects_missing_nested_partial_objects() {
        let actual = json!([
            {
                "id": "00000000-0000-4000-8000-000000000306",
                "object": {
                    "objectType": "Activity",
                    "id": "http://example.org:80/kiva/conformance/activity/iri-default-port"
                }
            }
        ]);
        let expected = json!({
            "id": "00000000-0000-4000-8000-000000000307"
        });

        assert_eq!(super::json_contains(&actual, &expected), Ok(false));
    }

    #[test]
    fn json_membership_does_not_match_identifier_prefixes() {
        let actual = serde_json::json!([{"id":"https://example.org/activity-extra"}]);
        let prefix = serde_json::json!({"id":"https://example.org/activity"});
        assert_eq!(super::json_contains(&actual, &prefix), Ok(false));
        let exact = serde_json::json!({"id":"https://example.org/activity-extra"});
        assert_eq!(super::json_contains(&actual, &exact), Ok(true));
    }

    #[test]
    fn nested_json_query_braces_are_literal_around_captures() {
        let literal = r#"{"account":{"homePage":"https://example.org","name":"learner"}}"#;
        assert!(
            super::template_variables(literal)
                .expect("literal JSON")
                .is_empty()
        );
        assert_eq!(
            super::render_template(literal, &BTreeMap::new()).expect("literal JSON"),
            literal
        );
        let template =
            r#"{"name":{},"account":{"homePage":"https://example.org","name":"{{learner}}"}}"#;
        let captures = BTreeMap::from([("learner".to_owned(), "alice".to_owned())]);
        assert_eq!(
            super::template_variables(template).expect("capture in JSON"),
            std::collections::BTreeSet::from(["learner".to_owned()])
        );
        let rendered = super::render_template(template, &captures).expect("render query");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&rendered)
                .expect("valid JSON")
                .pointer("/account/name"),
            Some(&json!("alice"))
        );
        assert!(super::template_variables("{{learner").is_err());
        assert!(super::render_template("{{missing}}", &captures).is_err());
    }
}

#[cfg(test)]
mod afc_tests;
