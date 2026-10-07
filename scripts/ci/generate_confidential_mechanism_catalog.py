#!/usr/bin/env python3
"""Generate confidential mechanism catalog artifacts from the source ledger."""

from __future__ import annotations

import argparse
from pathlib import Path

from confidential_mechanism_catalog_model import mechanism_catalog_document, render_yaml
from confidential_mechanism_registry_codegen import render_registry
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts


DEFAULT_SOURCE = Path("spec/confidential/mechanism-catalog.source.yaml")
DEFAULT_MATURITY = Path("spec/confidential/mechanism-maturity.yaml")
DEFAULT_PROVIDER_CONTRACT = Path("spec/confidential/secret-compute-provider-contract.yaml")
DEFAULT_CATALOG_OUTPUT = Path("spec/confidential/mechanism-catalog.yaml")
DEFAULT_REGISTRY_OUTPUT = Path("crates/kiva-confidential/src/mechanism/registry.rs")


class ConfidentialMechanismCatalogError(RuntimeError):
    """Raised when confidential mechanism catalog generation fails."""


def parse_args() -> argparse.Namespace:
    """Parse CLI arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--maturity", type=Path, default=DEFAULT_MATURITY)
    parser.add_argument("--provider-contract", type=Path, default=DEFAULT_PROVIDER_CONTRACT)
    parser.add_argument("--out", type=Path, default=DEFAULT_CATALOG_OUTPUT)
    parser.add_argument("--registry-out", type=Path, default=DEFAULT_REGISTRY_OUTPUT)
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def main() -> int:
    """Generate or validate confidential mechanism catalog artifacts."""
    args = parse_args()
    document = mechanism_catalog_document(
        source_path=args.source,
        maturity_path=args.maturity,
        provider_path=args.provider_contract,
    )
    catalog = render_yaml(document)
    registry = render_registry(document["mechanisms"], args.source)
    command_hint = "scripts/ci/generate_confidential_mechanism_catalog.py"
    try:
        write_or_check_generated_artifacts(
            [
                GeneratedArtifact(args.out, catalog),
                GeneratedArtifact(args.registry_out, registry),
            ],
            check=args.check,
            command_hint=command_hint,
            error_cls=ConfidentialMechanismCatalogError,
        )
    except ConfidentialMechanismCatalogError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(
            "validated generated confidential mechanism artifacts: "
            f"{args.out}, {args.registry_out}"
        )
        return 0

    print(f"generated confidential mechanism catalog: {args.out}")
    print(f"generated confidential mechanism registry: {args.registry_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
