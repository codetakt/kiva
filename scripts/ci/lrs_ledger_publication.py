#!/usr/bin/env python3
"""Check the final LRS ledger publication and its generated issue table.

Uses Python's standard library. --write-issues regenerates the issue table alone;
it does not renew publication hashes or imply source/implementation acceptance.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
REQUIREMENTS = Path("spec/requirements")
PUBLICATION = REQUIREMENTS / "lrs-requirements-ledger-publication.json"
LEDGER = REQUIREMENTS / "lrs-requirements-ledger"


def read_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def safe_path(root: Path, name: str) -> Path:
    relative = Path(name)
    require(not relative.is_absolute() and ".." not in relative.parts,
            f"Unsafe publication path: {name}")
    resolved = (root / relative).resolve()
    require(resolved.is_relative_to(root.resolve()), f"Path escapes root: {name}")
    return resolved


def issue_markdown(records: list[dict]) -> str:
    """Render every record in a single Markdown table, in source order."""
    require(len({item["id"] for item in records}) == len(records), "Duplicate issue IDs")

    def cell(value: str) -> str:
        return value.replace("\\", "\\\\").replace("|", "\\|").replace("\r\n", "\n").replace("\n", "<br>")

    lines = [
        "# LRS issue register", "",
        "Generated from [issues.json](issues.json) by",
        "`python -B scripts/ci/lrs_ledger_publication.py --write-issues`.",
        "Source questions, policy holds and runtime observations retain separate states.",
        "See the [publication record](../lrs-requirements-ledger-publication.md)",
        "for review scope and final dispositions.", "",
        "| ID | Kind / state | Question or observation | Next action |",
        "| --- | --- | --- | --- |",
    ]
    for item in records:
        values = [item["id"], f"{item['kind']} / {item['state']}", item["title"], item["next_action"]]
        lines.append("| " + " | ".join(cell(value) for value in values) + " |")
    return "\n".join(lines) + "\n"


def verify_entries(root: Path, entries: list[dict], label: str, count: int | None = None) -> None:
    require(len({entry["path"] for entry in entries}) == len(entries), f"Duplicate {label} paths")
    if count is not None:
        require(len(entries) == count, f"Unexpected {label} count: {len(entries)}")
    for entry in entries:
        require(digest(safe_path(root, entry["path"]).read_bytes()) == entry["sha256"],
                f"{label} digest mismatch: {entry['path']}")
    print(f"{label}: {len(entries)} verified")


def verify(root: Path) -> None:
    expected = issue_markdown(read_json(root / LEDGER / "issues.json")["records"])
    require((root / LEDGER / "ISSUES.md").read_bytes() == expected.encode(), "ISSUES.md is out of date")
    publication = read_json(root / PUBLICATION)
    verify_entries(root, publication["files"], "Publication files")
    ledger = read_json(root / LEDGER / "ledger.json")
    actual = {path.name for path in (root / LEDGER).iterdir() if path.is_file()}
    require(set(ledger["files"]) == actual, "Ledger file inventory differs")
    require(ledger["state"] == "published", "Ledger is not a final publication")
    print("Issue rendering and final ledger inventory: verified")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--check", action="store_true", help="Check the final publication bindings (default)")
    group.add_argument("--write-issues", action="store_true", help="Regenerate only ISSUES.md")
    args = parser.parse_args()
    root = args.root.resolve()
    if args.write_issues:
        text = issue_markdown(read_json(root / LEDGER / "issues.json")["records"])
        (root / LEDGER / "ISSUES.md").write_bytes(text.encode())
        print("Regenerated ISSUES.md; publication bindings were not changed")
    else:
        verify(root)


if __name__ == "__main__":
    main()
