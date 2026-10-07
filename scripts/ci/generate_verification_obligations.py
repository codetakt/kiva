#!/usr/bin/env python3
"""Generate the tool-neutral verification obligation index."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator

from kiva_ci.architecture_compiler import dump_yaml
from kiva_ci.manifest import (
    load_json,
    load_yaml,
    require_list,
    require_mapping,
    require_string,
)
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts


DEFAULT_SOURCE = Path("spec/verification/verification-obligations.source.yaml")
DEFAULT_OUTPUT = Path("generated/verification/verification-obligations.yaml")
DEFAULT_MARKDOWN = Path("docs/architecture/generated/verification-obligations.md")
DEFAULT_SCHEMA = Path("spec/verification/schemas/verification-obligations.schema.json")


class VerificationObligationsError(RuntimeError):
    """Raised when verification obligations cannot be generated."""


def discharge_count(value: Any, label: str) -> int:
    """Return the number of discharge rows for one method."""
    return len(require_list(value, label, VerificationObligationsError))


def discharge_rows(
    obligation_id: str,
    kernel: dict[str, Any],
    method_fields: dict[str, Any],
) -> list[dict[str, Any]]:
    """Return discharge rows for one verification obligation."""
    rows: list[dict[str, Any]] = []
    for method, field in method_fields.items():
        field_name = require_string(field, f"obligation_model.method_fields.{method}", VerificationObligationsError)
        values = kernel.get(field_name, [])
        count = discharge_count(values, field_name)
        if count == 0:
            continue
        rows.append(
            {
                "method": str(method),
                "receipt_id": f"verification-receipt.{obligation_id}.{method}",
                "source_field": field_name,
                "count": count,
                "status": "declared",
            }
        )
    return rows


def required_methods(kernel: dict[str, Any]) -> list[str]:
    """Return required discharge methods for one kernel."""
    return [
        str(value)
        for value in require_list(
            kernel.get("required_evidence_kinds"),
            "verified_kernel.required_evidence_kinds",
            VerificationObligationsError,
        )
    ]


def obligation_document(
    source_path: Path,
    output_path: Path,
    markdown_path: Path,
) -> dict[str, Any]:
    """Return the generated VerificationObligation index."""
    source = load_yaml(source_path, VerificationObligationsError)
    if source.get("kind") != "kiva-verification-obligations-source":
        raise VerificationObligationsError(f"{source_path}.kind must be kiva-verification-obligations-source")
    source_ledger = Path(
        require_string(source.get("source_ledger"), f"{source_path}.source_ledger", VerificationObligationsError)
    )
    kernels = load_yaml(source_ledger, VerificationObligationsError)
    obligation_model = require_mapping(
        source.get("obligation_model"),
        f"{source_path}.obligation_model",
        VerificationObligationsError,
    )
    method_fields = require_mapping(
        obligation_model.get("method_fields"),
        f"{source_path}.obligation_model.method_fields",
        VerificationObligationsError,
    )

    obligations: list[dict[str, Any]] = []
    for index, value in enumerate(
        require_list(kernels.get("kernels"), f"{source_ledger}.kernels", VerificationObligationsError)
    ):
        kernel = require_mapping(value, f"{source_ledger}.kernels[{index}]", VerificationObligationsError)
        obligation_id = require_string(kernel.get("id"), f"{source_ledger}.kernels[{index}].id", VerificationObligationsError)
        methods = required_methods(kernel)
        discharges = discharge_rows(obligation_id, kernel, method_fields)
        declared_methods = {row["method"] for row in discharges}
        missing = sorted(set(methods).difference(declared_methods))
        obligations.append(
            {
                "id": obligation_id,
                "root_type": "VerificationObligation",
                "status": require_string(kernel.get("status"), f"{source_ledger}.kernels[{index}].status", VerificationObligationsError),
                "mechanism": require_string(kernel.get("mechanism"), f"{source_ledger}.kernels[{index}].mechanism", VerificationObligationsError),
                "boundary_property": require_string(
                    kernel.get("claim_boundary"),
                    f"{source_ledger}.kernels[{index}].claim_boundary",
                    VerificationObligationsError,
                ),
                "required_discharge_methods": methods,
                "declared_discharges": discharges,
                "missing_required_discharge_methods": missing,
                "complete": not missing,
            }
        )

    incomplete = [obligation for obligation in obligations if not obligation["complete"]]
    return {
        "schema_version": 1,
        "kind": "kiva-verification-obligation-index",
        "status": "active",
        "description": (
            "Generated tool-neutral verification obligation index. Boundary "
            "properties are primary; Kani, Tamarin, F*, Rust tests, implementation "
            "anchors, and assurance-registry entries are discharge methods."
        ),
        "generated_from": [
            source_path.as_posix(),
            source_ledger.as_posix(),
        ],
        "generated_artifacts": {
            "obligation_index": output_path.as_posix(),
            "markdown": markdown_path.as_posix(),
        },
        "model": {
            "root_type": "VerificationObligation",
            "property_first": True,
            "tool_neutral": True,
            "discharge_methods": list(
                require_list(
                    obligation_model.get("discharge_methods"),
                    f"{source_path}.obligation_model.discharge_methods",
                    VerificationObligationsError,
                )
            ),
        },
        "summary": {
            "obligation_count": len(obligations),
            "complete_count": len(obligations) - len(incomplete),
            "incomplete_count": len(incomplete),
            "receipt_required_count": sum(
                len(obligation["declared_discharges"])
                for obligation in obligations
            ),
        },
        "obligations": obligations,
    }


def validate_schema(document: dict[str, Any], schema_path: Path) -> None:
    """Validate the generated verification obligation index."""
    schema = load_json(schema_path, VerificationObligationsError)
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(document),
        key=lambda error: list(error.path),
    )
    if errors:
        details = "\n".join(
            f"- {'/'.join(str(part) for part in error.path) or '<root>'}: {error.message}"
            for error in errors
        )
        raise VerificationObligationsError(f"VerificationObligation index violates {schema_path}:\n{details}")


def markdown_cell(value: object) -> str:
    """Render a Markdown table cell."""
    return str(value).replace("|", "\\|").replace("\n", " ")


def render_markdown(document: dict[str, Any]) -> str:
    """Render a compact verification obligation table."""
    rows = [
        "| Obligation | Mechanism | Required Methods | Discharges | Receipts | Complete |",
        "| --- | --- | --- | ---: | ---: | --- |",
    ]
    for value in require_list(document.get("obligations"), "obligations", VerificationObligationsError):
        obligation = require_mapping(value, "obligations[]", VerificationObligationsError)
        rows.append(
            "| "
            + " | ".join(
                [
                    f"`{markdown_cell(obligation['id'])}`",
                    f"`{markdown_cell(obligation['mechanism'])}`",
                    ", ".join(f"`{markdown_cell(method)}`" for method in obligation["required_discharge_methods"]),
                    str(len(obligation["declared_discharges"])),
                    str(len([row for row in obligation["declared_discharges"] if "receipt_id" in row])),
                    "`yes`" if obligation["complete"] else "`no`",
                ]
            )
            + " |"
        )
    return f"""# Generated Verification Obligations

This file is generated by `scripts/ci/generate_verification_obligations.py`.

Each row names a boundary property first. Tool outputs are discharge methods
for that property, not independent sources of the claim.

{chr(10).join(rows)}
"""


def main() -> int:
    """Generate or validate the VerificationObligation index."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--markdown-out", type=Path, default=DEFAULT_MARKDOWN)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    document = obligation_document(args.source, args.out, args.markdown_out)
    validate_schema(document, args.schema)
    rendered_yaml = dump_yaml(document)
    rendered_markdown = render_markdown(document)
    command_hint = "scripts/ci/generate_verification_obligations.py"
    try:
        write_or_check_generated_artifacts(
            [
                GeneratedArtifact(args.out, rendered_yaml),
                GeneratedArtifact(args.markdown_out, rendered_markdown),
            ],
            check=args.check,
            command_hint=command_hint,
            error_cls=VerificationObligationsError,
        )
    except VerificationObligationsError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(f"validated generated VerificationObligation index: {args.out}, {args.markdown_out}")
        return 0
    print(f"generated VerificationObligation index: {args.out}, {args.markdown_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
