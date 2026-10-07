use std::{error::Error, net::SocketAddr};

use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use kiva_identity::{OAuthScope, ResourceServerOperation};
use reqwest::Url;
use serde_json::{Value, json};
use time::OffsetDateTime;
use tokio::{net::TcpListener, task::JoinHandle};

use crate::{
    BearerTokenVerifier, ClaimMapping, IdpAdapterError, IdpTokenVerifier, IntrospectionConfig,
    IntrospectionTokenVerifier, JwksSource, JwtTokenVerifier, JwtVerifierConfig,
};

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

const ISSUER: &str = "https://idp.example.test";
const AUDIENCE: &str = "kiva-api";
const KEY_ID: &str = "kiva-fixture-rsa-1";
const MODULUS: &str = "q99MWyB8C0nsaBVGYVN9nngEXVaut7BNs5_nSagvv5scNMpuhz1HI0qk-Zv-KLr1dr77SjjDayjx2cUo1aDkKoLTBbFQcVW_zLak4bjiStcmct-16KeabKBJEdjXFL7QP0TkBTSCGNco2ZTdfkjabzRmXLeC_M-kg-jNdUkUuTfzHLs5BzCgpawXkxXqpLPD7VbtBeEe0ykkMp8TZ24hlQy4a0cVdaOUCUsmf_yf05YyHlNpOK8uj9d-AI-NACRklBU9E3SB_caBPzxSF0BzKKV1_98F2q8hCL5M1a-wfPJwhLFefW6fy0Ao9SGlefUL8FDPUSxOMHXTjw7YnhEddw";
const PRIVATE_KEY: &str = r"-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCr30xbIHwLSexo
FUZhU32eeARdVq63sE2zn+dJqC+/mxw0ym6HPUcjSqT5m/4ouvV2vvtKOMNrKPHZ
xSjVoOQqgtMFsVBxVb/MtqThuOJK1yZy37Xop5psoEkR2NcUvtA/ROQFNIIY1yjZ
lN1+SNpvNGZct4L8z6SD6M11SRS5N/McuzkHMKClrBeTFeqks8PtVu0F4R7TKSQy
nxNnbiGVDLhrRxV1o5QJSyZ//J/TljIeU2k4ry6P134Aj40AJGSUFT0TdIH9xoE/
PFIXQHMopXX/3wXaryEIvkzVr7B88nCEsV59bp/LQCj1IaV59QvwUM9RLE4wddOP
DtieER13AgMBAAECggEANgQe5kDmW1HnV6rgDLSFtWk4zfDxRk/Fi9+EfDS8lPZ1
ve2lISHUGrZZ9QzeqwRfIuvQ5DnEyQf8QpnhRIqlSusE0TQx+PyfJAJ+/WXy0Nyf
e9jtPep28Q7LCC3jIuT5Y/kNxMkE3jO45fEQT6Er7WlFhnwyUawco0xmTiciuTbk
d7rgjsvLgellTfUPoGnTJK5rf8jfig8k75U/E1mo7hwmNFmO5ZDO3KKJ0sAfxULb
APGyfqRcDd7mr/vOLD1ZsVaNbgVetSqhErsbXuQsx2oehJKyCR0kKseiP0le9zcX
KSRANdGsr6GWSOJjQkOd/hGoBoBMWWAl95CC2v8lGQKBgQDtaVU0Or82Qn2/+Y4m
PCcFHAGUSwZpOX0mGX51FRK+Ra0N82NarvcTh9m2rUVv2+4XcnX/y39iuk0NhEXP
6E2BVZrzCEUWg5icOfZZS7havwkC4met2SGirSTf5jGP68oVpd9kFHK9S1kwDcgT
leXvJSWHYWtRRp9LFA4vSeI7owKBgQC5VEzL9wIJG/0rnx5smqU8azjhyClBsDWk
zJmOirnLit7MVMSnb10jNUX0WZIhS+FS8J0GMG+C0sg3jLm40SYyq1C6xH3ZpCXW
b9mHEdxLIMDt+sMEL/KsroDQ1zxspmkXyckGYNtW4YQjUWkatqt9q0sGAt2Z8gXF
f3vHIJP0HQKBgEcU8IPQywtN4SSnbV4ZSH2JIxfpVqQOqF/N5kTb6F67sdKpeqfe
Fu/sDs1ZF58kNCclCBtsLFm8+fEAF37mSL5eiSQW58iT4g90c0PNHgddhy8r5mrw
xddVC+NVoqU6rajJnxbx4GC4vJ1covjqOovdjVz5WA5fnLX37T9PWcMNAoGAMk93
NZuNeumVNcVRXgwxNRCqdWiAAqwSM/Wy7eYs/9toFjIg3I83ynoIzoeOSumtdnWJ
OwflDa+o2KOkO07TvseQ99Klk3KjLu5YL/miQubWWQRM8pMNnuEUaR008w3N075c
ixW6FYDM/UV5cd4K+eeJS0112PfBp1avRGRMpeUCgYEAsYwkrZYXxzY4wqMJ9Lpf
CxRPVNp14GSXDGg8bFp6zikqDTrg6efyeXgU5mO6WUD+hGMGCP4uW6axxEp3RJVO
z0wS6VztsA6wBghmqEAbruMuz4RuCyzGbDnH5ow1cWJsZuniJKASRUwB0d7NJ3cJ
nOZ+j9m2u33HXd//gunxOaM=
-----END PRIVATE KEY-----";

#[derive(Clone)]
struct FixtureState {
    active: bool,
}

struct FixtureIdp {
    base_url: Url,
    task: JoinHandle<()>,
}

impl Drop for FixtureIdp {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl FixtureIdp {
    async fn start(active: bool) -> TestResult<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let address = listener.local_addr()?;
        let app = Router::new()
            .route("/jwks", get(jwks))
            .route("/introspect", axum::routing::post(introspect))
            .with_state(FixtureState { active });
        let task = tokio::spawn(async move {
            if let Err(error) = axum::serve(listener, app).await {
                eprintln!("fixture IdP failed: {error}");
            }
        });
        Ok(Self {
            base_url: Url::parse(&format!("http://{address}/"))?,
            task,
        })
    }

    fn endpoint(&self, path: &str) -> TestResult<Url> {
        Ok(self.base_url.join(path)?)
    }
}

async fn jwks() -> Json<Value> {
    Json(jwks_document())
}

async fn introspect(State(state): State<FixtureState>, headers: HeaderMap) -> Json<Value> {
    let authenticated = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("Basic "));
    let now = OffsetDateTime::now_utc().unix_timestamp();
    Json(if state.active && authenticated {
        json!({
            "active": true,
            "sub": "opaque-user",
            "iss": ISSUER,
            "aud": AUDIENCE,
            "exp": now + 300,
            "nbf": now - 1,
            "scope": "xapi:read pds:read",
            "scp": ["confidential:read"],
            "tenant_id": "tenant-a"
        })
    } else {
        json!({ "active": false })
    })
}

fn jwks_document() -> Value {
    json!({
        "keys": [{
            "kid": KEY_ID,
            "kty": "RSA",
            "alg": "RS256",
            "n": MODULUS,
            "e": "AQAB"
        }]
    })
}

fn jwt_verifier(endpoint: Url, tenant_claim: Option<&str>) -> JwtTokenVerifier {
    jwt_verifier_with_mapping(endpoint, ClaimMapping::new(tenant_claim.map(str::to_owned)))
}

fn jwt_verifier_with_mapping(endpoint: Url, claim_mapping: ClaimMapping) -> JwtTokenVerifier {
    JwtTokenVerifier::new(JwtVerifierConfig {
        issuer: ISSUER.to_owned(),
        audience: AUDIENCE.to_owned(),
        jwks: JwksSource::Remote(endpoint),
        claim_mapping,
    })
}

fn introspection_verifier(endpoint: Url) -> IntrospectionTokenVerifier {
    IntrospectionTokenVerifier::new(IntrospectionConfig {
        endpoint,
        client_id: "kiva-client".to_owned(),
        client_secret: "fixture-secret".to_owned(),
        issuer: ISSUER.to_owned(),
        audience: AUDIENCE.to_owned(),
        claim_mapping: ClaimMapping::new(Some("tenant_id".to_owned())),
    })
}

fn signed_token(
    algorithm: Algorithm,
    kid: &str,
    issuer: &str,
    audience: &str,
    expiration: i64,
    not_before: i64,
    scope: &Value,
) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = json!({
        "sub": "jwt-user",
        "iss": issuer,
        "aud": audience,
        "exp": expiration,
        "nbf": not_before,
        "scope": scope,
        "tenant_id": "tenant-a"
    });
    let mut header = Header::new(algorithm);
    header.kid = Some(kid.to_owned());
    encode(
        &header,
        &claims,
        &EncodingKey::from_rsa_pem(PRIVATE_KEY.as_bytes())?,
    )
}

fn now() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}

fn unused_endpoint(path: &str) -> TestResult<Url> {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0))?;
    let address: SocketAddr = listener.local_addr()?;
    drop(listener);
    Ok(Url::parse(&format!("http://{address}/{path}"))?)
}

#[tokio::test]
async fn remote_jwks_accepts_valid_token_and_maps_surface_scopes() -> TestResult {
    let idp = FixtureIdp::start(true).await?;
    let verifier = jwt_verifier(idp.endpoint("jwks")?, Some("tenant_id"));
    let token = signed_token(
        Algorithm::RS256,
        KEY_ID,
        ISSUER,
        AUDIENCE,
        now() + 300,
        now() - 1,
        &json!(["xapi:read", "pds:read", "confidential:read"]),
    )?;

    let principal = verifier.verify_token(&token).await?;
    assert_eq!(principal.subject, "tenant-a:jwt-user");
    assert!(principal.allows_operation(ResourceServerOperation::XapiRead));
    assert!(principal.allows_operation(ResourceServerOperation::PdsRead));
    assert!(principal.allows_operation(ResourceServerOperation::ConfidentialRead));
    assert!(!principal.allows_operation(ResourceServerOperation::XapiWrite));
    Ok(())
}

#[tokio::test]
async fn jwt_rejects_invalid_signature_and_unknown_key() -> TestResult {
    let idp = FixtureIdp::start(true).await?;
    let verifier = jwt_verifier(idp.endpoint("jwks")?, None);
    let token = signed_token(
        Algorithm::RS256,
        KEY_ID,
        ISSUER,
        AUDIENCE,
        now() + 300,
        now() - 1,
        &json!("xapi:read"),
    )?;
    let mut invalid_signature = token;
    invalid_signature.push('A');
    assert!(matches!(
        verifier.verify_token(&invalid_signature).await,
        Err(IdpAdapterError::InvalidJwt)
    ));

    let unknown_key = signed_token(
        Algorithm::RS256,
        "unknown-key",
        ISSUER,
        AUDIENCE,
        now() + 300,
        now() - 1,
        &json!("xapi:read"),
    )?;
    assert!(matches!(
        verifier.verify_token(&unknown_key).await,
        Err(IdpAdapterError::SigningKeyUnavailable)
    ));
    Ok(())
}

#[tokio::test]
async fn jwt_rejects_invalid_lifetime() -> TestResult {
    let idp = FixtureIdp::start(true).await?;
    let verifier = jwt_verifier(idp.endpoint("jwks")?, None);
    let expired = signed_token(
        Algorithm::RS256,
        KEY_ID,
        ISSUER,
        AUDIENCE,
        now() - 120,
        now() - 300,
        &json!("xapi:read"),
    )?;
    assert!(matches!(
        verifier.verify_token(&expired).await,
        Err(IdpAdapterError::InvalidJwt)
    ));

    let premature = signed_token(
        Algorithm::RS256,
        KEY_ID,
        ISSUER,
        AUDIENCE,
        now() + 300,
        now() + 120,
        &json!("xapi:read"),
    )?;
    assert!(matches!(
        verifier.verify_token(&premature).await,
        Err(IdpAdapterError::InvalidJwt)
    ));
    Ok(())
}

#[tokio::test]
async fn jwt_rejects_algorithm_issuer_and_audience_mismatch() -> TestResult {
    let idp = FixtureIdp::start(true).await?;
    let verifier = jwt_verifier(idp.endpoint("jwks")?, None);
    let wrong_algorithm = signed_token(
        Algorithm::RS384,
        KEY_ID,
        ISSUER,
        AUDIENCE,
        now() + 300,
        now() - 1,
        &json!("xapi:read"),
    )?;
    assert!(matches!(
        verifier.verify_token(&wrong_algorithm).await,
        Err(IdpAdapterError::AlgorithmNotAllowed)
    ));

    let mut symmetric_header = Header::new(Algorithm::HS256);
    symmetric_header.kid = Some(KEY_ID.to_owned());
    let symmetric = encode(
        &symmetric_header,
        &json!({
            "sub": "jwt-user",
            "iss": ISSUER,
            "aud": AUDIENCE,
            "exp": now() + 300
        }),
        &EncodingKey::from_secret(b"forbidden-symmetric-key"),
    )?;
    assert!(matches!(
        verifier.verify_token(&symmetric).await,
        Err(IdpAdapterError::AlgorithmNotAllowed)
    ));

    for (issuer, audience) in [("https://other.example", AUDIENCE), (ISSUER, "other-api")] {
        let token = signed_token(
            Algorithm::RS256,
            KEY_ID,
            issuer,
            audience,
            now() + 300,
            now() - 1,
            &json!("xapi:read"),
        )?;
        assert!(matches!(
            verifier.verify_token(&token).await,
            Err(IdpAdapterError::InvalidJwt)
        ));
    }
    Ok(())
}

#[tokio::test]
async fn introspection_accepts_active_token_and_maps_deterministically() -> TestResult {
    let idp = FixtureIdp::start(true).await?;
    let verifier = introspection_verifier(idp.endpoint("introspect")?);
    let first = verifier.verify_token("opaque-token").await?;
    let second = verifier.verify_token("opaque-token").await?;

    assert_eq!(first, second);
    assert_eq!(first.subject, "tenant-a:opaque-user");
    assert!(first.allows_operation(ResourceServerOperation::XapiRead));
    assert!(first.allows_operation(ResourceServerOperation::PdsRead));
    assert!(first.allows_operation(ResourceServerOperation::ConfidentialRead));
    Ok(())
}

#[tokio::test]
async fn introspection_rejects_inactive_and_unavailable_endpoints() -> TestResult {
    let inactive_idp = FixtureIdp::start(false).await?;
    let inactive = introspection_verifier(inactive_idp.endpoint("introspect")?);
    assert!(matches!(
        inactive.verify_token("opaque-token").await,
        Err(IdpAdapterError::InactiveToken)
    ));

    let unavailable = introspection_verifier(unused_endpoint("introspect")?);
    assert!(matches!(
        unavailable.verify_token("opaque-token").await,
        Err(IdpAdapterError::IntrospectionUnavailable)
    ));
    Ok(())
}

#[tokio::test]
async fn jwt_rejects_unknown_scope_instead_of_granting_it() -> TestResult {
    let idp = FixtureIdp::start(true).await?;
    let verifier = jwt_verifier(idp.endpoint("jwks")?, None);
    let token = signed_token(
        Algorithm::RS256,
        KEY_ID,
        ISSUER,
        AUDIENCE,
        now() + 300,
        now() - 1,
        &json!("xapi:read unknown:scope"),
    )?;
    assert!(matches!(
        verifier.verify_token(&token).await,
        Err(IdpAdapterError::UnsupportedScope)
    ));
    Ok(())
}

#[tokio::test]
async fn jwt_accepts_opted_in_oidc_scopes_without_granting_authority() -> TestResult {
    let idp = FixtureIdp::start(true).await?;
    let mapping =
        ClaimMapping::try_with_ignored_scopes(None, ["openid".to_owned(), "profile".to_owned()])?;
    let verifier = jwt_verifier_with_mapping(idp.endpoint("jwks")?, mapping);
    let token = signed_token(
        Algorithm::RS256,
        KEY_ID,
        ISSUER,
        AUDIENCE,
        now() + 300,
        now() - 1,
        &json!("openid xapi:read profile"),
    )?;

    let principal = verifier.verify_token(&token).await?;
    assert_eq!(
        principal.scopes(),
        &std::iter::once(OAuthScope::XapiRead).collect()
    );
    assert!(principal.allows_operation(ResourceServerOperation::XapiRead));
    assert!(!principal.allows_operation(ResourceServerOperation::XapiWrite));
    assert!(!principal.allows_operation(ResourceServerOperation::PdsRead));
    Ok(())
}

#[tokio::test]
async fn jwt_rejects_oidc_scopes_without_ignore_configuration() -> TestResult {
    let idp = FixtureIdp::start(true).await?;
    let verifier = jwt_verifier(idp.endpoint("jwks")?, None);
    let token = signed_token(
        Algorithm::RS256,
        KEY_ID,
        ISSUER,
        AUDIENCE,
        now() + 300,
        now() - 1,
        &json!("openid xapi:read profile"),
    )?;

    assert!(matches!(
        verifier.verify_token(&token).await,
        Err(IdpAdapterError::UnsupportedScope)
    ));
    Ok(())
}

#[tokio::test]
async fn jwt_with_ignore_configuration_rejects_unlisted_unknown_scope() -> TestResult {
    let idp = FixtureIdp::start(true).await?;
    let mapping = ClaimMapping::try_with_ignored_scopes(None, ["openid".to_owned()])?;
    let verifier = jwt_verifier_with_mapping(idp.endpoint("jwks")?, mapping);
    let token = signed_token(
        Algorithm::RS256,
        KEY_ID,
        ISSUER,
        AUDIENCE,
        now() + 300,
        now() - 1,
        &json!("openid xapi:read unknown:scope"),
    )?;

    assert!(matches!(
        verifier.verify_token(&token).await,
        Err(IdpAdapterError::UnsupportedScope)
    ));
    Ok(())
}

#[test]
fn ignored_scope_configuration_rejects_kiva_authorization_scopes() {
    for label in [
        "xapi:read",
        "xapi:write",
        "pds:read",
        "pds:write",
        "pds:delegate",
        "pds:revoke",
        "pds:audit",
        "confidential:read",
        "confidential:write",
        "confidential:compute",
        "confidential:release",
    ] {
        assert!(matches!(
            ClaimMapping::try_with_ignored_scopes(None, [label.to_owned()]),
            Err(IdpAdapterError::InvalidScopeConfiguration)
        ));
    }
}

#[tokio::test]
async fn ignored_scope_mapping_remains_deterministic() -> TestResult {
    let idp = FixtureIdp::start(true).await?;
    let mapping = ClaimMapping::try_with_ignored_scopes(
        Some("tenant_id".to_owned()),
        ["openid".to_owned(), "profile".to_owned()],
    )?;
    let verifier = jwt_verifier_with_mapping(idp.endpoint("jwks")?, mapping);
    let token = signed_token(
        Algorithm::RS256,
        KEY_ID,
        ISSUER,
        AUDIENCE,
        now() + 300,
        now() - 1,
        &json!(["profile", "xapi:read", "openid", "profile"]),
    )?;

    let first = verifier.verify_token(&token).await?;
    let second = verifier.verify_token(&token).await?;
    assert_eq!(first, second);
    assert_eq!(first.subject, "tenant-a:jwt-user");
    assert_eq!(
        first.scopes(),
        &std::iter::once(OAuthScope::XapiRead).collect()
    );
    Ok(())
}

#[tokio::test]
async fn jwt_rejects_missing_required_subject_or_tenant_claim() -> TestResult {
    let idp = FixtureIdp::start(true).await?;
    let verifier = jwt_verifier(idp.endpoint("jwks")?, Some("tenant_id"));
    let claims = json!({
        "iss": ISSUER,
        "aud": AUDIENCE,
        "exp": now() + 300,
        "scope": "xapi:read",
        "tenant_id": "tenant-a"
    });
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(KEY_ID.to_owned());
    let missing_subject = encode(
        &header,
        &claims,
        &EncodingKey::from_rsa_pem(PRIVATE_KEY.as_bytes())?,
    )?;
    assert!(matches!(
        verifier.verify_token(&missing_subject).await,
        Err(IdpAdapterError::InvalidJwt)
    ));

    let missing_tenant = signed_token(
        Algorithm::RS256,
        KEY_ID,
        ISSUER,
        AUDIENCE,
        now() + 300,
        now() - 1,
        &json!("xapi:read"),
    )?;
    let no_tenant_verifier = jwt_verifier(idp.endpoint("jwks")?, Some("missing_tenant"));
    assert!(matches!(
        no_tenant_verifier.verify_token(&missing_tenant).await,
        Err(IdpAdapterError::InvalidClaims)
    ));
    Ok(())
}

#[tokio::test]
async fn combined_verifier_routes_token_shapes_and_fails_closed() -> TestResult {
    let idp = FixtureIdp::start(true).await?;
    let combined = IdpTokenVerifier::new(
        Some(jwt_verifier(idp.endpoint("jwks")?, None)),
        Some(introspection_verifier(idp.endpoint("introspect")?)),
    );
    let jwt = signed_token(
        Algorithm::RS256,
        KEY_ID,
        ISSUER,
        AUDIENCE,
        now() + 300,
        now() - 1,
        &json!("xapi:read"),
    )?;
    assert_eq!(combined.verify(&jwt).await?.subject, "jwt-user");
    assert_eq!(
        combined.verify("opaque-token").await?.subject,
        "tenant-a:opaque-user"
    );

    let jwt_disabled = IdpTokenVerifier::new(
        None,
        Some(introspection_verifier(idp.endpoint("introspect")?)),
    );
    assert!(matches!(
        jwt_disabled.verify(&jwt).await,
        Err(IdpAdapterError::JwtDisabled)
    ));
    Ok(())
}
