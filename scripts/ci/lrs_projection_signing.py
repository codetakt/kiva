"""Load fixed public signature vectors without invoking a crypto provider.

These input definitions preserve the exact candidate cases identified by the
fixture's provenance. Signatures/certificates are never produced by expansion.
Changing a signed vector requires an explicit fixture change and impact review.
"""

import json

from generate_lrs_projection_cases import ROOT


def signing_cases():
    document = json.loads(
        (ROOT / "tests/conformance/fixtures/signing/cases.json").read_text()
    )
    cases = document["cases"]
    if document["schema_version"] != 1 or not cases:
        raise ValueError("invalid fixed signature vectors")
    if len({case["id"] for case in cases}) != len(cases) or any(
        not case["id"].startswith("signature-") for case in cases
    ):
        raise ValueError("duplicate or unexpected fixed signature case id")
    return cases
