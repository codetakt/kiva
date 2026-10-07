#!/usr/bin/env python3
"""Run repository generators from the ProjectionRuntime dependency graph."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path
import subprocess
import sys

from kiva_ci.manifest import load_yaml, require_list, require_mapping, require_string


DEFAULT_PLAN = Path("spec/architecture/projection-runtime.source.yaml")


class GeneratorRunnerError(RuntimeError):
    """Raised when the generator plan or execution fails."""


@dataclass(frozen=True)
class GeneratorDefinition:
    """One generator node from ProjectionRuntime."""

    node_id: str
    script: str
    depends_on: tuple[str, ...]


@dataclass(frozen=True)
class GeneratorRun:
    """One executed generator result."""

    node_id: str
    script: str
    returncode: int


def load_generator_plan(root: Path, plan_path: Path) -> list[GeneratorDefinition]:
    """Load and validate the ProjectionRuntime generator dependency graph."""
    source = load_yaml(root / plan_path, GeneratorRunnerError)
    if source.get("kind") != "kiva-projection-runtime-source":
        raise GeneratorRunnerError(f"{plan_path}.kind must be kiva-projection-runtime-source")
    plan = require_mapping(source.get("generator_plan"), f"{plan_path}.generator_plan", GeneratorRunnerError)
    if plan.get("order_model") != "stable-topological-sort":
        raise GeneratorRunnerError(f"{plan_path}.generator_plan.order_model must be stable-topological-sort")
    definitions: list[GeneratorDefinition] = []
    seen_ids: set[str] = set()
    seen_scripts: set[str] = set()
    for index, raw_node in enumerate(require_list(plan.get("nodes"), f"{plan_path}.generator_plan.nodes", GeneratorRunnerError)):
        node = require_mapping(raw_node, f"{plan_path}.generator_plan.nodes[{index}]", GeneratorRunnerError)
        node_id = require_string(node.get("id"), f"generator_plan.nodes[{index}].id", GeneratorRunnerError)
        script = require_string(node.get("script"), f"generator_plan.nodes[{index}].script", GeneratorRunnerError)
        dependencies = tuple(
            require_string(value, f"generator_plan.nodes[{index}].depends_on[]", GeneratorRunnerError)
            for value in require_list(node.get("depends_on"), f"generator_plan.nodes[{index}].depends_on", GeneratorRunnerError)
        )
        if node_id in seen_ids:
            raise GeneratorRunnerError(f"duplicate generator node id: {node_id}")
        if script in seen_scripts:
            raise GeneratorRunnerError(f"generator script has multiple plan owners: {script}")
        if not (root / script).is_file():
            raise GeneratorRunnerError(f"generator plan script is missing: {script}")
        seen_ids.add(node_id)
        seen_scripts.add(script)
        definitions.append(GeneratorDefinition(node_id=node_id, script=script, depends_on=dependencies))
    unknown = sorted({dependency for definition in definitions for dependency in definition.depends_on}.difference(seen_ids))
    if unknown:
        raise GeneratorRunnerError(f"generator plan references unknown dependencies: {unknown}")
    return definitions


def topological_order(definitions: list[GeneratorDefinition]) -> list[GeneratorDefinition]:
    """Return stable dependency order or reject a cycle."""
    by_id = {definition.node_id: definition for definition in definitions}
    declaration_order = {definition.node_id: index for index, definition in enumerate(definitions)}
    remaining = {definition.node_id: set(definition.depends_on) for definition in definitions}
    ordered: list[GeneratorDefinition] = []
    while remaining:
        ready = sorted(
            (node_id for node_id, dependencies in remaining.items() if not dependencies),
            key=declaration_order.__getitem__,
        )
        if not ready:
            cycle = {node_id: sorted(dependencies) for node_id, dependencies in remaining.items()}
            raise GeneratorRunnerError(f"generator dependency cycle: {cycle}")
        for node_id in ready:
            ordered.append(by_id[node_id])
            del remaining[node_id]
        completed = set(ready)
        for dependencies in remaining.values():
            dependencies.difference_update(completed)
    return ordered


def selected_closure(definitions: list[GeneratorDefinition], selectors: list[str]) -> list[GeneratorDefinition]:
    """Return selected generator nodes and their transitive dependencies."""
    if not selectors:
        return definitions
    by_id = {definition.node_id: definition for definition in definitions}
    by_script = {definition.script: definition for definition in definitions}
    selected: set[str] = set()

    def include(node_id: str) -> None:
        if node_id in selected:
            return
        selected.add(node_id)
        for dependency in by_id[node_id].depends_on:
            include(dependency)

    for selector in selectors:
        definition = by_id.get(selector) or by_script.get(selector)
        if definition is None:
            raise GeneratorRunnerError(f"unknown generator selector: {selector}")
        include(definition.node_id)
    return [definition for definition in definitions if definition.node_id in selected]


def run_generator(root: Path, definition: GeneratorDefinition, check: bool) -> GeneratorRun:
    """Run one generator and return its result."""
    script_path = root / definition.script
    if not script_path.exists():
        raise GeneratorRunnerError(f"generator script is missing: {definition.script}")
    args = [sys.executable, definition.script]
    if check:
        args.append("--check")
    print(f"{'checking' if check else 'generating'} {definition.node_id}: {definition.script}", flush=True)
    completed = subprocess.run(args, cwd=root, check=False)
    return GeneratorRun(node_id=definition.node_id, script=definition.script, returncode=completed.returncode)


def run_generators(root: Path, definitions: list[GeneratorDefinition], check: bool) -> list[GeneratorRun]:
    """Run generators in dependency order, stopping at the first failure."""
    runs: list[GeneratorRun] = []
    for definition in topological_order(definitions):
        run = run_generator(root, definition, check)
        runs.append(run)
        if run.returncode != 0:
            raise GeneratorRunnerError(f"{definition.script} failed with exit code {run.returncode}")
    return runs


def main() -> int:
    """Run generator scripts from the source-owned plan."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--plan", type=Path, default=DEFAULT_PLAN)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--generator", action="append", default=[], help="Run a generator id or script and its dependencies.")
    parser.add_argument("--list", action="store_true", help="Print the resolved generator order without executing it.")
    args = parser.parse_args()

    root = args.root.resolve()
    definitions = selected_closure(load_generator_plan(root, args.plan), args.generator)
    ordered = topological_order(definitions)
    if args.list:
        for definition in ordered:
            print(f"{definition.node_id}\t{definition.script}")
        return 0
    runs = run_generators(root, definitions, args.check)
    verb = "validated" if args.check else "ran"
    print(f"{verb} generators: {len(runs)}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except GeneratorRunnerError as error:
        raise SystemExit(str(error)) from error
