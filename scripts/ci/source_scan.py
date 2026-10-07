#!/usr/bin/env python3
"""Source and assurance-evidence helpers for repository validators."""

from __future__ import annotations

from pathlib import Path
from typing import Any

import yaml


DEFAULT_ASSURANCE_EVIDENCE_REGISTRY = Path(
    "spec/confidential/assurance-evidence-registry.yaml",
)

EVIDENCE_KIND_FIELDS = {
    "rust-symbol": "symbol",
    "rust-test": "test",
    "kani-harness": "harness",
    "tamarin-lemma": "lemma",
}

REGISTRY_LIST_EVIDENCE_KINDS = {
    "confidential_rust_terms": ("rust-symbol",),
    "rust_tests": ("rust-test",),
    "kani_harnesses": ("kani-harness",),
    "tamarin_lemmas": ("tamarin-lemma",),
}


def rust_tree_source(root: str | Path) -> str:
    """Return all Rust source under a crate source tree as one stable string."""
    root_path = Path(root)
    return "\n".join(
        path.read_text(encoding="utf-8")
        for path in sorted(root_path.rglob("*.rs"))
        if path.is_file()
    )


def confidential_rust_source() -> str:
    """Return the complete kiva-confidential Rust source."""
    return rust_tree_source("crates/kiva-confidential/src")


def server_rust_source() -> str:
    """Return the complete kiva-server Rust source."""
    return rust_tree_source("crates/kiva-server/src")


def load_assurance_evidence_registry(
    path: str | Path = DEFAULT_ASSURANCE_EVIDENCE_REGISTRY,
) -> dict[str, Any]:
    """Load the confidential-computation assurance evidence registry."""
    registry_path = Path(path)
    with registry_path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise ValueError(f"{registry_path} root must be a mapping")
    if value.get("schema_version") != 1:
        raise ValueError(f"{registry_path} schema_version must be 1")
    if value.get("kind") != "confidential-assurance-evidence-registry":
        raise ValueError(f"{registry_path} kind is invalid")
    entries = value.get("entries")
    if not isinstance(entries, dict) or not entries:
        raise ValueError(f"{registry_path} entries must be a non-empty mapping")
    return value


def assurance_registry_entry(
    entry_id: str,
    path: str | Path = DEFAULT_ASSURANCE_EVIDENCE_REGISTRY,
) -> dict[str, Any]:
    """Return one assurance registry entry by stable identifier."""
    entries = load_assurance_evidence_registry(path)["entries"]
    entry = entries.get(entry_id)
    if not isinstance(entry, dict):
        raise ValueError(f"assurance registry entry is missing: {entry_id}")
    return entry


def registry_evidence_entries(entry_id: str, kind: str | None = None) -> list[dict[str, str]]:
    """Return validated typed evidence entries for one registry entry."""
    raw_entries = assurance_registry_entry(entry_id).get("evidence", [])
    if raw_entries is None:
        raw_entries = []
    if not isinstance(raw_entries, list):
        raise ValueError(f"assurance registry {entry_id}.evidence must be a list")

    entries: list[dict[str, str]] = []
    seen_ids: set[str] = set()
    for index, raw in enumerate(raw_entries):
        label = f"assurance registry {entry_id}.evidence[{index}]"
        if not isinstance(raw, dict):
            raise ValueError(f"{label} must be a mapping")
        item_id = raw.get("id")
        item_kind = raw.get("kind")
        source_file = raw.get("source_file")
        if not isinstance(item_id, str) or not item_id:
            raise ValueError(f"{label}.id must be a non-empty string")
        if item_id in seen_ids:
            raise ValueError(f"assurance registry {entry_id}.evidence id is duplicated: {item_id}")
        seen_ids.add(item_id)
        if item_kind not in EVIDENCE_KIND_FIELDS:
            raise ValueError(f"{label}.kind is invalid: {item_kind!r}")
        if not isinstance(source_file, str) or not source_file:
            raise ValueError(f"{label}.source_file must be a non-empty string")
        value_field = EVIDENCE_KIND_FIELDS[item_kind]
        value = raw.get(value_field)
        if not isinstance(value, str) or not value:
            raise ValueError(f"{label}.{value_field} must be a non-empty string")

        normalized = {
            "id": item_id,
            "kind": item_kind,
            "source_file": source_file,
            value_field: value,
        }
        for optional_key in ("mechanism", "profile", "owner"):
            optional_value = raw.get(optional_key)
            if optional_value is not None:
                if not isinstance(optional_value, str) or not optional_value:
                    raise ValueError(f"{label}.{optional_key} must be a non-empty string")
                normalized[optional_key] = optional_value
        if kind is None or item_kind == kind:
            entries.append(normalized)
    return entries


def registry_evidence_values(entry_id: str, key: str) -> list[str]:
    """Return list-compatible values derived from typed evidence entries."""
    kinds = REGISTRY_LIST_EVIDENCE_KINDS.get(key)
    if kinds is None:
        raise ValueError(f"assurance registry list key is invalid: {key}")
    values: list[str] = []
    for evidence_kind in kinds:
        value_field = EVIDENCE_KIND_FIELDS[evidence_kind]
        values.extend(
            entry[value_field]
            for entry in registry_evidence_entries(entry_id, evidence_kind)
        )
    if len(values) != len(set(values)):
        raise ValueError(f"assurance registry {entry_id}.evidence has duplicate {key} values")
    return values


def registry_string_list(entry_id: str, key: str) -> list[str]:
    """Return a string-list field from one registry entry.

    Legacy list fields remain supported for the existing validators, but typed
    evidence entries are canonical when present and must match any legacy list.
    """
    raw_value = assurance_registry_entry(entry_id).get(key, [])
    if not isinstance(raw_value, list) or not all(
        isinstance(item, str) and item for item in raw_value
    ):
        raise ValueError(f"assurance registry {entry_id}.{key} must be a string list")
    if len(raw_value) != len(set(raw_value)):
        raise ValueError(f"assurance registry {entry_id}.{key} must not contain duplicates")

    typed_value = registry_evidence_values(entry_id, key)
    if typed_value:
        if raw_value and set(raw_value) != set(typed_value):
            raise ValueError(
                f"assurance registry {entry_id}.{key} must match typed evidence entries",
            )
        return typed_value
    return raw_value


def missing_terms(required_terms: list[str], source: str) -> list[str]:
    """Return required terms that are absent from the supplied source text."""
    return [term for term in required_terms if term not in source]


def missing_confidential_rust_terms(entry_id: str, source: str | None = None) -> list[str]:
    """Return registry-required kiva-confidential Rust terms that are absent."""
    return missing_terms(
        registry_string_list(entry_id, "confidential_rust_terms"),
        confidential_rust_source() if source is None else source,
    )


def missing_rust_tests(entry_id: str, source: str | None = None) -> list[str]:
    """Return registry-required Rust test names that are absent from source."""
    source_text = (
        "\n".join((confidential_rust_source(), server_rust_source()))
        if source is None
        else source
    )
    return [
        test
        for test in registry_string_list(entry_id, "rust_tests")
        if test.rsplit("::", maxsplit=1)[-1] not in source_text
    ]


def missing_kani_harnesses(entry_id: str, runner_source: str | None = None) -> list[str]:
    """Return registry-required Kani harness names absent from the runner."""
    runner = (
        Path("scripts/ci/run_kani_verification.sh").read_text(encoding="utf-8")
        if runner_source is None
        else runner_source
    )
    return [
        harness
        for harness in registry_string_list(entry_id, "kani_harnesses")
        if harness not in runner
    ]


def missing_tamarin_lemmas(entry_id: str, tamarin_source: str) -> list[str]:
    """Return registry-required Tamarin lemma names absent from a model."""
    return [
        lemma
        for lemma in registry_string_list(entry_id, "tamarin_lemmas")
        if f"lemma {lemma}" not in tamarin_source
    ]
