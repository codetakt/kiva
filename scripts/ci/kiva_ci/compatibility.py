"""Compatibility-surface ledger validation helpers."""

from __future__ import annotations

import os
from pathlib import Path
import subprocess
from typing import Any, TypeVar

from .manifest import (
    load_yaml,
    relative_path,
    require_list,
    require_mapping,
    require_string,
    require_string_list,
    validation_error,
)


ErrorT = TypeVar("ErrorT", bound=RuntimeError)

RUST_USE_PREFIXES = ("use ", "pub use ", "pub(super) use ", "pub(crate) use ")
PUBLIC_RUST_USE_PREFIXES = ("pub use ", "pub(super) use ", "pub(crate) use ")
RUST_USE_SCAN_MANIFEST = Path("tools/kiva-rust-use-scan/Cargo.toml")
RUST_USE_SCAN_CACHE: dict[tuple[str, ...], set[str]] = {}


def strip_rust_line_comment(line: str) -> str:
    """Return a Rust source line without a trailing line comment."""
    return line.split("//", 1)[0].rstrip()


def rust_use_statements(text: str) -> set[str]:
    """Return normalized Rust use/re-export statements from source text."""
    statements: set[str] = set()
    current: list[str] = []
    collecting = False
    for raw_line in text.splitlines():
        stripped = strip_rust_line_comment(raw_line).strip()
        if not stripped:
            continue
        if not collecting and not stripped.startswith(RUST_USE_PREFIXES):
            continue
        if not collecting:
            current = []
            collecting = True
        current.append(stripped)
        if ";" not in stripped:
            continue
        statement = " ".join(" ".join(current).split())
        statements.add(statement)
        current = []
        collecting = False
    return statements


def public_use_lines(text: str) -> set[str]:
    """Return public use/re-export lines in a Rust source file."""
    return {
        statement
        for statement in rust_use_statements(text)
        if statement.startswith(PUBLIC_RUST_USE_PREFIXES)
    }


def rust_ast_use_statements(
    root: Path,
    paths: list[Path],
    label: str,
    error_cls: type[ErrorT],
) -> set[str]:
    """Return Rust use statements extracted by the AST scanner."""
    if not paths:
        return set()
    manifest = root / RUST_USE_SCAN_MANIFEST
    if not manifest.exists():
        raise validation_error(error_cls, f"Rust use AST scanner is missing: {RUST_USE_SCAN_MANIFEST}")
    cache_key = tuple(sorted(path.as_posix() for path in paths))
    cached = RUST_USE_SCAN_CACHE.get(cache_key)
    if cached is not None:
        return cached
    scanner_override = os.environ.get("KIVA_RUST_USE_SCAN")
    if scanner_override:
        scanner = Path(scanner_override)
        if not scanner.is_file():
            raise validation_error(error_cls, f"{label} Rust use AST scanner override is missing: {scanner}")
        command = [scanner.as_posix(), *cache_key]
    else:
        command = [
            "cargo",
            "run",
            "--quiet",
            "--manifest-path",
            RUST_USE_SCAN_MANIFEST.as_posix(),
            "--",
            *cache_key,
        ]
    completed = subprocess.run(
        command,
        cwd=root,
        check=False,
        capture_output=True,
        text=True,
    )
    if completed.returncode != 0:
        raise validation_error(
            error_cls,
            f"{label} Rust use AST scan failed: {completed.stderr.strip()}",
        )
    statements = {line for line in completed.stdout.splitlines() if line}
    RUST_USE_SCAN_CACHE[cache_key] = statements
    return statements


def public_ast_use_lines(
    root: Path,
    path: Path,
    label: str,
    error_cls: type[ErrorT],
) -> set[str]:
    """Return public use/re-export statements from the Rust AST scanner."""
    return {
        statement
        for statement in rust_ast_use_statements(root, [path], label, error_cls)
        if statement.startswith(PUBLIC_RUST_USE_PREFIXES)
    }


def iter_text_files(root: Path, search_root: str) -> list[Path]:
    """Return text source files scanned by compatibility negative checks."""
    base = root / search_root
    if base.is_file():
        return [base]
    if not base.exists() or not base.is_dir():
        return []
    suffixes = {".rs", ".md", ".yaml", ".yml", ".toml"}
    return sorted(path for path in base.rglob("*") if path.is_file() and path.suffix in suffixes)


def validate_allowed_owner_exports(
    root: Path,
    owner_path: Path,
    owner: str,
    owner_text: str,
    entry: dict[str, Any],
    entry_id: str,
    error_cls: type[ErrorT],
) -> list[str]:
    """Validate that a compatibility owner has no unlisted public use exports."""
    checked: list[str] = []
    owner_export_policy = str(entry.get("owner_export_policy", "strict-public-use"))
    if owner_export_policy not in {"strict-public-use", "retained-surface-only"}:
        raise validation_error(
            error_cls,
            f"{entry_id}.owner_export_policy must be strict-public-use or retained-surface-only",
        )
    allowed = set(
        require_string_list(
            entry.get("allowed_owner_exports"),
            f"{entry_id}.allowed_owner_exports",
            error_cls,
            allow_empty=True,
        )
    )
    actual = public_ast_use_lines(root, owner_path, f"{entry_id}.allowed_owner_exports", error_cls)
    missing = sorted(allowed.difference(actual))
    unexpected = [] if owner_export_policy == "retained-surface-only" else sorted(actual.difference(allowed))
    if missing or unexpected:
        raise validation_error(
            error_cls,
            f"{entry_id}.allowed_owner_exports drift for {owner}: "
            f"missing={missing} unexpected={unexpected}",
        )
    checked.append(f"{entry_id}:allowed_owner_exports:{owner_export_policy}")
    return checked


def validate_forbidden_call_sites(
    root: Path,
    entry: dict[str, Any],
    entry_id: str,
    error_cls: type[ErrorT],
) -> list[str]:
    """Validate that compatibility convenience imports do not expand to new call sites."""
    check = require_mapping(
        entry.get("forbidden_call_site_check"),
        f"{entry_id}.forbidden_call_site_check",
        error_cls,
    )
    check_type = require_string(check.get("type"), f"{entry_id}.forbidden_call_site_check.type", error_cls)
    if check_type not in {"text-absent", "rust-use-absent"}:
        raise validation_error(
            error_cls,
            f"{entry_id}.forbidden_call_site_check.type must be text-absent or rust-use-absent",
        )
    search_roots = require_string_list(
        check.get("search_roots"),
        f"{entry_id}.forbidden_call_site_check.search_roots",
        error_cls,
    )
    forbidden_patterns = require_string_list(
        check.get("forbidden_patterns"),
        f"{entry_id}.forbidden_call_site_check.forbidden_patterns",
        error_cls,
    )
    checked: list[str] = []
    for search_root in search_roots:
        files = iter_text_files(root, search_root)
        if not files:
            raise validation_error(
                error_cls,
                f"{entry_id}.forbidden_call_site_check.search_roots missing: {search_root}",
            )
        if check_type == "rust-use-absent":
            rust_files = [path.relative_to(root) for path in files if path.suffix == ".rs"]
            scan_text = "\n".join(
                sorted(
                    rust_ast_use_statements(
                        root,
                        rust_files,
                        f"{entry_id}.forbidden_call_site_check.{search_root}",
                        error_cls,
                    )
                )
            )
            for pattern in forbidden_patterns:
                if pattern in scan_text:
                    raise validation_error(
                        error_cls,
                        f"{entry_id}.forbidden_call_site_check found forbidden Rust use pattern "
                        f"{pattern!r} under {search_root}",
                    )
            checked.append(f"{entry_id}:forbidden_call_site_check:{search_root}")
            continue
        for path in files:
            text = path.read_text(encoding="utf-8")
            scan_text = text
            for pattern in forbidden_patterns:
                if pattern in scan_text:
                    relative = path.relative_to(root).as_posix()
                    raise validation_error(
                        error_cls,
                        f"{entry_id}.forbidden_call_site_check found forbidden pattern "
                        f"{pattern!r} in {relative}",
                    )
        checked.append(f"{entry_id}:forbidden_call_site_check:{search_root}")
    return checked


def validate_compatibility_surface_ledgers(
    root: Path,
    manifest: dict[str, Any],
    error_cls: type[ErrorT],
) -> list[str]:
    """Validate compatibility-surface ledgers include executable retirement gates."""
    checked: list[str] = []
    ledgers = require_string_list(
        manifest.get("compatibility_surface_ledgers", []),
        "compatibility_surface_ledgers",
        error_cls,
        allow_empty=True,
    )
    for raw_ledger in ledgers:
        ledger_path = relative_path(raw_ledger, "compatibility_surface_ledgers entry", error_cls)
        ledger = load_yaml(root / ledger_path, error_cls)
        rules = require_mapping(ledger.get("rules"), f"{raw_ledger}.rules", error_cls)
        required_rules = [
            "new_root_reexports_require_entry",
            "compatibility_entries_require_migration_target",
            "compatibility_entries_require_expansion_policy",
            "compatibility_entries_require_retirement_milestone",
            "compatibility_entries_require_retirement_condition",
            "compatibility_entries_require_retirement_check",
            "compatibility_entries_require_allowed_owner_exports",
            "compatibility_entries_require_forbidden_call_site_check",
            "compatibility_entries_require_lifecycle_status",
        ]
        for rule in required_rules:
            if rules.get(rule) is not True:
                raise validation_error(error_cls, f"{raw_ledger}.rules.{rule} must be true")
            checked.append(f"{raw_ledger}:{rule}")

        for index, value in enumerate(require_list(ledger.get("entries"), f"{raw_ledger}.entries", error_cls)):
            entry = require_mapping(value, f"{raw_ledger}.entries[{index}]", error_cls)
            entry_id = require_string(entry.get("id"), f"{raw_ledger}.entries[{index}].id", error_cls)
            for field in [
                "owner",
                "status",
                "reason",
                "migration_target",
                "expansion_policy",
                "retirement_milestone",
                "retirement_condition",
            ]:
                require_string(entry.get(field), f"{entry_id}.{field}", error_cls)
                checked.append(f"{entry_id}:{field}")

            status = require_string(entry.get("status"), f"{entry_id}.status", error_cls)
            if status not in {"retained", "deletion-ready", "removed"}:
                raise validation_error(
                    error_cls,
                    f"{entry_id}.status must be retained, deletion-ready, or removed",
                )
            owner = relative_path(require_string(entry.get("owner"), f"{entry_id}.owner", error_cls), f"{entry_id}.owner", error_cls)
            owner_path = root / owner
            if status != "removed" and (not owner_path.exists() or not owner_path.is_file()):
                raise validation_error(error_cls, f"{entry_id}.owner path is missing: {owner}")
            if status == "retained":
                next_status = require_string(
                    entry.get("next_status"),
                    f"{entry_id}.next_status",
                    error_cls,
                )
                if next_status != "deletion-ready":
                    raise validation_error(
                        error_cls,
                        f"{entry_id}.next_status must be deletion-ready while retained",
                    )
                checked.append(f"{entry_id}:next_status")
            for replacement_path in require_string_list(
                entry.get("replacement_paths", []),
                f"{entry_id}.replacement_paths",
                error_cls,
                allow_empty=True,
            ):
                replacement_file = root / relative_path(
                    replacement_path,
                    f"{entry_id}.replacement_paths",
                    error_cls,
                )
                if not replacement_file.exists() or not replacement_file.is_file():
                    raise validation_error(
                        error_cls,
                        f"{entry_id}.replacement_paths missing: {replacement_path}",
                    )
                checked.append(f"{entry_id}:replacement_paths:{replacement_path}")
            retirement_milestone = require_string(
                entry.get("retirement_milestone"),
                f"{entry_id}.retirement_milestone",
                error_cls,
            )
            if not retirement_milestone.startswith("compat-"):
                raise validation_error(error_cls, f"{entry_id}.retirement_milestone must start with compat-")

            if status == "removed":
                checked.extend(validate_forbidden_call_sites(root, entry, entry_id, error_cls))
                checked.append(f"{entry_id}:removed")
                continue

            owner_text = owner_path.read_text(encoding="utf-8")
            retirement_check = require_mapping(
                entry.get("retirement_check"),
                f"{entry_id}.retirement_check",
                error_cls,
            )
            if retirement_check.get("type") != "owner-anchor-present":
                raise validation_error(
                    error_cls,
                    f"{entry_id}.retirement_check.type must be owner-anchor-present",
                )
            for anchor in require_string_list(
                retirement_check.get("anchors"),
                f"{entry_id}.retirement_check.anchors",
                error_cls,
            ):
                if anchor not in owner_text:
                    raise validation_error(
                        error_cls,
                        f"{entry_id}.retirement_check anchor is not present in owner: {anchor}",
                    )
                checked.append(f"{entry_id}:retirement_check:{anchor}")

            retained_surface = require_string_list(entry.get("retained_surface"), f"{entry_id}.retained_surface", error_cls)
            for surface in retained_surface:
                if surface.startswith("pub use ") and surface not in owner_text:
                    raise validation_error(
                        error_cls,
                        f"{entry_id}.retained_surface is not present in owner: {surface}",
                    )
                checked.append(f"{entry_id}:retained_surface:{surface}")
            checked.append(f"{entry_id}:retained_surface")
            checked.extend(validate_allowed_owner_exports(root, owner, owner.as_posix(), owner_text, entry, entry_id, error_cls))
            checked.extend(validate_forbidden_call_sites(root, entry, entry_id, error_cls))
    return checked
