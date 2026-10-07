#!/usr/bin/env python3
"""Generate the confidential CapabilityBinding index."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.architecture_compiler import dump_yaml
from kiva_ci.ledger import validate_json_schema
from kiva_ci.manifest import (
    load_json,
    load_yaml,
    require_list,
    require_mapping,
    require_string,
    require_string_list,
)
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts


DEFAULT_SOURCE = Path("spec/confidential/capability-bindings.source.yaml")
DEFAULT_OUTPUT = Path("generated/confidential/capability-binding-index.yaml")
DEFAULT_MARKDOWN = Path("docs/architecture/generated/confidential-capability-binding-index.md")
DEFAULT_SCHEMA = Path("spec/confidential/schemas/capability-binding-index.schema.json")
DEFAULT_TAXONOMY = Path("spec/confidential/capability-taxonomy.source.yaml")


class CapabilityBindingIndexError(RuntimeError):
    """Raised when CapabilityBinding generation fails."""


def operation_ref(operation: dict[str, Any], label: str) -> dict[str, str]:
    """Return a compact OperationKernel reference."""
    app = require_mapping(
        operation.get("application_operation"),
        f"{label}.application_operation",
        CapabilityBindingIndexError,
    )
    route = require_mapping(operation.get("route_key"), f"{label}.route_key", CapabilityBindingIndexError)
    operation_key = require_mapping(
        operation.get("operation_key"),
        f"{label}.operation_key",
        CapabilityBindingIndexError,
    )
    operation_id = route.get("operation_id", operation_key.get("operation_id"))
    return {
        "id": require_string(operation.get("id"), f"{label}.id", CapabilityBindingIndexError),
        "operation_id": require_string(operation_id, f"{label}.operation_id", CapabilityBindingIndexError),
        "method": require_string(route.get("method"), f"{label}.route_key.method", CapabilityBindingIndexError),
        "path": require_string(route.get("path"), f"{label}.route_key.path", CapabilityBindingIndexError),
        "workflow_family": require_string(
            app.get("workflow_family"),
            f"{label}.application_operation.workflow_family",
            CapabilityBindingIndexError,
        ),
    }


def operation_refs_by_workflow_family(operation_kernel: dict[str, Any]) -> dict[str, list[dict[str, str]]]:
    """Return OperationKernel rows grouped by application workflow family."""
    grouped: dict[str, list[dict[str, str]]] = {}
    for index, value in enumerate(
        require_list(operation_kernel.get("operations"), "operation_kernel.operations", CapabilityBindingIndexError)
    ):
        operation = require_mapping(value, f"operation_kernel.operations[{index}]", CapabilityBindingIndexError)
        app = operation.get("application_operation")
        if app is None:
            continue
        app_map = require_mapping(
            app,
            f"operation_kernel.operations[{index}].application_operation",
            CapabilityBindingIndexError,
        )
        workflow_family = app_map.get("workflow_family")
        if not isinstance(workflow_family, str) or not workflow_family:
            continue
        grouped.setdefault(workflow_family, []).append(
            operation_ref(operation, f"operation_kernel.operations[{index}]")
        )
    return {
        family: sorted(refs, key=lambda ref: (ref["path"], ref["method"], ref["operation_id"]))
        for family, refs in grouped.items()
    }


def capability_taxonomy(taxonomy_path: Path) -> dict[str, dict[str, Any]]:
    """Return capability defaults keyed by capability id."""
    taxonomy = load_yaml(taxonomy_path, CapabilityBindingIndexError)
    if taxonomy.get("schema_version") != 1:
        raise CapabilityBindingIndexError(f"{taxonomy_path}.schema_version must be 1")
    if taxonomy.get("kind") != "kiva-confidential-capability-taxonomy-source":
        raise CapabilityBindingIndexError(
            f"{taxonomy_path}.kind must be kiva-confidential-capability-taxonomy-source"
        )
    if taxonomy.get("status") != "active":
        raise CapabilityBindingIndexError(f"{taxonomy_path}.status must be active")
    capabilities: dict[str, dict[str, Any]] = {}
    for index, value in enumerate(
        require_list(taxonomy.get("capabilities"), f"{taxonomy_path}.capabilities", CapabilityBindingIndexError)
    ):
        row = require_mapping(value, f"{taxonomy_path}.capabilities[{index}]", CapabilityBindingIndexError)
        capability_id = require_string(row.get("id"), f"{taxonomy_path}.capabilities[{index}].id", CapabilityBindingIndexError)
        if capability_id in capabilities:
            raise CapabilityBindingIndexError(f"duplicate capability taxonomy id: {capability_id}")
        capabilities[capability_id] = {
            "binding_kind": require_string(
                row.get("binding_kind"),
                f"{taxonomy_path}.capabilities[{index}].binding_kind",
                CapabilityBindingIndexError,
            ),
            "workflow_families": require_string_list(
                row.get("workflow_families"),
                f"{taxonomy_path}.capabilities[{index}].workflow_families",
                CapabilityBindingIndexError,
            ),
        }
    if not capabilities:
        raise CapabilityBindingIndexError(f"{taxonomy_path}.capabilities must not be empty")
    return capabilities


def binding_affinity(row: dict[str, Any], label: str, taxonomy: dict[str, dict[str, Any]]) -> tuple[str, list[str]]:
    """Return binding kind and workflow families for one CapabilityBinding row."""
    capability = require_string(row.get("capability"), f"{label}.capability", CapabilityBindingIndexError)
    defaults = taxonomy.get(capability)
    if defaults is None:
        raise CapabilityBindingIndexError(f"{label}.capability is not in taxonomy: {capability}")
    binding_kind = row.get("binding_kind", defaults["binding_kind"])
    if not isinstance(binding_kind, str) or not binding_kind:
        raise CapabilityBindingIndexError(f"{label}.binding_kind must be a non-empty string")
    if "operation_selector" not in row:
        return binding_kind, list(defaults["workflow_families"])
    selector = require_mapping(row.get("operation_selector"), f"{label}.operation_selector", CapabilityBindingIndexError)
    return (
        binding_kind,
        require_string_list(
            selector.get("workflow_families"),
            f"{label}.operation_selector.workflow_families",
            CapabilityBindingIndexError,
        ),
    )


def binding_document(source_path: Path, output_path: Path, markdown_path: Path) -> dict[str, Any]:
    """Return the generated CapabilityBinding index."""
    source = load_yaml(source_path, CapabilityBindingIndexError)
    if source.get("schema_version") != 1:
        raise CapabilityBindingIndexError(f"{source_path}.schema_version must be 1")
    if source.get("kind") != "kiva-confidential-capability-bindings-source":
        raise CapabilityBindingIndexError(
            f"{source_path}.kind must be kiva-confidential-capability-bindings-source"
        )
    if source.get("status") != "active":
        raise CapabilityBindingIndexError(f"{source_path}.status must be active")
    operation_kernel_path = Path(
        require_string(source.get("operation_kernel"), f"{source_path}.operation_kernel", CapabilityBindingIndexError)
    )
    taxonomy_path = Path(
        require_string(
            source.get("taxonomy_source", DEFAULT_TAXONOMY.as_posix()),
            f"{source_path}.taxonomy_source",
            CapabilityBindingIndexError,
        )
    )
    operation_kernel = load_yaml(operation_kernel_path, CapabilityBindingIndexError)
    refs_by_family = operation_refs_by_workflow_family(operation_kernel)
    taxonomy = capability_taxonomy(taxonomy_path)

    bindings: list[dict[str, Any]] = []
    seen_maturity: set[str] = set()
    for index, value in enumerate(require_list(source.get("bindings"), f"{source_path}.bindings", CapabilityBindingIndexError)):
        row = require_mapping(value, f"{source_path}.bindings[{index}]", CapabilityBindingIndexError)
        maturity = require_string(
            row.get("maturity_mechanism"),
            f"{source_path}.bindings[{index}].maturity_mechanism",
            CapabilityBindingIndexError,
        )
        if maturity in seen_maturity:
            raise CapabilityBindingIndexError(f"duplicate CapabilityBinding maturity_mechanism: {maturity}")
        seen_maturity.add(maturity)
        capability = require_string(
            row.get("capability"),
            f"{source_path}.bindings[{index}].capability",
            CapabilityBindingIndexError,
        )
        binding_kind, workflow_families = binding_affinity(
            row,
            f"{source_path}.bindings[{index}]",
            taxonomy,
        )
        missing_families = sorted(set(workflow_families).difference(refs_by_family))
        if missing_families:
            raise CapabilityBindingIndexError(
                f"{maturity} references missing OperationKernel workflow families: {missing_families}"
            )
        operation_refs = [
            ref
            for family in workflow_families
            for ref in refs_by_family[family]
        ]
        if not operation_refs:
            raise CapabilityBindingIndexError(f"{maturity} must bind at least one operation")
        bindings.append(
            {
                "id": require_string(row.get("id"), f"{source_path}.bindings[{index}].id", CapabilityBindingIndexError),
                "root_type": "CapabilityBinding",
                "maturity_mechanism": maturity,
                "capability": capability,
                "binding_kind": binding_kind,
                "operation_selector": {"workflow_families": workflow_families},
                "operation_count": len(operation_refs),
                "operation_refs": operation_refs,
                "boundary_note": require_string(
                    row.get("boundary_note"),
                    f"{source_path}.bindings[{index}].boundary_note",
                    CapabilityBindingIndexError,
                ),
            }
        )

    return {
        "schema_version": 1,
        "kind": "kiva-confidential-capability-binding-index",
        "status": "active",
        "description": require_string(source.get("description"), f"{source_path}.description", CapabilityBindingIndexError),
        "generated_from": [source_path.as_posix(), taxonomy_path.as_posix(), operation_kernel_path.as_posix()],
        "generated_artifacts": {
            "binding_index": output_path.as_posix(),
            "markdown": markdown_path.as_posix(),
        },
        "binding_model": require_mapping(
            source.get("binding_model"),
            f"{source_path}.binding_model",
            CapabilityBindingIndexError,
        ),
        "summary": {
            "binding_count": len(bindings),
            "operation_ref_count": sum(binding["operation_count"] for binding in bindings),
        },
        "bindings": bindings,
    }


def validate_schema(document: dict[str, Any], schema_path: Path) -> None:
    """Validate the generated CapabilityBinding index."""
    validate_json_schema(
        document,
        load_json(schema_path, CapabilityBindingIndexError),
        label=schema_path.as_posix(),
        error_cls=CapabilityBindingIndexError,
    )


def markdown_cell(value: object) -> str:
    """Render a Markdown table cell."""
    return str(value).replace("|", "\\|").replace("\n", " ")


def render_markdown(document: dict[str, Any]) -> str:
    """Render a compact CapabilityBinding review table."""
    rows = [
        "| Binding | Maturity Mechanism | Kind | Workflow Families | Operations |",
        "| --- | --- | --- | --- | ---: |",
    ]
    for value in require_list(document.get("bindings"), "bindings", CapabilityBindingIndexError):
        binding = require_mapping(value, "bindings[]", CapabilityBindingIndexError)
        selector = require_mapping(binding.get("operation_selector"), "binding.operation_selector", CapabilityBindingIndexError)
        rows.append(
            "| "
            + " | ".join(
                [
                    f"`{markdown_cell(binding['id'])}`",
                    f"`{markdown_cell(binding['maturity_mechanism'])}`",
                    f"`{markdown_cell(binding['binding_kind'])}`",
                    ", ".join(f"`{markdown_cell(family)}`" for family in selector["workflow_families"]),
                    str(binding["operation_count"]),
                ]
            )
            + " |"
        )
    return f"""# Generated Confidential CapabilityBinding Index

This file is generated by `scripts/ci/generate_confidential_capability_binding_index.py`.

`CapabilityBinding` owns the mechanism-to-operation affinity used by
`MechanismProfile`. It keeps dedicated mechanism APIs and generic confidential
job/participant orchestration explicit.

{chr(10).join(rows)}
"""


def main() -> int:
    """Generate or validate the confidential CapabilityBinding index."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--markdown-out", type=Path, default=DEFAULT_MARKDOWN)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    document = binding_document(args.source, args.out, args.markdown_out)
    validate_schema(document, args.schema)
    rendered_yaml = dump_yaml(document)
    rendered_markdown = render_markdown(document)
    command_hint = "scripts/ci/generate_confidential_capability_binding_index.py"
    try:
        write_or_check_generated_artifacts(
            [
                GeneratedArtifact(args.out, rendered_yaml),
                GeneratedArtifact(args.markdown_out, rendered_markdown),
            ],
            check=args.check,
            command_hint=command_hint,
            error_cls=CapabilityBindingIndexError,
        )
    except CapabilityBindingIndexError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(f"validated generated CapabilityBinding index: {args.out}, {args.markdown_out}")
        return 0
    print(f"generated CapabilityBinding index: {args.out}, {args.markdown_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
