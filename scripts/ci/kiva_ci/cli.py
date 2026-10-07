"""Typed `kiva-ci` command dispatcher derived from CI ledgers."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path
import subprocess
import sys
from typing import Any, Literal

from .evidence import load_evidence_check_manifest
from .manifest import load_yaml, require_list, require_mapping, require_string, require_string_list
from .projection import projection_specs_from_normal_form, validate_projection_spec_compatibility_view


CommandFamily = Literal["generate", "validate", "run"]

DEFAULT_EVIDENCE_MANIFEST = Path("spec/ci/evidence-checks.yaml")
DEFAULT_PROJECTION_SPEC = Path("spec/architecture/projection-specs.source.yaml")
DEFAULT_OPERATION_KERNEL = Path("generated/architecture/operation-kernel.yaml")
DEFAULT_ASSURANCE_GRAPH = Path("generated/architecture/assurance-graph.yaml")
DEFAULT_MECHANISM_PROFILE_INDEX = Path("generated/confidential/mechanism-profile-index.yaml")
DEFAULT_SOURCE_REGISTRY = Path("generated/architecture/source-registry.yaml")

ALIASES: dict[tuple[CommandFamily, str], str] = {
    ("generate", "generated-contract-dto-map"): "openapi-generated-contract-dto-map",
    ("validate", "dto-boundary-ledger"): "openapi-dto-boundary-ledger",
}


class KivaCiError(RuntimeError):
    """Raised when the typed CI command table cannot be resolved."""


@dataclass(frozen=True)
class CiCommand:
    """One typed CI command exposed through the `kiva-ci` entrypoint."""

    family: CommandFamily
    name: str
    script: Path
    description: str
    args: tuple[str, ...] = ()
    alias_for: str | None = None

    @property
    def key(self) -> tuple[CommandFamily, str]:
        """Return the dispatch key."""
        return (self.family, self.name)


def projection_descriptions(root: Path) -> dict[str, str]:
    """Return ProjectionSpec-derived descriptions keyed by script path."""
    path = root / DEFAULT_PROJECTION_SPEC
    if not path.exists():
        return {}
    spec = load_yaml(path, KivaCiError)
    projections = projection_specs_from_normal_form(root, spec, DEFAULT_PROJECTION_SPEC, KivaCiError)
    validate_projection_spec_compatibility_view(spec, DEFAULT_PROJECTION_SPEC, projections, KivaCiError)
    descriptions: dict[str, list[str]] = {}
    for source_key, raw_projection in projections.items():
        projection = require_mapping(
            raw_projection,
            f"{DEFAULT_PROJECTION_SPEC}.projections.{source_key}",
            KivaCiError,
        )
        projection_id = projection.get("projection_id")
        if projection_id is None:
            continue
        label = (
            f"{source_key}: "
            f"{require_string(projection_id, f'{source_key}.projection_id', KivaCiError)}"
        )
        for field in ("projection_generator", "validator"):
            script = projection.get(field)
            if isinstance(script, str) and script:
                descriptions.setdefault(script, []).append(label)
    return {
        script: "; ".join(sorted(set(values)))
        for script, values in descriptions.items()
    }


def command_args_for_check(check: dict[str, Any], check_id: str, family: CommandFamily) -> tuple[str, ...]:
    """Return default command arguments for one evidence check."""
    args = require_string_list(
        check.get("args", []),
        f"{check_id}.args",
        KivaCiError,
        allow_empty=True,
    )
    if family == "generate":
        args = [arg for arg in args if arg != "--check"]
    return tuple(args)


def command_description(
    check: dict[str, Any],
    check_id: str,
    family: CommandFamily,
    projection_by_script: dict[str, str],
) -> str:
    """Return a concise ledger-derived command description."""
    script = require_string(check.get("script"), f"{check_id}.script", KivaCiError)
    projection = projection_by_script.get(script)
    if projection is not None:
        return projection
    check_kind = require_string(check.get("check_kind"), f"{check_id}.check_kind", KivaCiError)
    if family == "generate":
        return f"Generate artifact for evidence check {check_id} ({check_kind})."
    return f"Validate evidence check {check_id} ({check_kind})."


def commands_from_evidence_manifest(root: Path) -> list[CiCommand]:
    """Return generate/validate commands declared by the CI evidence manifest."""
    manifest = load_evidence_check_manifest(root / DEFAULT_EVIDENCE_MANIFEST, KivaCiError)
    projection_by_script = projection_descriptions(root)
    commands: list[CiCommand] = []
    seen: set[tuple[CommandFamily, str]] = set()
    for group_index, raw_group in enumerate(require_list(manifest.get("groups"), "groups", KivaCiError)):
        group = require_mapping(raw_group, f"groups[{group_index}]", KivaCiError)
        group_id = require_string(group.get("id"), f"groups[{group_index}].id", KivaCiError)
        for check_index, raw_check in enumerate(
            require_list(group.get("checks"), f"groups.{group_id}.checks", KivaCiError)
        ):
            check = require_mapping(
                raw_check,
                f"groups.{group_id}.checks[{check_index}]",
                KivaCiError,
            )
            check_id = require_string(
                check.get("id"),
                f"groups.{group_id}.checks[{check_index}].id",
                KivaCiError,
            )
            check_kind = require_string(check.get("check_kind"), f"{check_id}.check_kind", KivaCiError)
            family: CommandFamily = "generate" if check_kind == "generator-check" else "validate"
            key = (family, check_id)
            if key in seen:
                raise KivaCiError(f"duplicate CI command: {family} {check_id}")
            seen.add(key)
            commands.append(
                CiCommand(
                    family=family,
                    name=check_id,
                    script=Path(require_string(check.get("script"), f"{check_id}.script", KivaCiError)),
                    description=command_description(check, check_id, family, projection_by_script),
                    args=command_args_for_check(check, check_id, family),
                )
            )
    return commands


def run_commands_from_manifest(root: Path) -> list[CiCommand]:
    """Return run commands declared by required evidence-check groups."""
    manifest = load_evidence_check_manifest(root / DEFAULT_EVIDENCE_MANIFEST, KivaCiError)
    commands = [
        CiCommand(
            "run",
            "evidence-checks",
            Path("scripts/ci/run_evidence_checks.py"),
            "Run manifest-declared evidence checks; pass --group through args.",
        ),
        CiCommand(
            "run",
            "design-review-suite",
            Path("scripts/ci/run_design_review_suite.py"),
            "Run the manifest-backed design-review suite.",
        ),
        CiCommand(
            "run",
            "generators",
            Path("scripts/ci/run_generators.py"),
            "Run repository generators in dependency order; pass --check through args.",
        ),
    ]
    for index, raw_group in enumerate(
        require_list(manifest.get("required_groups"), "required_groups", KivaCiError)
    ):
        group = require_mapping(raw_group, f"required_groups[{index}]", KivaCiError)
        group_id = require_string(group.get("id"), f"required_groups[{index}].id", KivaCiError)
        claim_boundary = require_string(
            group.get("claim_boundary"),
            f"required_groups[{index}].claim_boundary",
            KivaCiError,
        )
        commands.append(
            CiCommand(
                "run",
                group_id,
                Path("scripts/ci/run_evidence_checks.py"),
                f"Run required evidence-check group for {claim_boundary}.",
                args=("--group", group_id),
            )
        )
    return commands


def apply_aliases(commands: list[CiCommand]) -> list[CiCommand]:
    """Add compatibility aliases over canonical manifest-derived command names."""
    by_key = {command.key: command for command in commands}
    result = list(commands)
    for alias_key, canonical_name in sorted(ALIASES.items()):
        family, alias_name = alias_key
        canonical = by_key.get((family, canonical_name))
        if canonical is None:
            raise KivaCiError(
                f"alias {family} {alias_name} points to missing command {canonical_name}"
            )
        result.append(
            CiCommand(
                family=family,
                name=alias_name,
                script=canonical.script,
                description=f"Alias for {family} {canonical.name}.",
                args=canonical.args,
                alias_for=canonical.name,
            )
        )
    return result


def load_commands(root: Path) -> tuple[CiCommand, ...]:
    """Load the complete typed CI command table."""
    commands = commands_from_evidence_manifest(root)
    commands.extend(run_commands_from_manifest(root))
    commands = apply_aliases(commands)
    duplicates = sorted(
        key for key in {command.key for command in commands}
        if sum(1 for command in commands if command.key == key) > 1
    )
    if duplicates:
        raise KivaCiError(f"duplicate command keys: {duplicates}")
    return tuple(sorted(commands, key=lambda command: command.key))


def parse_args(argv: list[str]) -> argparse.Namespace:
    """Parse `kiva-ci` arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    subcommands = parser.add_subparsers(dest="family", required=True)

    list_parser = subcommands.add_parser("list", help="List known CI commands.")
    list_parser.set_defaults(family="list")

    explain_parser = subcommands.add_parser("explain", help="Explain one generated review-surface row.")
    explain_parser.add_argument("kind", choices=("operation", "claim", "mechanism", "source"))
    explain_parser.add_argument("id")
    explain_parser.set_defaults(family="explain")

    for family in ("generate", "validate", "run"):
        family_parser = subcommands.add_parser(family, help=f"{family} a CI artifact or check.")
        family_parser.add_argument("name")
        if family == "generate":
            family_parser.add_argument(
                "--check",
                action="store_true",
                help="Pass --check to generator commands.",
            )
        family_parser.add_argument("args", nargs=argparse.REMAINDER)
    return parser.parse_args(argv)


def forwarded_args(raw_args: list[str]) -> list[str]:
    """Return arguments after an optional argparse remainder separator."""
    if raw_args and raw_args[0] == "--":
        return raw_args[1:]
    return raw_args


def list_commands(commands: tuple[CiCommand, ...]) -> int:
    """Print available commands."""
    for family in ("generate", "validate", "run"):
        print(f"{family}:")
        for command in (command for command in commands if command.family == family):
            alias = "" if command.alias_for is None else f" (alias for {command.alias_for})"
            print(f"  {command.name}{alias}: {command.description}")
    return 0


def print_mapping(title: str, values: list[tuple[str, object]]) -> None:
    """Print a compact human-readable row."""
    print(title)
    for key, value in values:
        if isinstance(value, list):
            rendered = ", ".join(str(item) for item in value) if value else "-"
        elif value is None:
            rendered = "-"
        else:
            rendered = str(value)
        print(f"  {key}: {rendered}")


def explain_operation(root: Path, row_id: str) -> int:
    """Explain one OperationKernel row."""
    document = load_yaml(root / DEFAULT_OPERATION_KERNEL, KivaCiError)
    for index, raw_operation in enumerate(require_list(document.get("operations"), "operation-kernel.operations", KivaCiError)):
        operation = require_mapping(raw_operation, f"operation-kernel.operations[{index}]", KivaCiError)
        operation_key = require_mapping(operation.get("operation_key"), f"operation-kernel.operations[{index}].operation_key", KivaCiError)
        http_contract = require_mapping(operation.get("http_contract"), f"operation-kernel.operations[{index}].http_contract", KivaCiError)
        candidates = {
            require_string(operation.get("id"), f"operation-kernel.operations[{index}].id", KivaCiError),
            require_string(operation_key.get("operation_id"), f"operation-kernel.operations[{index}].operation_key.operation_id", KivaCiError),
        }
        if row_id not in candidates:
            continue
        app_operation = operation.get("application_operation")
        app_label = None
        if isinstance(app_operation, dict):
            app_label = app_operation.get("workflow_label") or app_operation.get("operation_id")
        family = require_mapping(operation.get("resource_family"), f"operation-kernel.operations[{index}].resource_family", KivaCiError)
        print_mapping(
            f"operation {operation['id']}",
            [
                ("operation_id", operation_key.get("operation_id")),
                ("contract", operation_key.get("openapi_contract")),
                ("method", http_contract.get("method")),
                ("path", http_contract.get("path")),
                ("required_scopes", http_contract.get("required_scopes", [])),
                ("requirements", http_contract.get("requirements", [])),
                ("workflow", app_label),
                ("surface_family", family.get("surface_family")),
                ("pds_resource_family", family.get("pds_resource_family")),
                ("pds_planes", family.get("pds_planes", [])),
            ],
        )
        return 0
    raise KivaCiError(f"unknown operation: {row_id}")


def explain_claim(root: Path, row_id: str) -> int:
    """Explain one AssuranceGraph claim node and its edges."""
    document = load_yaml(root / DEFAULT_ASSURANCE_GRAPH, KivaCiError)
    claim_id = row_id if row_id.startswith("claim:") else f"claim:{row_id}"
    nodes = require_list(document.get("nodes"), "assurance-graph.nodes", KivaCiError)
    node = next(
        (
            require_mapping(raw_node, "assurance-graph.nodes[]", KivaCiError)
            for raw_node in nodes
            if isinstance(raw_node, dict) and raw_node.get("id") == claim_id
        ),
        None,
    )
    if node is None:
        raise KivaCiError(f"unknown claim: {row_id}")
    outgoing = []
    incoming = []
    for raw_edge in require_list(document.get("edges"), "assurance-graph.edges", KivaCiError):
        edge = require_mapping(raw_edge, "assurance-graph.edges[]", KivaCiError)
        if edge.get("from") == claim_id:
            outgoing.append(f"{edge.get('edge_type')} -> {edge.get('to')}")
        if edge.get("to") == claim_id:
            incoming.append(f"{edge.get('from')} -> {edge.get('edge_type')}")
    print_mapping(
        claim_id,
        [
            ("node_type", node.get("node_type")),
            ("status", node.get("status")),
            ("supports", outgoing),
            ("supported_by", incoming),
        ],
    )
    return 0


def explain_mechanism(root: Path, row_id: str) -> int:
    """Explain one PackageProfile or MechanismProfile row."""
    document = load_yaml(root / DEFAULT_MECHANISM_PROFILE_INDEX, KivaCiError)
    for index, raw_package in enumerate(
        require_list(document.get("package_profiles"), "mechanism-profile-index.package_profiles", KivaCiError)
    ):
        package = require_mapping(raw_package, f"mechanism-profile-index.package_profiles[{index}]", KivaCiError)
        maturity = require_mapping(package.get("maturity"), f"package_profiles[{index}].maturity", KivaCiError)
        binding = require_mapping(package.get("capability_binding"), f"package_profiles[{index}].capability_binding", KivaCiError)
        candidates = {
            require_string(package.get("id"), f"package_profiles[{index}].id", KivaCiError),
            require_string(package.get("maturity_mechanism"), f"package_profiles[{index}].maturity_mechanism", KivaCiError),
            require_string(binding.get("id"), f"package_profiles[{index}].capability_binding.id", KivaCiError),
        }
        if row_id not in candidates:
            continue
        verification_refs = [
            require_mapping(raw_ref, "package-profile.verification_kernel_refs[]", KivaCiError).get("id")
            for raw_ref in require_list(package.get("verification_kernel_refs", []), "package-profile.verification_kernel_refs", KivaCiError)
        ]
        print_mapping(
            f"mechanism package {package['id']}",
            [
                ("maturity_mechanism", package.get("maturity_mechanism")),
                ("capability", package.get("capability")),
                ("provider_kind", package.get("provider_kind")),
                ("mechanism_catalog_id", package.get("mechanism_catalog_id")),
                ("maturity_level", maturity.get("level")),
                ("production_claim", maturity.get("production_claim")),
                ("capability_binding", binding.get("id")),
                ("operation_count", binding.get("operation_count")),
                ("assurance_registry_entry", package.get("assurance_registry_entry")),
                ("verification_family", package.get("verification_family")),
                ("verification", verification_refs),
            ],
        )
        return 0
    for index, raw_profile in enumerate(require_list(document.get("profiles"), "mechanism-profile-index.profiles", KivaCiError)):
        profile = require_mapping(raw_profile, f"mechanism-profile-index.profiles[{index}]", KivaCiError)
        provider = require_mapping(profile.get("provider"), f"mechanism-profile-index.profiles[{index}].provider", KivaCiError)
        maturity = require_mapping(profile.get("maturity"), f"mechanism-profile-index.profiles[{index}].maturity", KivaCiError)
        binding = require_mapping(profile.get("capability_binding"), f"mechanism-profile-index.profiles[{index}].capability_binding", KivaCiError)
        candidates = {
            require_string(profile.get("id"), f"mechanism-profile-index.profiles[{index}].id", KivaCiError),
            require_string(profile.get("maturity_mechanism"), f"mechanism-profile-index.profiles[{index}].maturity_mechanism", KivaCiError),
            require_string(binding.get("id"), f"mechanism-profile-index.profiles[{index}].capability_binding.id", KivaCiError),
        }
        if row_id not in candidates:
            continue
        verification_refs = [
            require_mapping(raw_ref, "mechanism-profile.verification_kernel_refs[]", KivaCiError).get("id")
            for raw_ref in require_list(profile.get("verification_kernel_refs", []), "mechanism-profile.verification_kernel_refs", KivaCiError)
        ]
        operations = require_list(profile.get("operation_refs", []), "mechanism-profile.operation_refs", KivaCiError)
        print_mapping(
            f"mechanism {profile['id']}",
            [
                ("maturity_mechanism", profile.get("maturity_mechanism")),
                ("technique", profile.get("technique")),
                ("provider_kind", provider.get("kind")),
                ("provider_boundary", provider.get("boundary_type")),
                ("required_capability", provider.get("required_capability")),
                ("maturity_level", maturity.get("level")),
                ("production_claim", maturity.get("production_claim")),
                ("capability_binding", binding.get("id")),
                ("operation_count", len(operations)),
                ("verification", verification_refs),
            ],
        )
        return 0
    raise KivaCiError(f"unknown mechanism: {row_id}")


def explain_source(root: Path, row_id: str) -> int:
    """Explain one SourceRegistry row by id or artifact path."""
    document = load_yaml(root / DEFAULT_SOURCE_REGISTRY, KivaCiError)
    for index, raw_source in enumerate(require_list(document.get("sources"), "source-registry.sources", KivaCiError)):
        source = require_mapping(raw_source, f"source-registry.sources[{index}]", KivaCiError)
        artifacts = require_string_list(
            source.get("artifacts", []),
            f"source-registry.sources[{index}].artifacts",
            KivaCiError,
            allow_empty=True,
        )
        if row_id != source.get("id") and row_id not in artifacts:
            continue
        print_mapping(
            f"source {source['id']}",
            [
                ("layer", source.get("source_layer")),
                ("kind", source.get("source_kind")),
                ("owner", source.get("owner")),
                ("schema", source.get("schema")),
                ("purpose", source.get("purpose")),
                ("artifacts", artifacts),
            ],
        )
        return 0
    raise KivaCiError(f"unknown source: {row_id}")


def explain_review_surface(root: Path, kind: str, row_id: str) -> int:
    """Explain one generated review-surface row."""
    if kind == "operation":
        return explain_operation(root, row_id)
    if kind == "claim":
        return explain_claim(root, row_id)
    if kind == "mechanism":
        return explain_mechanism(root, row_id)
    if kind == "source":
        return explain_source(root, row_id)
    raise KivaCiError(f"unsupported explain kind: {kind}")


def command_by_key(commands: tuple[CiCommand, ...]) -> dict[tuple[CommandFamily, str], CiCommand]:
    """Return commands keyed by family and name."""
    return {command.key: command for command in commands}


def run_command(root: Path, command: CiCommand, args: list[str]) -> int:
    """Run one registered CI command."""
    script = root / command.script
    if not script.exists():
        raise FileNotFoundError(f"registered CI command script is missing: {command.script}")
    return subprocess.run(
        [sys.executable, script.as_posix(), *command.args, *args],
        cwd=root,
        check=False,
    ).returncode


def main(argv: list[str] | None = None) -> int:
    """Run the typed CI dispatcher."""
    parsed = parse_args(sys.argv[1:] if argv is None else argv)
    root = Path.cwd().resolve()
    if parsed.family == "explain":
        return explain_review_surface(root, parsed.kind, parsed.id)
    commands = load_commands(root)
    if parsed.family == "list":
        return list_commands(commands)
    family = parsed.family
    if family not in {"generate", "validate", "run"}:
        raise ValueError(f"unsupported command family: {family}")
    command = command_by_key(commands).get((family, parsed.name))
    if command is None:
        names = ", ".join(command.name for command in commands if command.family == family)
        raise KivaCiError(f"unknown command {family} {parsed.name}; expected one of: {names}")
    args = forwarded_args(parsed.args)
    if family == "generate" and parsed.check:
        args = [*args, "--check"]
    return run_command(root, command, args)


if __name__ == "__main__":
    raise SystemExit(main())
