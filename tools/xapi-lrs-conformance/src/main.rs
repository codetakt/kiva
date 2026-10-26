//! Command-line entry point for the xAPI LRS conformance aid.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

use clap::Parser;
use serde::Deserialize;
use url::Url;
use xapi_lrs_conformance::{
    AuthProfile, CaseOutcome, Report, RunFilter, RunMode, RunOptions, identifier_has_valid_syntax,
    load_endpoint_profile, run,
};

const AUTH_PROFILE_FILE_SCHEMA_VERSION: u16 = 1;
const STANDARD: &str = "IEEE/ISO/IEC 39274-1-1-2025";

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Cli {
    /// Validate a JSON batch of URI/IRI strings without an HTTP target.
    #[arg(long, value_name = "PATH", conflicts_with_all = [
        "case_roots", "suites", "cases", "requirements", "endpoint", "endpoint_profile",
        "headers", "bearer_token_env", "auth_profiles", "auth_profile_files",
        "validate_only", "allow_destructive", "junit"
    ])]
    check_identifiers: Option<PathBuf>,

    #[arg(long = "case", value_name = "PATH")]
    case_roots: Vec<PathBuf>,

    #[arg(long = "suite", value_name = "ID")]
    suites: Vec<String>,

    #[arg(long = "case-id", value_name = "ID")]
    cases: Vec<String>,

    #[arg(long = "requirement", value_name = "ID")]
    requirements: Vec<String>,

    #[arg(long, value_name = "URL")]
    endpoint: Option<Url>,

    #[arg(long = "endpoint-profile", value_name = "PATH")]
    endpoint_profile: Option<PathBuf>,

    #[arg(long = "header", value_name = "NAME:VALUE")]
    headers: Vec<HeaderArgument>,

    #[arg(long, value_name = "ENV")]
    bearer_token_env: Option<String>,

    #[arg(long = "auth-profile", value_name = "NAME=ENV")]
    auth_profiles: Vec<AuthProfileArgument>,

    #[arg(long = "auth-profile-file", value_name = "PATH")]
    auth_profile_files: Vec<PathBuf>,

    #[arg(long)]
    validate_only: bool,

    #[arg(long)]
    allow_destructive: bool,

    #[arg(long)]
    json: bool,

    #[arg(long, value_name = "PATH")]
    junit: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct HeaderArgument {
    name: String,
    value: String,
}

#[derive(Debug, Clone)]
struct AuthProfileArgument {
    name: String,
    env: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthProfileFileDocument {
    schema_version: u16,
    standard: String,
    profiles: BTreeMap<String, AuthProfileFileProfile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthProfileFileProfile {
    #[serde(default)]
    bearer_token_env: Option<String>,
    #[serde(default)]
    headers: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IdentifierInput {
    value: String,
    ascii_only: bool,
}

impl FromStr for HeaderArgument {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (raw_name, raw_value) = value
            .split_once(':')
            .ok_or_else(|| "header must use NAME:VALUE syntax".to_owned())?;
        let name = raw_name.trim();
        let header_value = raw_value.trim_start();
        if name.is_empty() {
            return Err("header name must not be empty".to_owned());
        }
        if header_value.is_empty() {
            return Err("header value must not be empty".to_owned());
        }
        Ok(Self {
            name: name.to_owned(),
            value: header_value.to_owned(),
        })
    }
}

impl FromStr for AuthProfileArgument {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (raw_name, raw_env) = value
            .split_once('=')
            .ok_or_else(|| "auth profile must use NAME=ENV syntax".to_owned())?;
        let name = raw_name.trim();
        let env = raw_env.trim();
        if name.is_empty() {
            return Err("auth profile name must not be empty".to_owned());
        }
        if env.is_empty() {
            return Err("auth profile env var must not be empty".to_owned());
        }
        Ok(Self {
            name: name.to_owned(),
            env: env.to_owned(),
        })
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    match execute().await {
        Ok(exit_code) => exit_code,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn execute() -> Result<ExitCode, xapi_lrs_conformance::Error> {
    let cli = Cli::parse();
    if let Some(path) = &cli.check_identifiers {
        return check_identifier_batch(path);
    }
    let common_headers = common_headers(&cli)?;
    let auth_profiles = auth_profiles(&cli)?;
    let endpoint_profile = cli
        .endpoint_profile
        .as_deref()
        .map(load_endpoint_profile)
        .transpose()?;
    let filter = run_filter(&cli);
    let options = RunOptions {
        case_roots: cli.case_roots,
        endpoint: cli.endpoint,
        common_headers,
        auth_profiles,
        endpoint_profile,
        validate_only: cli.validate_only,
        allow_destructive: cli.allow_destructive,
        filter,
    };
    let report = run(options).await?;

    if let Some(path) = &cli.junit {
        write_junit_report(path, &report)?;
    }

    if cli.json {
        print_json_report(&report)?;
    } else {
        print_text_report(&report);
    }

    if report.is_success() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}

fn check_identifier_batch(path: &Path) -> Result<ExitCode, xapi_lrs_conformance::Error> {
    let bytes = fs::read(path).map_err(|source| xapi_lrs_conformance::Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let entries: Vec<IdentifierInput> = serde_json::from_slice(&bytes).map_err(|error| {
        xapi_lrs_conformance::Error::Validation(format!("invalid identifier batch: {error}"))
    })?;
    let valid: Vec<bool> = entries
        .iter()
        .map(|entry| identifier_has_valid_syntax(&entry.value, entry.ascii_only))
        .collect();
    println!("{}", serde_json::json!({"valid": valid}));
    Ok(ExitCode::SUCCESS)
}

fn common_headers(cli: &Cli) -> Result<BTreeMap<String, String>, xapi_lrs_conformance::Error> {
    let mut headers = BTreeMap::new();
    for header in &cli.headers {
        insert_cli_header(&mut headers, &header.name, &header.value)?;
    }

    if let Some(variable) = &cli.bearer_token_env {
        if variable.is_empty() {
            return Err(xapi_lrs_conformance::Error::Validation(
                "--bearer-token-env must not be empty".to_owned(),
            ));
        }
        let token = std::env::var(variable).map_err(|error| {
            xapi_lrs_conformance::Error::Validation(format!(
                "failed to read bearer token from {variable}: {error}"
            ))
        })?;
        if token.is_empty() {
            return Err(xapi_lrs_conformance::Error::Validation(format!(
                "{variable} must not be empty"
            )));
        }
        insert_cli_header(&mut headers, "Authorization", &format!("Bearer {token}"))?;
    }

    Ok(headers)
}

fn auth_profiles(cli: &Cli) -> Result<BTreeMap<String, AuthProfile>, xapi_lrs_conformance::Error> {
    let mut profiles = BTreeMap::new();
    for path in &cli.auth_profile_files {
        let document = load_auth_profile_file(path)?;
        merge_auth_profile_file(&mut profiles, path, document, &read_env)?;
    }

    for profile in &cli.auth_profiles {
        let token = read_env(&profile.env)?;
        let mut headers = BTreeMap::new();
        insert_unique_header(
            &mut headers,
            "Authorization",
            &format!("Bearer {token}"),
            "auth profile",
        )?;
        insert_auth_profile(
            &mut profiles,
            &profile.name,
            AuthProfile { headers },
            "--auth-profile",
        )?;
    }
    Ok(profiles)
}

fn load_auth_profile_file(
    path: &Path,
) -> Result<AuthProfileFileDocument, xapi_lrs_conformance::Error> {
    let bytes = fs::read(path).map_err(|source| xapi_lrs_conformance::Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    serde_yaml::from_slice::<AuthProfileFileDocument>(&bytes).map_err(|source| {
        xapi_lrs_conformance::Error::ParseYaml {
            path: path.to_path_buf(),
            source,
        }
    })
}

fn merge_auth_profile_file(
    profiles: &mut BTreeMap<String, AuthProfile>,
    path: &Path,
    document: AuthProfileFileDocument,
    env: &impl Fn(&str) -> Result<String, xapi_lrs_conformance::Error>,
) -> Result<(), xapi_lrs_conformance::Error> {
    if document.schema_version != AUTH_PROFILE_FILE_SCHEMA_VERSION {
        return Err(xapi_lrs_conformance::Error::Validation(format!(
            "{} auth profile schema_version must be {AUTH_PROFILE_FILE_SCHEMA_VERSION}",
            path.display()
        )));
    }
    if document.standard != STANDARD {
        return Err(xapi_lrs_conformance::Error::Validation(format!(
            "{} auth profile standard must be {STANDARD}",
            path.display()
        )));
    }
    if document.profiles.is_empty() {
        return Err(xapi_lrs_conformance::Error::Validation(format!(
            "{} auth profile file must declare at least one profile",
            path.display()
        )));
    }

    for (name, profile) in document.profiles {
        let auth_profile = auth_profile_from_file_profile(path, &name, profile, env)?;
        insert_auth_profile(profiles, &name, auth_profile, &path.display().to_string())?;
    }
    Ok(())
}

fn auth_profile_from_file_profile(
    path: &Path,
    profile_name: &str,
    profile: AuthProfileFileProfile,
    env: &impl Fn(&str) -> Result<String, xapi_lrs_conformance::Error>,
) -> Result<AuthProfile, xapi_lrs_conformance::Error> {
    let mut headers = BTreeMap::new();
    if let Some(variable) = profile.bearer_token_env {
        let token = env(&variable)?;
        insert_unique_header(
            &mut headers,
            "Authorization",
            &format!("Bearer {token}"),
            &format!("auth profile {profile_name}"),
        )?;
    }
    for (header_name, variable) in profile.headers {
        let value = env(&variable)?;
        insert_unique_header(
            &mut headers,
            &header_name,
            &value,
            &format!("auth profile {profile_name}"),
        )?;
    }
    if headers.is_empty() {
        return Err(xapi_lrs_conformance::Error::Validation(format!(
            "{} auth profile {profile_name} must declare bearer_token_env or headers",
            path.display()
        )));
    }
    Ok(AuthProfile { headers })
}

fn read_env(variable: &str) -> Result<String, xapi_lrs_conformance::Error> {
    if variable.is_empty() {
        return Err(xapi_lrs_conformance::Error::Validation(
            "auth profile env var must not be empty".to_owned(),
        ));
    }
    let value = std::env::var(variable).map_err(|error| {
        xapi_lrs_conformance::Error::Validation(format!(
            "failed to read auth profile value from {variable}: {error}"
        ))
    })?;
    if value.is_empty() {
        return Err(xapi_lrs_conformance::Error::Validation(format!(
            "{variable} must not be empty"
        )));
    }
    Ok(value)
}

fn insert_auth_profile(
    profiles: &mut BTreeMap<String, AuthProfile>,
    name: &str,
    profile: AuthProfile,
    label: &str,
) -> Result<(), xapi_lrs_conformance::Error> {
    if profiles.contains_key(name) {
        return Err(xapi_lrs_conformance::Error::Validation(format!(
            "duplicate auth profile {name} in {label}"
        )));
    }
    profiles.insert(name.to_owned(), profile);
    Ok(())
}

fn run_filter(cli: &Cli) -> RunFilter {
    RunFilter {
        suites: cli.suites.iter().cloned().collect::<BTreeSet<_>>(),
        cases: cli.cases.iter().cloned().collect::<BTreeSet<_>>(),
        requirements: cli.requirements.iter().cloned().collect::<BTreeSet<_>>(),
    }
}

fn insert_cli_header(
    headers: &mut BTreeMap<String, String>,
    name: &str,
    value: &str,
) -> Result<(), xapi_lrs_conformance::Error> {
    insert_unique_header(headers, name, value, "common")
}

fn insert_unique_header(
    headers: &mut BTreeMap<String, String>,
    name: &str,
    value: &str,
    label: &str,
) -> Result<(), xapi_lrs_conformance::Error> {
    if headers
        .keys()
        .any(|existing| existing.eq_ignore_ascii_case(name))
    {
        return Err(xapi_lrs_conformance::Error::Validation(format!(
            "duplicate {label} header: {name}"
        )));
    }
    headers.insert(name.to_owned(), value.to_owned());
    Ok(())
}

fn print_json_report(report: &Report) -> Result<(), xapi_lrs_conformance::Error> {
    let text = serde_json::to_string_pretty(report)?;
    println!("{text}");
    Ok(())
}

fn write_junit_report(path: &Path, report: &Report) -> Result<(), xapi_lrs_conformance::Error> {
    let text = junit_report(report);
    fs::write(path, text).map_err(|source| xapi_lrs_conformance::Error::Write {
        path: path.to_path_buf(),
        source,
    })
}

fn junit_report(report: &Report) -> String {
    let mut output = String::new();
    output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    output.push_str("<testsuites name=\"");
    output.push_str(&escape_xml_attribute(report.standard));
    output.push_str("\" tests=\"");
    output.push_str(&report.totals.total.to_string());
    output.push_str("\" failures=\"");
    output.push_str(&report.totals.failed.to_string());
    output.push_str("\">\n");
    output.push_str("  <properties>\n");
    push_property(
        &mut output,
        "    ",
        "schema_version",
        &report.schema_version.to_string(),
    );
    push_property(&mut output, "    ", "standard", report.standard);
    push_property(&mut output, "    ", "mode", &report.mode.to_string());
    push_property(&mut output, "    ", "status", &report.status.to_string());
    output.push_str("  </properties>\n");

    for suite in &report.suites {
        let failures = suite
            .cases
            .iter()
            .filter(|case| matches!(case.outcome, CaseOutcome::Failed))
            .count();
        output.push_str("  <testsuite name=\"");
        output.push_str(&escape_xml_attribute(&suite.id));
        output.push_str("\" tests=\"");
        output.push_str(&suite.cases.len().to_string());
        output.push_str("\" failures=\"");
        output.push_str(&failures.to_string());
        output.push_str("\">\n");
        output.push_str("    <properties>\n");
        push_property(
            &mut output,
            "      ",
            "path",
            &suite.path.display().to_string(),
        );
        output.push_str("    </properties>\n");

        for case in &suite.cases {
            output.push_str("    <testcase classname=\"");
            output.push_str(&escape_xml_attribute(&suite.id));
            output.push_str("\" name=\"");
            output.push_str(&escape_xml_attribute(&case.id));
            output.push_str("\">\n");
            output.push_str("      <properties>\n");
            push_property(
                &mut output,
                "        ",
                "outcome",
                &case.outcome.to_string(),
            );
            push_property(
                &mut output,
                "        ",
                "requirements",
                &case.requirements.join(","),
            );
            output.push_str("      </properties>\n");
            for reason in &case.not_run_reasons {
                output.push_str("      <skipped message=\"");
                output.push_str(&escape_xml_attribute(reason));
                output.push_str("\" />\n");
            }
            push_property(
                &mut output,
                "      ",
                "diagnostic_statuses",
                &format!("{:?}", case.diagnostic_statuses),
            );
            for failure in &case.failures {
                output.push_str("      <failure message=\"");
                output.push_str(&escape_xml_attribute(failure));
                output.push_str("\">");
                output.push_str(&escape_xml_text(failure));
                output.push_str("</failure>\n");
            }
            output.push_str("    </testcase>\n");
        }

        output.push_str("  </testsuite>\n");
    }

    output.push_str("</testsuites>\n");
    output
}

fn push_property(output: &mut String, indent: &str, name: &str, value: &str) {
    output.push_str(indent);
    output.push_str("<property name=\"");
    output.push_str(&escape_xml_attribute(name));
    output.push_str("\" value=\"");
    output.push_str(&escape_xml_attribute(value));
    output.push_str("\" />\n");
}

fn escape_xml_attribute(value: &str) -> String {
    escape_xml(value)
}

fn escape_xml_text(value: &str) -> String {
    escape_xml(value)
}

fn escape_xml(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn print_text_report(report: &Report) {
    println!("{} xAPI LRS conformance aid", report.standard);
    println!("mode: {}", report.mode);
    println!("status: {}", report.status);
    println!(
        "cases: {} total, {} passed, {} failed, {} validated, {} not-run",
        report.totals.total,
        report.totals.passed,
        report.totals.failed,
        report.totals.validated,
        report.totals.not_run
    );

    for suite in &report.suites {
        println!("suite: {} ({})", suite.id, suite.path.display());
        for case in &suite.cases {
            println!("  [{}] {}", case.outcome, case.id);
            for reason in &case.not_run_reasons {
                println!("    - {reason}");
            }
            if !case.diagnostic_statuses.is_empty() {
                println!(
                    "    diagnostic status (outside claim): {:?}",
                    case.diagnostic_statuses
                );
            }
            for failure in &case.failures {
                println!("    - {failure}");
            }
        }
    }

    if matches!(report.mode, RunMode::ValidateOnly) {
        println!("HTTP execution skipped");
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::Path;
    use std::path::PathBuf;

    use xapi_lrs_conformance::{
        CaseOutcome, CaseReport, CaseSafety, Error, Report, ReportStatus, RunMode, SuiteReport,
        Totals,
    };

    #[test]
    fn junit_report_escapes_attributes_and_failures() {
        let report = Report {
            schema_version: 1,
            standard: "IEEE/ISO/IEC 39274-1-1-2025",
            mode: RunMode::ExecuteHttp,
            status: ReportStatus::Failed,
            totals: Totals {
                total: 1,
                passed: 0,
                failed: 1,
                validated: 0,
                not_run: 0,
            },
            suites: vec![SuiteReport {
                id: "suite<&\"".to_owned(),
                path: PathBuf::from("tests/conformance/http/example.yaml"),
                cases: vec![CaseReport {
                    id: "case<&\"".to_owned(),
                    requirements: vec!["xapi-lrs-inline".to_owned()],
                    safety: CaseSafety {
                        destructive: false,
                        requires_isolated_lrs: false,
                    },
                    outcome: CaseOutcome::Failed,
                    not_run_reasons: Vec::new(),
                    unresolved_checks: Vec::new(),
                    diagnostic_statuses: Vec::new(),
                    retrieval_attempts: Vec::new(),
                    failures: vec!["expected <ok> & got \"bad\"".to_owned()],
                }],
            }],
        };

        let xml = super::junit_report(&report);

        assert!(xml.contains("suite&lt;&amp;&quot;"));
        assert!(xml.contains("case&lt;&amp;&quot;"));
        assert!(xml.contains("expected &lt;ok&gt; &amp; got &quot;bad&quot;"));
        assert!(
            xml.contains("<failure message=\"expected &lt;ok&gt; &amp; got &quot;bad&quot;\">")
        );
    }

    #[test]
    fn auth_profile_file_supports_bearer_and_header_envs() -> Result<(), Error> {
        let mut header_envs = BTreeMap::new();
        header_envs.insert("X-Kiva-Fixture".to_owned(), "FIXTURE_HEADER".to_owned());
        let profile = super::AuthProfileFileProfile {
            bearer_token_env: Some("WRITER_TOKEN".to_owned()),
            headers: header_envs,
        };
        let env = |name: &str| match name {
            "WRITER_TOKEN" => Ok("writer-token".to_owned()),
            "FIXTURE_HEADER" => Ok("fixture-value".to_owned()),
            other => Err(Error::Validation(format!("unexpected env var: {other}"))),
        };

        let profile =
            super::auth_profile_from_file_profile(Path::new("auth.yaml"), "writer", profile, &env)?;

        assert_eq!(
            profile.headers.get("Authorization").map(String::as_str),
            Some("Bearer writer-token")
        );
        assert_eq!(
            profile.headers.get("X-Kiva-Fixture").map(String::as_str),
            Some("fixture-value")
        );
        Ok(())
    }

    #[test]
    fn auth_profile_file_rejects_duplicate_headers() {
        let mut header_envs = BTreeMap::new();
        header_envs.insert(
            "authorization".to_owned(),
            "AUTHORIZATION_HEADER".to_owned(),
        );
        let profile = super::AuthProfileFileProfile {
            bearer_token_env: Some("WRITER_TOKEN".to_owned()),
            headers: header_envs,
        };
        let env = |name: &str| match name {
            "WRITER_TOKEN" => Ok("writer-token".to_owned()),
            "AUTHORIZATION_HEADER" => Ok("Bearer other-token".to_owned()),
            other => Err(Error::Validation(format!("unexpected env var: {other}"))),
        };

        let result =
            super::auth_profile_from_file_profile(Path::new("auth.yaml"), "writer", profile, &env);

        assert!(result.is_err());
    }
}
