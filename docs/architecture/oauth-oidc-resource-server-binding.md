# OAuth/OIDC Resource-Server Binding

Kiva treats OAuth/OIDC as the platform authorization substrate for its xAPI,
PDS, and confidential-computation HTTP resources. M11 remains the
resource-server enforcement boundary: it consumes verified bearer-token
subjects and exact Kiva scopes, maps them to Kiva operations, and fails closed
when a request is unauthenticated or under-scoped.

M16 adds `kiva-idp-adapter` outside that core. The deployment adapter verifies
asymmetric JWKS-backed JWT access tokens or RFC 7662 token introspection
responses and then produces the verified principal consumed by M11. Kiva
connects to production OAuth/OIDC identity providers. This is a connectivity
claim, not a claim that Kiva provides an authorization server or a complete
production-ready authentication deployment.

## Deployment Adapter

The adapter exposes `BearerTokenVerifier`, which maps one untrusted bearer
token to `AuthenticatedPrincipal`. `IdpTokenVerifier` dispatches three-segment
tokens to `JwtTokenVerifier` and opaque tokens to
`IntrospectionTokenVerifier`. The server composition boundary stores the
verifier behind `Arc<dyn BearerTokenVerifier>`; HTTP handlers continue to see
only M11 authorization methods and typed operation witnesses.

JWT verification accepts only RSA JWKS keys and `RS256`, `RS384`, or `RS512`.
It requires `kid`, `sub`, `iss`, `aud`, and `exp`, validates `nbf` when present,
and rejects unknown keys, mismatched algorithms, invalid signatures, invalid
lifetimes, and issuer or audience mismatches. `alg: none`, HMAC downgrade, and
ambiguous tokens are rejected. JWKS may be loaded from a deployment-owned file
or fetched from a configured endpoint.

Opaque tokens are posted to a configured RFC 7662 endpoint with client basic
authentication. An inactive token, endpoint failure, invalid response,
issuer/audience mismatch, or invalid lifetime is rejected. `scope` strings and
`scp` strings or arrays are mapped to the exact scope vocabulary below. By
default, any unknown scope rejects the complete token rather than silently
widening or partially granting authority. A deployment may explicitly list
non-authorizing scope labels, such as `openid` or `profile`, in
`KIVA_IDP_IGNORED_SCOPES`. These labels are skipped and never become Kiva
permissions. Any unlisted unknown label still rejects the token, and listing a
Kiva authorization scope is an invalid startup configuration. An optional
deployment-specific tenant claim is required when configured and
deterministically qualifies the subject as `<tenant>:<sub>`.

The deployment environment is configured with:

- `KIVA_IDP_ISSUER` and `KIVA_IDP_AUDIENCE`;
- exactly one of `KIVA_IDP_JWKS_URL` or `KIVA_IDP_JWKS_FILE`, when JWTs are
  accepted;
- `KIVA_IDP_INTROSPECTION_URL`, `KIVA_IDP_CLIENT_ID`, and
  `KIVA_IDP_CLIENT_SECRET`, when opaque tokens are accepted;
- optional `KIVA_IDP_TENANT_CLAIM` for tenant-qualified subjects;
- optional, space-delimited `KIVA_IDP_IGNORED_SCOPES` for exact external scope
  labels that convey no Kiva authority.

At least one JWKS or introspection mode must be configured to install the
adapter. When installed, it is authoritative: fixture-token lookup is not a
fallback. If neither mode is configured, existing local conformance fixtures
retain their previous behavior.

## ADL Basic Compatibility Mode

`KIVA_ADL_COMPAT_BASIC_AUTH=true` installs a deployment-edge compatibility
middleware for the pinned ADL v2.0 test harness. On xAPI paths only, the fixed
helper-emitted credential `undefined:undefined` is translated to the existing
`statement-writer-a` fixture bearer identity before the M11 core boundary. The
setting is disabled by default. Unknown Basic credentials, malformed headers,
and Basic credentials on non-xAPI surfaces are not translated and remain
rejected by the Bearer-only core.

This mode does not add Basic authentication to the resource-server core and is
not a production identity-provider integration. It compensates for the ADL
Axios helper's unconditional Basic header so that the suite can exercise xAPI
resource semantics. IEEE/ISO/IEC 39274-1-1-2025 clause 4.1.8 does not define a
specific authentication mechanism.

## Scope Vocabulary

The policy crate defines the exact scope labels Kiva accepts:

- `xapi:read`
- `xapi:write`
- `pds:read`
- `pds:write`
- `pds:delegate`
- `pds:revoke`
- `pds:audit`
- `confidential:read`
- `confidential:write`
- `confidential:compute`
- `confidential:release`
- `analytics:read`

Scopes are not hierarchical. A write scope does not imply read, release,
delegate, revoke, audit, or compute authority unless the token carries each
required scope explicitly.

## Enforcement Rules

PDS and confidential-computation endpoints require a bearer token and the exact
operation scope declared by the OpenAPI contract. Missing or refused tokens
return `401 Unauthorized`; accepted tokens without the required scope return
`403 Forbidden`.

xAPI endpoints preserve Kiva's conformance-fixture behavior: anonymous xAPI
requests remain deployment configurable unless `KIVA_REQUIRE_AUTHORIZATION=true`.
When a bearer token is presented, or when authorization is required, xAPI
requests are checked against `xapi:read` or `xapi:write` as appropriate.

## Contract Projection

The OpenAPI contracts project the runtime boundary through
`x-kiva-required-scopes` on every protected operation:

- `spec/openapi/kiva-xapi.yaml`
- `spec/openapi/kiva-pds.yaml`
- `spec/openapi/kiva-confidential.yaml`
- `spec/openapi/kiva-analytics.yaml`

The OpenAPI validators reject missing or mismatched scope metadata, so contract
drift is caught in CI.

## Assurance Boundary

Rust tests cover missing, refused, and under-scoped tokens for xAPI, PDS, and
confidential-computation endpoints. The Kani harness
`oauth_scope_gate_matches_exact_resource_server_operations` checks that the pure
scope gate is an exact finite mapping from `OAuthScope` to
`ResourceServerOperation`.

The Tamarin OAuth/OIDC model remains a symbolic boundary for trusted issuers,
audiences, delegated scopes, revocation, introspection, token binding,
refresh-token exchange, and replay resistance. M16 conforms to that symbolic
trust boundary by producing a principal only after issuer, audience, lifetime,
signature or active-introspection, and scope checks. The model is not a proof
of `jsonwebtoken`, `reqwest`, JWKS retrieval or rotation, TLS, an external IdP,
or a production introspection service. Fixture-IdP contract tests are
implementation evidence for those concrete adapter paths, not a complete
protocol proof.
