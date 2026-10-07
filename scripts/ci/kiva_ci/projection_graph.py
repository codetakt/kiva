"""Projection graph helpers for CI evidence checks."""

from __future__ import annotations

from pathlib import Path
from typing import Any, TypeVar

from .manifest import (
    ManifestError,
    load_yaml,
    require_list,
    require_mapping,
    require_string,
    require_string_list,
    validation_error,
)


ErrorT = TypeVar("ErrorT", bound=RuntimeError)


def load_projection_graph(
    path: Path,
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, Any]:
    """Load and validate the projection graph envelope."""
    graph = load_yaml(path, error_cls)
    if graph.get("schema_version") != 1:
        raise validation_error(error_cls, f"{path}.schema_version must be 1")
    if graph.get("kind") != "kiva-ci-projection-graph":
        raise validation_error(error_cls, f"{path}.kind must be kiva-ci-projection-graph")
    return graph


def derived_inputs_by_check(
    graph: dict[str, Any],
    error_cls: type[ErrorT] = ManifestError,
) -> dict[tuple[str, str], set[str]]:
    """Return projection-derived input paths keyed by CI group and check id."""
    inputs: dict[tuple[str, str], set[str]] = {}
    for projection_index, raw_projection in enumerate(
        require_list(graph.get("projections"), "projections", error_cls)
    ):
        projection = require_mapping(
            raw_projection,
            f"projections[{projection_index}]",
            error_cls,
        )
        projection_sources = set(
            require_string_list(
                projection.get("sources"),
                f"projections[{projection_index}].sources",
                error_cls,
            )
        )
        projection_sources.update(
            require_string_list(
                projection.get("targets"),
                f"projections[{projection_index}].targets",
                error_cls,
            )
        )
        for validator_index, raw_validator in enumerate(
            require_list(
                projection.get("validators"),
                f"projections[{projection_index}].validators",
                error_cls,
            )
        ):
            validator = require_mapping(
                raw_validator,
                f"projections[{projection_index}].validators[{validator_index}]",
                error_cls,
            )
            key = (
                require_string(
                    validator.get("group"),
                    f"projections[{projection_index}].validators[{validator_index}].group",
                    error_cls,
                ),
                require_string(
                    validator.get("check_id"),
                    f"projections[{projection_index}].validators[{validator_index}].check_id",
                    error_cls,
                ),
            )
            inputs.setdefault(key, set()).update(projection_sources)
    return inputs
