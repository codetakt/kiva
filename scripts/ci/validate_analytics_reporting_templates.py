#!/usr/bin/env python3
"""Validate analytics reference reporting deployment templates."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import yaml
from jsonschema import Draft202012Validator, FormatChecker


DEFAULT_CATALOG = Path("spec/analytics/reporting-templates.yaml")
DEFAULT_SCHEMA = Path("spec/analytics/schemas/reporting-template-catalog.schema.json")
DEFAULT_KERNEL = Path("generated/architecture/operation-kernel.yaml")
ANALYTICS_CONTRACT = "spec/openapi/kiva-analytics.yaml"
EXACT_OPERATION_SEQUENCE = [
    "getAnalyticsProducts",
    "getAnalyticsProduct",
    "getAnalyticsProductAggregate",
]
EXACT_RELEASE_KINDS = {
    "threshold-release",
    "differential-privacy-release",
}
REQUIRED_DEPLOYMENT_KINDS = {
    "school-district",
    "corporate-training",
}
KIVA_AUTHORIZATION_SCOPES = {
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
    "analytics:read",
}
REQUIRED_SCOPE_DECLARATIONS = (
    "do not extend",
    "39274-1-1:2025",
    "do not host a bi product",
    "not legal advice",
)


class AnalyticsReportingTemplateError(RuntimeError):
    """Raised when a reporting template violates the BI2 boundary."""


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise AnalyticsReportingTemplateError(f"{path} root must be a mapping")
    return value


def load_json(path: Path) -> dict[str, Any]:
    """Load a JSON mapping."""
    with path.open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise AnalyticsReportingTemplateError(f"{path} root must be a mapping")
    return value


def format_validation_error(error: Any) -> str:
    """Format a JSON Schema error with a stable path."""
    path = "$"
    for part in error.absolute_path:
        path += f"[{part}]" if isinstance(part, int) else f".{part}"
    return f"- {path}: {error.message}"


def validate_schema(catalog: dict[str, Any], schema_path: Path) -> None:
    """Validate the catalog against its JSON Schema."""
    schema = load_json(schema_path)
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    errors = sorted(validator.iter_errors(catalog), key=lambda error: list(error.absolute_path))
    if errors:
        details = "\n".join(format_validation_error(error) for error in errors)
        raise AnalyticsReportingTemplateError(
            f"analytics reporting template catalog violates {schema_path}:\n{details}",
        )


def analytics_kernel_operations(kernel_path: Path) -> dict[str, dict[str, Any]]:
    """Return analytics operations keyed by OpenAPI operationId."""
    kernel = load_yaml(kernel_path)
    operations = kernel.get("operations")
    if not isinstance(operations, list):
        raise AnalyticsReportingTemplateError(f"{kernel_path}.operations must be a list")
    result: dict[str, dict[str, Any]] = {}
    for operation in operations:
        if not isinstance(operation, dict):
            continue
        operation_key = operation.get("operation_key")
        if not isinstance(operation_key, dict):
            continue
        if operation_key.get("openapi_contract") != ANALYTICS_CONTRACT:
            continue
        operation_id = operation_key.get("operation_id")
        if not isinstance(operation_id, str) or not operation_id:
            raise AnalyticsReportingTemplateError(
                f"{kernel_path} contains an analytics operation without operation_id",
            )
        if operation_id in result:
            raise AnalyticsReportingTemplateError(
                f"{kernel_path} contains duplicate analytics operation {operation_id}",
            )
        result[operation_id] = operation
    return result


def validate_scope_declarations(catalog: dict[str, Any]) -> None:
    """Require the three BI2 non-claim declarations."""
    scope = str(catalog.get("scope", "")).lower()
    missing = [declaration for declaration in REQUIRED_SCOPE_DECLARATIONS if declaration not in scope]
    if missing:
        raise AnalyticsReportingTemplateError(
            "catalog scope is missing non-extension declarations: " + ", ".join(missing),
        )


def validate_template(
    template: dict[str, Any],
    kernel_operations: dict[str, dict[str, Any]],
) -> None:
    """Validate one template against the BI1 operation and scope boundary."""
    template_id = template["id"]
    operation_rows = template["operation_sequence"]
    sequence = [row["sequence"] for row in operation_rows]
    operation_ids = [row["operation_id"] for row in operation_rows]
    if sequence != [1, 2, 3]:
        raise AnalyticsReportingTemplateError(
            f"{template_id}.operation_sequence must be ordered 1, 2, 3",
        )
    if operation_ids != EXACT_OPERATION_SEQUENCE:
        raise AnalyticsReportingTemplateError(
            f"{template_id} must use the exact BI1 operation sequence {EXACT_OPERATION_SEQUENCE}",
        )
    for operation_id in operation_ids:
        operation = kernel_operations.get(operation_id)
        if operation is None:
            raise AnalyticsReportingTemplateError(
                f"{template_id} references unknown OperationKernel operation {operation_id}",
            )
        http_contract = operation.get("http_contract")
        if not isinstance(http_contract, dict):
            raise AnalyticsReportingTemplateError(
                f"OperationKernel operation {operation_id} has no http_contract",
            )
        if http_contract.get("required_scopes") != ["analytics:read"]:
            raise AnalyticsReportingTemplateError(
                f"OperationKernel operation {operation_id} must require only analytics:read",
            )

    client = template["client"]
    if client["required_scopes"] != ["analytics:read"]:
        raise AnalyticsReportingTemplateError(
            f"{template_id}.client.required_scopes must contain only analytics:read",
        )
    forbidden_ignored = set(client["ignored_scopes"]) & KIVA_AUTHORIZATION_SCOPES
    if forbidden_ignored:
        raise AnalyticsReportingTemplateError(
            f"{template_id} ignores Kiva authorization scopes: {sorted(forbidden_ignored)}",
        )
    if set(template["data_products"]["release_kinds"]) != EXACT_RELEASE_KINDS:
        raise AnalyticsReportingTemplateError(
            f"{template_id} must classify both governed aggregate release kinds",
        )


def validate_catalog(
    catalog: dict[str, Any],
    *,
    schema: Path,
    operation_kernel: Path,
) -> dict[str, Any]:
    """Validate the BI2 catalog and return a stable report."""
    validate_schema(catalog, schema)
    validate_scope_declarations(catalog)
    kernel_operations = analytics_kernel_operations(operation_kernel)
    templates = catalog["templates"]
    template_ids = [template["id"] for template in templates]
    if len(template_ids) != len(set(template_ids)):
        raise AnalyticsReportingTemplateError("templates.id must be unique")
    deployment_kinds = {template["deployment_kind"] for template in templates}
    if deployment_kinds != REQUIRED_DEPLOYMENT_KINDS:
        raise AnalyticsReportingTemplateError(
            "catalog deployment kinds must be " + ", ".join(sorted(REQUIRED_DEPLOYMENT_KINDS)),
        )
    for template in templates:
        validate_template(template, kernel_operations)
    return {
        "schema_version": 1,
        "kind": "analytics-reporting-template-validation",
        "status": "passed",
        "catalog": DEFAULT_CATALOG.as_posix(),
        "template_count": len(templates),
        "templates": sorted(template_ids),
        "deployment_kinds": sorted(deployment_kinds),
        "operation_ids": EXACT_OPERATION_SEQUENCE,
        "required_scopes": ["analytics:read"],
    }


def main() -> int:
    """Run analytics reporting template validation."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--catalog", type=Path, default=DEFAULT_CATALOG)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument("--operation-kernel", type=Path, default=DEFAULT_KERNEL)
    parser.add_argument("--json-report", type=Path)
    args = parser.parse_args()

    report = validate_catalog(
        load_yaml(args.catalog),
        schema=args.schema,
        operation_kernel=args.operation_kernel,
    )
    if args.json_report is not None:
        args.json_report.parent.mkdir(parents=True, exist_ok=True)
        args.json_report.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    print(
        "validated analytics reporting templates: "
        f"{report['template_count']} templates, "
        f"operations={','.join(report['operation_ids'])}",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
