"""Rust DTO parsing helpers shared by Kiva CI validators."""

from __future__ import annotations

import re
from pathlib import Path


PUBLIC_RUST_ITEM = re.compile(
    r"\bpub(?:\([^)]*\))?\s+(?:struct|enum)\s+([A-Za-z_][A-Za-z0-9_]*)\b"
)
RUST_ITEM = re.compile(
    r"\bpub(?:\([^)]*\))?\s+(?P<kind>struct|enum)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\b"
)
RUST_FIELD = re.compile(
    r"^(?:pub(?:\([^)]*\))?\s+)?(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*:\s*(?P<ty>[^,]+),?$"
)
RUST_ENUM_VARIANT = re.compile(r"^(?P<name>[A-Z][A-Za-z0-9_]*)(?:\s*[,=({].*)?$")
SERDE_RENAME = re.compile(r'rename\s*=\s*"([^"]+)"')
SERDE_RENAME_ALL = re.compile(r'rename_all\s*=\s*"([^"]+)"')


class RustDtoError(RuntimeError):
    """Raised when a Rust DTO source cannot be parsed consistently."""


def public_rust_items(path: Path) -> set[str]:
    """Return public Rust struct and enum names declared in a source file."""
    source = path.read_text(encoding="utf-8")
    return set(PUBLIC_RUST_ITEM.findall(source))


def rust_source_text(root: Path, paths: list[str]) -> str:
    """Return concatenated Rust source text from mapping paths."""
    return "\n".join((root / path).read_text(encoding="utf-8") for path in paths)


def matching_brace_index(source: str, open_brace: int) -> int:
    """Return the matching closing brace index for a Rust item body."""
    depth = 0
    for index in range(open_brace, len(source)):
        character = source[index]
        if character == "{":
            depth += 1
        elif character == "}":
            depth -= 1
            if depth == 0:
                return index
    raise RustDtoError("Rust item body has unbalanced braces")


def item_attributes(source: str, item_start: int) -> list[str]:
    """Return serde-relevant attributes immediately preceding a Rust item."""
    prefix = source[:item_start].splitlines()
    attributes = []
    for line in reversed(prefix):
        stripped = line.strip()
        if not stripped or stripped.startswith("///"):
            continue
        if stripped.startswith("#["):
            attributes.append(stripped)
            continue
        break
    return list(reversed(attributes))


def rust_item_body(source: str, rust_type: str, kind: str) -> tuple[list[str], str]:
    """Return attributes and body text for a named public Rust item."""
    for match in RUST_ITEM.finditer(source):
        if match.group("kind") != kind or match.group("name") != rust_type:
            continue
        open_brace = source.find("{", match.end())
        if open_brace < 0:
            raise RustDtoError(f"Rust {kind} has no body: {rust_type}")
        close_brace = matching_brace_index(source, open_brace)
        return item_attributes(source, match.start()), source[open_brace + 1 : close_brace]
    raise RustDtoError(f"Rust {kind} is missing: {rust_type}")


def serde_rename(attributes: list[str]) -> str | None:
    """Return a serde rename value from Rust attributes, if present."""
    for attribute in attributes:
        match = SERDE_RENAME.search(attribute)
        if match is not None:
            return match.group(1)
    return None


def serde_rename_all(attributes: list[str]) -> str | None:
    """Return a serde rename_all rule from Rust attributes, if present."""
    for attribute in attributes:
        match = SERDE_RENAME_ALL.search(attribute)
        if match is not None:
            return match.group(1)
    return None


def camel_case(identifier: str) -> str:
    """Convert a Rust identifier to lower camelCase."""
    words = identifier.split("_")
    return words[0] + "".join(word[:1].upper() + word[1:] for word in words[1:])


def split_pascal_variant(identifier: str) -> list[str]:
    """Split a Rust enum variant into lowercase words."""
    words = re.findall(r"[A-Z]+(?=[A-Z][a-z]|$)|[A-Z]?[a-z]+|[0-9]+", identifier)
    return [word.lower() for word in words]


def rename_all_label(identifier: str, rule: str | None) -> str:
    """Apply a supported serde rename_all rule."""
    if rule is None:
        return identifier
    if rule == "camelCase":
        if "_" in identifier:
            return camel_case(identifier)
        words = split_pascal_variant(identifier)
        return words[0] + "".join(word[:1].upper() + word[1:] for word in words[1:])
    if rule == "kebab-case":
        words = identifier.split("_") if "_" in identifier else split_pascal_variant(identifier)
        return "-".join(word.lower() for word in words)
    if rule == "snake_case":
        words = identifier.split("_") if "_" in identifier else split_pascal_variant(identifier)
        return "_".join(word.lower() for word in words)
    raise RustDtoError(f"unsupported serde rename_all rule: {rule}")


def rust_struct_fields(source: str, rust_type: str) -> dict[str, bool]:
    """Return Rust struct field wire names and whether each is required."""
    item_attrs, body = rust_item_body(source, rust_type, "struct")
    rename_all = serde_rename_all(item_attrs)
    fields: dict[str, bool] = {}
    pending_attrs: list[str] = []
    for line in body.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("//"):
            continue
        if stripped.startswith("#["):
            pending_attrs.append(stripped)
            continue
        if stripped.startswith("///"):
            continue
        match = RUST_FIELD.match(stripped)
        if match is None:
            pending_attrs = []
            continue
        name = match.group("name")
        ty = match.group("ty").strip()
        wire_name = serde_rename(pending_attrs) or rename_all_label(name, rename_all)
        is_required = (
            not ty.startswith("Option<")
            and not any("default" in attr for attr in pending_attrs)
            and not any("skip_deserializing" in attr for attr in pending_attrs)
        )
        fields[wire_name] = is_required
        pending_attrs = []
    return fields


def rust_enum_labels(source: str, rust_type: str) -> set[str]:
    """Return Rust enum labels after serde rename attributes are applied."""
    item_attrs, body = rust_item_body(source, rust_type, "enum")
    rename_all = serde_rename_all(item_attrs)
    labels = set()
    pending_attrs: list[str] = []
    for line in body.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("//"):
            continue
        if stripped.startswith("#["):
            pending_attrs.append(stripped)
            continue
        if stripped.startswith("///"):
            continue
        match = RUST_ENUM_VARIANT.match(stripped.rstrip(","))
        if match is None:
            pending_attrs = []
            continue
        name = match.group("name")
        labels.add(serde_rename(pending_attrs) or rename_all_label(name, rename_all))
        pending_attrs = []
    return labels


def sorted_list(values: set[str]) -> list[str]:
    """Return deterministic display order for string sets."""
    return sorted(values)
