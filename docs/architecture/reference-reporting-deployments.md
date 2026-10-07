# Reference Reporting Deployments

## Boundary

BI2 provides validated reference reporting deployments for schools and
corporate training. Each deployment is guidance for an external HTTP and OAuth
consumer of the BI1 governed analytics API. Kiva does not host the dashboard,
does not select a BI vendor, and does not extend the IEEE/ISO/IEC
39274-1-1:2025 xAPI LRS conformance claim. The templates are not legal advice.

The machine-readable catalog is
`spec/analytics/reporting-templates.yaml`. Its validator checks the catalog
against the BI1 operation kernel and exact scope boundary.

## OAuth/OIDC Client

1. Register a deployment-owned confidential client with the external identity
   provider connected through `kiva-idp-adapter`.
2. Grant only `analytics:read`. No xAPI, PDS, or confidential-computation scope
   is required or implied.
3. Configure issuer, audience, JWKS or introspection, and client credentials as
   described by `oauth-oidc-resource-server-binding.md`.
4. When the provider adds non-authorizing OIDC labels such as `openid` or
   `profile`, list only those exact labels in `KIVA_IDP_IGNORED_SCOPES`. They
   are discarded and grant no Kiva authority. An unlisted unknown scope rejects
   the token, and a Kiva authorization scope cannot be placed in this list.
5. Keep credentials in the external consumer's secret facility. Do not embed
   them in browser assets, dashboard definitions, exported reports, or shared
   connection files.

## BI1 Request Sequence

The consumer uses the three BI1 operations in this order:

1. Call `getAnalyticsProducts` (`GET /analytics/products`) to enumerate visible
   approved products. Set `pageSize` from 1 through 100. Treat `nextCursor` as
   opaque and request subsequent pages with the cursor unchanged until it is
   absent or null. Do not merge cursors across tenant or subject partitions.
2. Call `getAnalyticsProduct` (`GET /analytics/products/{dataProductId}`) for a
   selected identifier. Use its metadata to label the report and verify that
   the release kind is `threshold-release` or
   `differential-privacy-release`.
3. Call `getAnalyticsProductAggregate`
   (`GET /analytics/products/{dataProductId}/aggregate`) and render only the
   admitted aggregate value returned by BI1. A differential-privacy product
   returns its admitted noised value rather than a source aggregate.

The dashboard must retain the deployment's tenant and subject partition when
caching product identifiers or cursors. A product visible in one partition is
not evidence that it is visible in another.

## Failure Handling

- `401 Unauthorized`: discard the refused token and reacquire it through the
  configured IdP flow. Do not retry anonymously.
- `403 Forbidden`: stop the request and correct the exact `analytics:read`
  grant. Do not add broader Kiva scopes as a workaround.
- `404 Not Found`: treat the product or admitted release as unavailable in the
  current governed partition. Do not probe other subjects or tenants.

Retries must remain bounded and must not convert authorization failures into
alternate data-source access. BI1 records read audit events at the Kiva
boundary; the external tool should separately retain its report refresh and
operator audit records.

## Dashboard Restrictions

The external dashboard must not:

- request or expose raw xAPI Statements or individual-level records;
- share one client credential across unrelated tenant partitions;
- infer that `analytics:read` grants any xAPI, PDS, or confidential scope;
- attempt re-identification, small-cell reconstruction, or linkage against
  external person-level sources;
- bypass a `401`, `403`, or `404` through direct database or storage access;
- describe the template as a hosted Kiva BI product or a production analytics
  guarantee.

The configuration is vendor-neutral. Any tool that can issue HTTPS requests,
obtain OAuth/OIDC access tokens, follow opaque cursor pagination, and parse JSON
can act as the external consumer.
