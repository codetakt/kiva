#!/usr/bin/env python3
"""Render the Rust application operation catalog from the YAML ledger."""

from __future__ import annotations

import json
from typing import Any


def require_entries(catalog: dict[str, Any]) -> list[dict[str, Any]]:
    """Return catalog entries or raise a validation error."""
    entries = catalog.get("entries")
    if not isinstance(entries, list):
        raise ValueError("operation catalog entries must be a list")
    for index, entry in enumerate(entries):
        if not isinstance(entry, dict):
            raise ValueError(f"operation catalog entries[{index}] must be a mapping")
    return entries


def catalog_source_path(catalog: dict[str, Any]) -> str:
    """Return the source path shown in generated artifacts."""
    value = catalog.get("generated_from", "generated/architecture/operation-catalog.yaml")
    if not isinstance(value, str) or not value:
        raise ValueError("operation catalog generated_from must be a non-empty string")
    return value


def surface_variant(surface: str) -> str:
    """Return the Rust surface variant for a catalog surface label."""
    match surface:
        case "pds":
            return "Pds"
        case "confidential":
            return "Confidential"
        case "analytics":
            return "Analytics"
        case _:
            raise ValueError(f"unknown operation catalog surface: {surface}")


def rust_string(value: str) -> str:
    """Return a Rust string literal for a catalog scalar."""
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def render_scope_list(scopes: list[str]) -> str:
    """Render required scopes as a Rust string slice literal."""
    return "&[" + ", ".join(rust_string(scope) for scope in scopes) + "]"


def required_scopes(entry: dict[str, Any]) -> list[str]:
    """Return validated required scopes for one catalog entry."""
    scopes = entry.get("required_scopes")
    if not isinstance(scopes, list) or not all(isinstance(scope, str) for scope in scopes):
        raise ValueError(f"{entry.get('id', '<unknown>')} required_scopes must be a string list")
    return scopes


def render_entry(entry: dict[str, Any]) -> str:
    """Render one operation_spec! macro invocation."""
    required = {
        "id",
        "surface",
        "rust_workflow_kind",
        "openapi_contract",
        "operation_id",
        "required_scopes",
    }
    missing = sorted(required.difference(entry))
    if missing:
        raise ValueError(f"operation catalog entry missing fields: {', '.join(missing)}")
    scopes = required_scopes(entry)
    return "\n".join(
        [
            "    operation_spec!(",
            f"        {rust_string(entry['id'])},",
            f"        ApplicationOperationSurface::{surface_variant(entry['surface'])},",
            f"        {entry['rust_workflow_kind']}.workflow_spec(),",
            f"        {rust_string(entry['rust_workflow_kind'])},",
            f"        {rust_string(entry['openapi_contract'])},",
            f"        {rust_string(entry['operation_id'])},",
            f"        {render_scope_list(scopes)},",
            "    ),",
        ]
    )


def operation_id_variant(catalog_id: str) -> str:
    """Return a Rust enum variant name for an operation catalog id."""
    parts = catalog_id.replace("_", "-").split("-")
    if not parts or any(not part or not part.replace("-", "").isalnum() for part in parts):
        raise ValueError(f"operation catalog id cannot be rendered as Rust enum: {catalog_id}")
    return "".join(part[:1].upper() + part[1:] for part in parts)


def render_operation_id_enum(entries: list[dict[str, Any]]) -> str:
    """Render the generated operation-id enum."""
    variants = "\n".join(
        [
            f"    /// Catalog operation `{entry['id']}`.",
            f"    {operation_id_variant(entry['id'])},",
        ][line]
        for entry in entries
        for line in range(2)
    )
    return f"""/// Typed operation identifiers generated from the application operation catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApplicationOperationId {{
{variants}
}}
"""


def render_operation_id_impl(entries: list[dict[str, Any]]) -> str:
    """Render generated operation-id lookup methods."""
    catalog_id_arms = "\n".join(
        f"            Self::{operation_id_variant(entry['id'])} => {rust_string(entry['id'])},"
        for entry in entries
    )
    spec_arms = "\n".join(
        f"            Self::{operation_id_variant(entry['id'])} => APPLICATION_OPERATION_SPECS[{index}],"
        for index, entry in enumerate(entries)
    )
    from_catalog_arms = "\n".join(
        f"            {rust_string(entry['id'])} => Some(Self::{operation_id_variant(entry['id'])}),"
        for entry in entries
    )
    from_openapi_arms = "\n".join(
        "            "
        + f"({rust_string(entry['openapi_contract'])}, {rust_string(entry['operation_id'])}) "
        + f"=> Some(Self::{operation_id_variant(entry['id'])}),"
        for entry in entries
    )
    return f"""impl ApplicationOperationId {{
    /// Returns the stable catalog identifier.
    #[must_use]
    pub const fn catalog_id(self) -> &'static str {{
        match self {{
{catalog_id_arms}
        }}
    }}

    /// Returns the generated operation specification.
    #[must_use]
    pub fn spec(self) -> ApplicationOperationSpec {{
        match self {{
{spec_arms}
        }}
    }}

    /// Resolves a stable catalog identifier to a typed operation id.
    #[must_use]
    pub fn from_catalog_id(catalog_id: &str) -> Option<Self> {{
        match catalog_id {{
{from_catalog_arms}
            _ => None,
        }}
    }}

    /// Resolves an `OpenAPI` contract path and operationId to a typed operation id.
    #[must_use]
    pub fn from_openapi_operation(openapi_contract: &str, operation_id: &str) -> Option<Self> {{
        match (openapi_contract, operation_id) {{
{from_openapi_arms}
            _ => None,
        }}
    }}
}}

/// Resolves a stable catalog identifier to an operation specification.
#[must_use]
pub fn application_operation_spec_by_catalog_id(
    catalog_id: &str,
) -> Option<ApplicationOperationSpec> {{
    ApplicationOperationId::from_catalog_id(catalog_id).map(ApplicationOperationId::spec)
}}

/// Resolves an `OpenAPI` contract path and operationId to an operation specification.
#[must_use]
pub fn application_operation_spec_by_openapi_operation(
    openapi_contract: &str,
    operation_id: &str,
) -> Option<ApplicationOperationSpec> {{
    ApplicationOperationId::from_openapi_operation(openapi_contract, operation_id)
        .map(ApplicationOperationId::spec)
}}
"""


def render_operation_ids(catalog: dict[str, Any]) -> str:
    """Render the generated Rust operation-id source."""
    entries = require_entries(catalog)
    source_path = catalog_source_path(catalog)
    operation_id_enum = render_operation_id_enum(entries)
    operation_id_impl = render_operation_id_impl(entries)
    return f"""//! Generated application operation identifiers.
//!
//! Generated from `{source_path}` by
//! `scripts/ci/generate_application_operation_catalog.py`.

use super::super::ApplicationOperationSpec;
use super::registry::APPLICATION_OPERATION_SPECS;

{operation_id_enum}

{operation_id_impl}

#[cfg(test)]
mod tests {{
    use super::{{
        ApplicationOperationId, application_operation_spec_by_catalog_id,
        application_operation_spec_by_openapi_operation,
    }};
    use super::super::registry::APPLICATION_OPERATION_SPECS;

    #[test]
    fn application_operation_ids_round_trip_to_specs() {{
        for spec in APPLICATION_OPERATION_SPECS {{
            let operation_id = ApplicationOperationId::from_catalog_id(spec.id)
                .expect("catalog id should resolve to a typed operation id");
            assert_eq!(operation_id.catalog_id(), spec.id);
            assert_eq!(operation_id.spec(), *spec);
            assert_eq!(
                ApplicationOperationId::from_openapi_operation(
                    spec.openapi_contract,
                    spec.operation_id,
                ),
                Some(operation_id),
            );
            assert_eq!(application_operation_spec_by_catalog_id(spec.id), Some(*spec));
            assert_eq!(
                application_operation_spec_by_openapi_operation(
                    spec.openapi_contract,
                    spec.operation_id,
                ),
                Some(*spec),
            );
        }}
    }}
}}
"""


def markdown_cell(value: object) -> str:
    """Render a Markdown table cell without introducing pipe ambiguity."""
    return str(value).replace("|", "\\|")


def render_operation_catalog_markdown(catalog: dict[str, Any]) -> str:
    """Render a human-readable operation catalog evidence table."""
    entries = require_entries(catalog)
    source_path = catalog_source_path(catalog)
    rows = [
        "| Catalog ID | Surface | Workflow Label | Rust Workflow | OpenAPI Operation | Required Scopes |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    for entry in entries:
        scopes = ", ".join(f"`{scope}`" for scope in required_scopes(entry))
        rows.append(
            "| "
            + " | ".join(
                [
                    f"`{markdown_cell(entry['id'])}`",
                    markdown_cell(entry["surface"]),
                    f"`{markdown_cell(entry['workflow_label'])}`",
                    f"`{markdown_cell(entry['rust_workflow_kind'])}`",
                    (
                        f"`{markdown_cell(entry['openapi_contract'])}"
                        f"#{markdown_cell(entry['operation_id'])}`"
                    ),
                    scopes,
                ]
            )
            + " |"
        )
    return f"""# Generated Application Operation Catalog

This file is generated from `{source_path}` by
`scripts/ci/generate_application_operation_catalog.py`.

It is review evidence for the join between application workflow labels, Rust
workflow variants, OpenAPI `operationId` values, and OAuth/OIDC resource-server
scopes.

{chr(10).join(rows)}
"""


def render_operation_catalog_evidence(catalog: dict[str, Any]) -> str:
    """Render machine-readable operation catalog evidence."""
    entries = require_entries(catalog)
    evidence = {
        "schema_version": 1,
        "kind": "kiva-application-operation-catalog-evidence",
        "generated_from": catalog_source_path(catalog),
        **(
            {"projection_basis": catalog["projection_basis"]}
            if "projection_basis" in catalog
            else {}
        ),
        "operation_count": len(entries),
        "operations": [
            {
                "id": entry["id"],
                "surface": entry["surface"],
                "workflow_label": entry["workflow_label"],
                "rust_workflow_kind": entry["rust_workflow_kind"],
                "openapi_contract": entry["openapi_contract"],
                "operation_id": entry["operation_id"],
                "required_scopes": required_scopes(entry),
                **(
                    {"workflow_family": entry["workflow_family"]}
                    if "workflow_family" in entry
                    else {}
                ),
            }
            for entry in entries
        ],
    }
    return json.dumps(evidence, indent=2, sort_keys=True) + "\n"


def family_variant(family: str) -> str:
    """Return the Rust confidential workflow family variant."""
    match family:
        case "additive":
            return "Additive"
        case "differential_privacy":
            return "DifferentialPrivacy"
        case "job":
            return "Job"
        case "participant":
            return "Participant"
        case "shamir":
            return "Shamir"
        case "threshold":
            return "Threshold"
        case _:
            raise ValueError(f"unknown confidential workflow family: {family}")


def workflow_entries(catalog: dict[str, Any], surface: str) -> list[dict[str, Any]]:
    """Return unique workflow entries for one application surface."""
    entries: list[dict[str, Any]] = []
    seen: dict[str, dict[str, Any]] = {}
    for entry in require_entries(catalog):
        if entry.get("surface") != surface:
            continue
        required = {"id", "workflow_label", "rust_workflow_kind"}
        if surface == "confidential":
            required.add("workflow_family")
        missing = sorted(required.difference(entry))
        if missing:
            raise ValueError(
                f"operation catalog entry missing workflow fields: {', '.join(missing)}"
            )
        key = entry["rust_workflow_kind"]
        if not isinstance(key, str) or "::" not in key:
            raise ValueError(f"{entry['id']} rust_workflow_kind must be an enum variant")
        if key in seen:
            previous = seen[key]
            for field in sorted(required.difference({"id"})):
                if entry[field] != previous[field]:
                    raise ValueError(
                        f"{entry['id']} conflicts with prior workflow metadata for {key}"
                    )
            continue
        seen[key] = entry
        entries.append(entry)
    if not entries:
        raise ValueError(f"operation catalog has no {surface} workflow entries")
    return entries


def render_workflow_spec_arm(entry: dict[str, Any]) -> str:
    """Render one workflow-spec match arm."""
    return "\n".join(
        [
            f"        {entry['rust_workflow_kind']} => {{",
            "            ApplicationWorkflowSpec::new(",
            f"                ApplicationWorkflowKind::new({rust_string(entry['workflow_label'])}),",
            "            )",
            "        }",
        ]
    )


def render_workflow_rust_kind_arm(entry: dict[str, Any]) -> str:
    """Render one workflow rust-kind match arm."""
    return "\n".join(
        [
            f"        {entry['rust_workflow_kind']} => {{",
            f"            {rust_string(entry['rust_workflow_kind'])}",
            "        }",
        ]
    )


def render_confidential_family_arm(entry: dict[str, Any]) -> str:
    """Render one confidential workflow-family match arm."""
    family = entry["workflow_family"]
    if not isinstance(family, str):
        raise ValueError(f"{entry['id']} workflow_family must be a string")
    return "\n".join(
        [
            f"        {entry['rust_workflow_kind']} => {{",
            f"            ConfidentialWorkflowFamily::{family_variant(family)}",
            "        }",
        ]
    )


def render_workflow_metadata(catalog: dict[str, Any]) -> str:
    """Render generated workflow metadata lookup functions."""
    source_path = catalog_source_path(catalog)
    pds_entries = workflow_entries(catalog, "pds")
    confidential_entries = workflow_entries(catalog, "confidential")
    analytics_entries = workflow_entries(catalog, "analytics")
    pds_spec_arms = "\n".join(render_workflow_spec_arm(entry) for entry in pds_entries)
    confidential_spec_arms = "\n".join(
        render_workflow_spec_arm(entry) for entry in confidential_entries
    )
    confidential_rust_kind_arms = "\n".join(
        render_workflow_rust_kind_arm(entry) for entry in confidential_entries
    )
    confidential_family_arms = "\n".join(
        render_confidential_family_arm(entry) for entry in confidential_entries
    )
    analytics_spec_arms = "\n".join(
        render_workflow_spec_arm(entry) for entry in analytics_entries
    )
    return f"""//! Generated application workflow metadata.
//!
//! Generated from `{source_path}` by
//! `scripts/ci/generate_application_operation_catalog.py`.

#![allow(
    clippy::match_same_arms,
    clippy::redundant_pub_crate,
    clippy::too_many_lines
)]

use crate::{{
    analytics::AnalyticsWorkflowKind,
    confidential::{{ConfidentialWorkflowFamily, ConfidentialWorkflowKind}},
    pds::PdsWorkflowKind,
    runtime::{{ApplicationWorkflowKind, ApplicationWorkflowSpec}},
}};

pub(crate) const fn analytics_workflow_spec(
    kind: AnalyticsWorkflowKind,
) -> ApplicationWorkflowSpec {{
    match kind {{
{analytics_spec_arms}
    }}
}}

pub(crate) const fn pds_workflow_spec(kind: PdsWorkflowKind) -> ApplicationWorkflowSpec {{
    match kind {{
{pds_spec_arms}
    }}
}}

pub(crate) const fn confidential_workflow_spec(
    kind: ConfidentialWorkflowKind,
) -> ApplicationWorkflowSpec {{
    match kind {{
{confidential_spec_arms}
    }}
}}

pub(crate) const fn confidential_workflow_rust_kind(
    kind: ConfidentialWorkflowKind,
) -> &'static str {{
    match kind {{
{confidential_rust_kind_arms}
    }}
}}

pub(crate) const fn confidential_workflow_family(
    kind: ConfidentialWorkflowKind,
) -> ConfidentialWorkflowFamily {{
    match kind {{
{confidential_family_arms}
    }}
}}
"""


def render_operation_catalog(catalog: dict[str, Any]) -> str:
    """Render the complete Rust operation catalog source."""
    entries = require_entries(catalog)
    source_path = catalog_source_path(catalog)
    rendered_entries = "\n".join(render_entry(entry) for entry in entries)
    return f"""//! Generated application operation registry.
//!
//! Generated from `{source_path}` by
//! `scripts/ci/generate_application_operation_catalog.py`.

use super::super::{{ApplicationOperationSpec, ApplicationOperationSurface}};
use crate::{{
    analytics::AnalyticsWorkflowKind, confidential::ConfidentialWorkflowKind,
    pds::PdsWorkflowKind,
}};

macro_rules! operation_spec {{
    (
        $id:literal,
        $surface:expr,
        $workflow_spec:expr,
        $rust_workflow_kind:literal,
        $openapi_contract:literal,
        $operation_id:literal,
        &[$($scope:literal),* $(,)?] $(,)?
    ) => {{
        ApplicationOperationSpec {{
            id: $id,
            surface: $surface,
            workflow_spec: $workflow_spec,
            rust_workflow_kind: $rust_workflow_kind,
            openapi_contract: $openapi_contract,
            operation_id: $operation_id,
            required_scopes: &[$($scope),*],
        }}
    }};
}}

/// Application operation registry generated from `{source_path}`.
pub(in crate::operation_catalog) const APPLICATION_OPERATION_SPECS: &[ApplicationOperationSpec] = &[
{rendered_entries}
];

#[cfg(test)]
mod tests {{
    use std::collections::HashSet;

    use super::APPLICATION_OPERATION_SPECS;

    #[test]
    fn application_operation_specs_are_non_empty_and_unique() {{
        let mut ids = HashSet::new();
        let mut workflow_labels = HashSet::new();

        for spec in APPLICATION_OPERATION_SPECS {{
            assert!(!spec.id.is_empty());
            assert!(!spec.surface.label().is_empty());
            assert!(!spec.workflow_label().is_empty());
            assert!(!spec.rust_workflow_kind().is_empty());
            assert!(!spec.openapi_contract.is_empty());
            assert!(!spec.operation_id.is_empty());
            assert!(!spec.required_scopes.is_empty());
            assert!(ids.insert(spec.id), "duplicate operation id: {{}}", spec.id);
            assert!(
                workflow_labels.insert(spec.workflow_label()),
                "duplicate workflow label: {{}}",
                spec.workflow_label()
            );
        }}
    }}

}}
"""
