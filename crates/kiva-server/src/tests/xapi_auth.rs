#![allow(clippy::wildcard_imports)]

use super::*;
use kiva_idp_adapter::{BearerTokenVerifier, IdpAdapterError, VerificationFuture};
use std::sync::Arc;

struct FixtureTokenVerifier;

impl BearerTokenVerifier for FixtureTokenVerifier {
    fn verify<'a>(&'a self, token: &'a str) -> VerificationFuture<'a> {
        Box::pin(async move {
            match token {
                "idp-reader" => Ok(kiva_identity::AuthenticatedPrincipal::new(
                    "tenant-a:reader",
                    kiva_identity::StatementWritePermission::Denied,
                    [OAuthScope::XapiRead].into(),
                )),
                "idp-writer" => Ok(kiva_identity::AuthenticatedPrincipal::new(
                    "tenant-a:writer",
                    kiva_identity::StatementWritePermission::Allowed,
                    [OAuthScope::XapiRead, OAuthScope::XapiWrite].into(),
                )),
                "idp-all-surfaces" => Ok(kiva_identity::AuthenticatedPrincipal::new(
                    "tenant-a:operator",
                    kiva_identity::StatementWritePermission::Allowed,
                    [
                        OAuthScope::XapiRead,
                        OAuthScope::XapiWrite,
                        OAuthScope::PdsRead,
                        OAuthScope::ConfidentialRelease,
                    ]
                    .into(),
                )),
                _ => Err(IdpAdapterError::InactiveToken),
            }
        })
    }
}

#[test]
fn normalizes_missing_statement_id() {
    let statement_id = Uuid::nil();
    let stored_at = fixed_time();
    let statement = match super::normalized_statement(
        json!({
            "actor": {
                "objectType": "Agent",
                "mbox": "mailto:test@example.org"
            },
            "verb": {
                "id": "https://adlnet.gov/expapi/verbs/experienced"
            },
            "object": {
                "id": "https://example.org/activity"
            }
        }),
        statement_id,
        stored_at,
        &super::authority_agent("test-writer"),
    ) {
        Ok(statement) => statement,
        Err(error) => panic!("statement should normalize: {error:?}"),
    };

    assert_eq!(
        statement.pointer("/id"),
        Some(&json!(statement_id.to_string()))
    );
    assert_eq!(statement.pointer("/version"), Some(&json!("2.0.0")));
    assert_eq!(
        statement.pointer("/stored"),
        Some(&json!("2025-01-01T00:00:00Z"))
    );
    assert_eq!(
        statement.pointer("/timestamp"),
        Some(&json!("2025-01-01T00:00:00Z"))
    );
    assert_eq!(
        statement.pointer("/authority/account/name"),
        Some(&json!("test-writer"))
    );
}

#[test]
fn rejects_mismatched_statement_id() {
    let result = super::normalized_statement(
        json!({
            "id": "00000000-0000-4000-8000-000000000001"
        }),
        Uuid::nil(),
        fixed_time(),
        &super::authority_agent("test-writer"),
    );

    assert!(result.is_err());
}

#[test]
fn preserves_single_context_activity_during_admission() {
    let statement_id = Uuid::nil();
    let statement = match super::normalized_statement(
        json!({
            "id": statement_id,
            "actor": {
                "objectType": "Agent",
                "mbox": "mailto:test@example.org"
            },
            "verb": {
                "id": "https://adlnet.gov/expapi/verbs/experienced"
            },
            "object": {
                "id": "https://example.org/activity"
            },
            "context": {
                "contextActivities": {
                    "parent": {
                        "objectType": "Activity",
                        "id": "https://example.org/parent"
                    }
                }
            }
        }),
        statement_id,
        fixed_time(),
        &super::authority_agent("test-writer"),
    ) {
        Ok(statement) => statement,
        Err(error) => panic!("statement should be admitted: {error:?}"),
    };

    assert_eq!(
        statement.pointer("/context/contextActivities/parent/id"),
        Some(&json!("https://example.org/parent"))
    );
}

#[test]
fn statement_comparison_ignores_lrs_managed_stored_time() {
    let left = json!({
        "id": "00000000-0000-4000-8000-000000000001",
        "stored": "2025-01-01T00:00:00Z",
        "timestamp": "2025-01-01T00:00:00Z",
        "result": {
            "duration": "PT1.231S"
        }
    });
    let right = json!({
        "id": "00000000-0000-4000-8000-000000000001",
        "stored": "2025-01-01T00:00:01Z",
        "timestamp": "2025-01-01T00:00:01Z",
        "result": {
            "duration": "PT1.232S"
        }
    });

    assert!(super::statements_equivalent(&left, &right));
}

#[tokio::test]
async fn rejects_refused_credential() {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_static("Bearer invalid-credential"),
    );

    let auth = super::AuthConfig::default();
    let result = super::authorize_xapi_request(
        &auth,
        &headers,
        &BTreeMap::new(),
        ResourceServerOperation::XapiRead,
    )
    .await;

    assert!(matches!(
        result,
        Err(super::HttpError {
            status: StatusCode::UNAUTHORIZED,
            ..
        })
    ));
}

#[tokio::test]
async fn rejects_basic_credentials_without_deployment_translation() {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_static("Basic dW5kZWZpbmVkOnVuZGVmaW5lZA=="),
    );

    let result = super::authorize_xapi_request(
        &super::AuthConfig::default(),
        &headers,
        &BTreeMap::new(),
        ResourceServerOperation::XapiRead,
    )
    .await;

    assert!(matches!(
        result,
        Err(super::HttpError {
            status: StatusCode::UNAUTHORIZED,
            ..
        })
    ));
}

#[tokio::test]
async fn adl_basic_translation_maps_only_the_known_fixture_credentials() -> Result<(), String> {
    let mut known_headers = HeaderMap::new();
    known_headers.insert(
        AUTHORIZATION,
        HeaderValue::from_static("Basic dW5kZWZpbmVkOnVuZGVmaW5lZA=="),
    );
    crate::deployment::translate_adl_basic_auth_for_path("/statements", &mut known_headers);

    let principal = super::authorize_xapi_request(
        &super::AuthConfig::default(),
        &known_headers,
        &BTreeMap::new(),
        ResourceServerOperation::XapiWrite,
    )
    .await
    .map_err(|error| error.message)?
    .ok_or_else(|| "translated ADL credentials should produce a principal".to_owned())?;
    assert_eq!(principal.subject, "statement-writer-a");

    let mut unknown_headers = HeaderMap::new();
    unknown_headers.insert(
        AUTHORIZATION,
        HeaderValue::from_static("Basic dW5rbm93bjp1bmtub3du"),
    );
    crate::deployment::translate_adl_basic_auth_for_path("/statements", &mut unknown_headers);
    let unknown = super::authorize_xapi_request(
        &super::AuthConfig::default(),
        &unknown_headers,
        &BTreeMap::new(),
        ResourceServerOperation::XapiRead,
    )
    .await;
    assert!(matches!(
        unknown,
        Err(super::HttpError {
            status: StatusCode::UNAUTHORIZED,
            ..
        })
    ));
    Ok(())
}

#[test]
fn adl_basic_translation_does_not_apply_to_extension_surfaces() {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_static("Basic dW5kZWZpbmVkOnVuZGVmaW5lZA=="),
    );

    crate::deployment::translate_adl_basic_auth_for_path("/pds/consents", &mut headers);

    assert_eq!(
        headers.get(AUTHORIZATION),
        Some(&HeaderValue::from_static(
            "Basic dW5kZWZpbmVkOnVuZGVmaW5lZA=="
        ))
    );
}

#[tokio::test]
async fn rejects_oauth_access_token_query_parameter() {
    let mut query = BTreeMap::new();
    query.insert("access_token".to_owned(), "opaque-token".to_owned());

    let auth = super::AuthConfig::default();
    let result = super::authorize_xapi_request(
        &auth,
        &HeaderMap::new(),
        &query,
        ResourceServerOperation::XapiRead,
    )
    .await;

    assert!(matches!(
        result,
        Err(super::HttpError {
            status: StatusCode::BAD_REQUEST,
            ..
        })
    ));
}

#[tokio::test]
async fn required_authorization_rejects_anonymous_xapi_requests() {
    let auth = super::AuthConfig::with_required_authorization(true);

    let result = super::authorize_xapi_request(
        &auth,
        &HeaderMap::new(),
        &BTreeMap::new(),
        ResourceServerOperation::XapiRead,
    )
    .await;

    assert!(matches!(
        result,
        Err(super::HttpError {
            status: StatusCode::UNAUTHORIZED,
            ..
        })
    ));
}

#[tokio::test]
async fn required_authorization_accepts_bearer_xapi_requests() -> Result<(), String> {
    let auth = super::AuthConfig::with_required_authorization(true);
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_static("Bearer statement-writer-a"),
    );

    let principal = super::authorize_xapi_request(
        &auth,
        &headers,
        &BTreeMap::new(),
        ResourceServerOperation::XapiRead,
    )
    .await
    .map_err(|error| error.message)?
    .ok_or_else(|| "principal should be present".to_owned())?;

    assert_eq!(principal.subject, "statement-writer-a");
    Ok(())
}

#[tokio::test]
async fn xapi_write_requires_xapi_write_scope_when_token_is_present() {
    let auth = super::AuthConfig::default();
    let headers = confidential_headers_for("statement-reader");

    let result = super::authorize_xapi_request(
        &auth,
        &headers,
        &BTreeMap::new(),
        ResourceServerOperation::XapiWrite,
    )
    .await;

    assert!(matches!(
        result,
        Err(super::HttpError {
            status: StatusCode::FORBIDDEN,
            ..
        })
    ));
}

#[tokio::test]
async fn pds_requests_require_matching_oauth_scope() {
    let mut auth = super::AuthConfig::default();
    auth.register_bearer_scopes("pds-read-only", [OAuthScope::PdsRead]);
    let headers = confidential_headers_for("pds-read-only");

    let read = super::authorize_pds_request(
        &auth,
        &headers,
        &BTreeMap::new(),
        ResourceServerOperation::PdsRead,
    )
    .await;
    assert!(read.is_ok());

    let write = super::authorize_pds_request(
        &auth,
        &headers,
        &BTreeMap::new(),
        ResourceServerOperation::PdsWrite,
    )
    .await;
    assert!(matches!(
        write,
        Err(super::HttpError {
            status: StatusCode::FORBIDDEN,
            ..
        })
    ));
}

#[tokio::test]
async fn confidential_requests_require_matching_oauth_scope() {
    let mut auth = super::AuthConfig::default();
    auth.register_bearer_scopes("confidential-read-only", [OAuthScope::ConfidentialRead]);
    let headers = confidential_headers_for("confidential-read-only");

    let read = super::authorize_confidential_request(
        &auth,
        &headers,
        &BTreeMap::new(),
        ResourceServerOperation::ConfidentialRead,
    )
    .await;
    assert!(read.is_ok());

    let release = super::authorize_confidential_request(
        &auth,
        &headers,
        &BTreeMap::new(),
        ResourceServerOperation::ConfidentialRelease,
    )
    .await;
    assert!(matches!(
        release,
        Err(super::HttpError {
            status: StatusCode::FORBIDDEN,
            ..
        })
    ));
}

#[tokio::test]
async fn reader_principal_cannot_write_statements() -> Result<(), String> {
    let auth = super::AuthConfig::default();
    let principal = auth
        .principal_for_bearer_token("statement_reader")
        .await
        .map_err(|error| error.message)?;

    let result = super::statement_authority(Some(&principal));

    assert!(matches!(
        result,
        Err(super::HttpError {
            status: StatusCode::FORBIDDEN,
            ..
        })
    ));
    Ok(())
}

#[tokio::test]
async fn writer_principal_maps_to_deterministic_authority() -> Result<(), String> {
    let auth = super::AuthConfig::default();
    let principal = auth
        .principal_for_bearer_token("statement_writer_a")
        .await
        .map_err(|error| error.message)?;

    let authority = super::statement_authority(Some(&principal)).map_err(|error| error.message)?;

    assert_eq!(
        authority.pointer("/account/name"),
        Some(&json!("statement-writer-a"))
    );
    assert_eq!(
        authority.pointer("/account/homePage"),
        Some(&json!("https://example.org/kiva/conformance/auth"))
    );
    Ok(())
}

#[tokio::test]
async fn auth_config_maps_custom_conformance_fixture_tokens() -> Result<(), String> {
    let mut auth = super::AuthConfig::default();
    auth.register_bearer_subject("opaque-writer-token", "statement-writer-a");
    auth.register_statement_reader_token("opaque-reader-token");
    auth.register_bearer_subject("opaque-reader-token", "statement-reader");
    auth.register_refused_token("opaque-invalid-token");

    let writer = auth
        .principal_for_bearer_token("opaque-writer-token")
        .await
        .map_err(|error| error.message)?;
    let authority = super::statement_authority(Some(&writer)).map_err(|error| error.message)?;
    assert_eq!(
        authority.pointer("/account/name"),
        Some(&json!("statement-writer-a"))
    );

    let reader = auth
        .principal_for_bearer_token("opaque-reader-token")
        .await
        .map_err(|error| error.message)?;
    assert!(matches!(
        super::statement_authority(Some(&reader)),
        Err(super::HttpError {
            status: StatusCode::FORBIDDEN,
            ..
        })
    ));

    assert!(matches!(
        auth.principal_for_bearer_token("opaque-invalid-token")
            .await,
        Err(super::HttpError {
            status: StatusCode::UNAUTHORIZED,
            ..
        })
    ));
    Ok(())
}

#[tokio::test]
async fn external_verifier_is_authoritative_and_preserves_scope_statuses() {
    let auth = super::AuthConfig::with_token_verifier(true, Arc::new(FixtureTokenVerifier));

    let accepted = super::authorize_xapi_request(
        &auth,
        &confidential_headers_for("idp-writer"),
        &BTreeMap::new(),
        ResourceServerOperation::XapiWrite,
    )
    .await;
    assert!(accepted.is_ok());

    let all_surface_headers = confidential_headers_for("idp-all-surfaces");
    let pds = super::authorize_pds_request(
        &auth,
        &all_surface_headers,
        &BTreeMap::new(),
        ResourceServerOperation::PdsRead,
    )
    .await;
    assert!(pds.is_ok());
    let confidential = super::authorize_confidential_request(
        &auth,
        &all_surface_headers,
        &BTreeMap::new(),
        ResourceServerOperation::ConfidentialRelease,
    )
    .await;
    assert!(confidential.is_ok());

    let under_scoped = super::authorize_xapi_request(
        &auth,
        &confidential_headers_for("idp-reader"),
        &BTreeMap::new(),
        ResourceServerOperation::XapiWrite,
    )
    .await;
    assert!(matches!(
        under_scoped,
        Err(super::HttpError {
            status: StatusCode::FORBIDDEN,
            ..
        })
    ));

    let fixture_fallback = super::authorize_xapi_request(
        &auth,
        &confidential_headers_for("statement-writer-a"),
        &BTreeMap::new(),
        ResourceServerOperation::XapiWrite,
    )
    .await;
    assert!(matches!(
        fixture_fallback,
        Err(super::HttpError {
            status: StatusCode::UNAUTHORIZED,
            ..
        })
    ));
}

#[test]
fn extracts_promoted_statement_metadata() {
    let metadata = super::StatementMetadata::from_statement(
        &json!({
            "actor": {
                "objectType": "Agent",
                "account": {
                    "homePage": "https://example.edu",
                    "name": "learner-1"
                }
            },
            "verb": {
                "id": "https://adlnet.gov/expapi/verbs/experienced"
            },
            "object": {
                "id": "https://example.edu/activity/1"
            },
            "authority": {
                "objectType": "Agent",
                "mbox": "mailto:lrs@example.edu"
            },
            "version": "2.0.0"
        }),
        super::XAPI_VERSION,
    );

    assert_eq!(
        metadata.actor_ifi.as_deref(),
        Some("account:https://example.edu:learner-1")
    );
    assert_eq!(
        metadata.verb_id.as_deref(),
        Some("https://adlnet.gov/expapi/verbs/experienced")
    );
    assert_eq!(
        metadata.object_id.as_deref(),
        Some("https://example.edu/activity/1")
    );
    assert_eq!(
        metadata.authority,
        Some(json!({
            "objectType": "Agent",
            "mbox": "mailto:lrs@example.edu"
        }))
    );
    assert_eq!(metadata.version, "2.0.0");
}
