#!/usr/bin/env python3
"""Run the authority filter procedure against a declared disposable LRS.

Uses the returned authority, including members of any Group. Header
files are private credentials and are never included in evidence. Exit 2 means
the fixture cannot establish an applicable, disjoint authority identifier.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import time
import uuid
from pathlib import Path
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import Request, urlopen

import yaml

from generate_lrs_projection_cases import (
    IRI,
    case,
    dumps,
    expect,
    read_statement,
    request,
    step,
)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--endpoint", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--runner", type=Path, required=True)
    parser.add_argument("--headers-file", type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    headers = {"X-Experience-API-Version": "2.0.0", "Content-Type": "application/json"}
    if args.headers_file:
        headers.update(json.loads(args.headers_file.read_text()))
    identifier = str(uuid.uuid4())
    actor = {
        "objectType": "Agent",
        "mbox": "mailto:authority-filter-" + identifier + "@example.org",
    }
    body = {
        "id": identifier,
        "actor": actor,
        "verb": {"id": IRI + "/authority-" + identifier},
        "object": {"id": IRI + "/authority-object"},
    }

    def http(method, query, value=None):
        req = Request(
            args.endpoint.rstrip("/") + "/statements?" + urlencode(query),
            data=dumps(value).encode() if value else None,
            headers=headers,
            method=method,
        )
        try:
            with urlopen(req, timeout=30) as response:
                return response.status, response.read()
        except HTTPError as error:
            return error.code, error.read()

    status, _ = http("PUT", {"statementId": identifier}, body)
    if status != 204:
        raise SystemExit("fixture unavailable: initial write did not succeed")
    deadline = time.monotonic() + 5
    while True:
        status, raw = http("GET", {"statementId": identifier})
        if status == 200 or time.monotonic() >= deadline:
            break
        time.sleep(0.1)
    if status != 200:
        raise SystemExit("fixture unavailable: accepted write is not visible")
    stored = json.loads(raw)
    (args.output / "setup.json").write_text(
        json.dumps({"request": body, "retrieved": stored}, indent=2) + "\n"
    )
    authority = stored.get("authority", {})
    identifiers = ["mbox", "mbox_sha1sum", "openid", "account"]
    pointers = ["/authority"] if any(key in authority for key in identifiers) else []
    if authority.get("objectType") == "Group":
        pointers += [
            "/authority/member/" + str(i)
            for i, member in enumerate(authority.get("member", []))
            if any(key in member for key in identifiers)
        ]
    if not pointers:
        (args.output / "fixture-status.json").write_text(
            '{"status":"not-run","reason":"no identifiable authority or member; record applicability judgment"}\n'
        )
        return 2
    members = [authority, *authority.get("member", [])]
    if any(member.get("mbox") == actor["mbox"] for member in members):
        raise SystemExit("fixture unavailable: authority is not disjoint from actor")
    read = read_statement(identifier)
    read["expect"]["capture"] = {
        "authority_" + str(i): {"pointer": pointer, "as_json": True}
        for i, pointer in enumerate(pointers)
    }
    steps = [read]
    for i in range(len(pointers)):
        query = {"verb": body["verb"]["id"], "agent": "{{authority_" + str(i) + "}}"}
        for suffix, related in [
            ("default", None),
            ("false", "false"),
            ("true", "true"),
        ]:
            filtered = dict(query, **({"related_agents": related} if related else {}))
            pointers_expect = (
                {"/statements": {"contains": {"id": identifier}}}
                if related == "true"
                else {"/statements": {"equals": []}}
            )
            steps.append(
                step(
                    "authority-" + str(i) + "-" + suffix,
                    request("GET", "/statements", filtered, await_retrieval_ms=5000),
                    expect(200, pointers_expect),
                )
            )
    document = {
        "suite": {
            "id": "related-agents-authority",
            "standard": "IEEE/ISO/IEC 39274-1-1-2025",
        },
        "cases": [
            case(
                "related-agents-returned-authority",
                ["xapi-lrs-4.1.6.1-statement-query-related-agents"],
                steps=steps,
            )
        ],
    }
    private_env = os.environ.copy()
    profile_headers = {}
    for i, (key, value) in enumerate(headers.items()):
        if key.lower() not in ["content-type", "x-experience-api-version"]:
            variable = "LRS_AUTHORITY_PROBE_HEADER_" + str(i)
            profile_headers[key] = variable
            private_env[variable] = value
    if profile_headers:
        for entry in steps:
            entry["request"]["auth_profile"] = "authority_probe"
    case_path = args.output / "cases.yaml"
    case_path.write_text(yaml.safe_dump(document, sort_keys=False))
    command = [
        str(args.runner.resolve()),
        "--case",
        str(case_path.resolve()),
        "--endpoint",
        args.endpoint,
        "--allow-destructive",
        "--json",
    ]
    # Environment references expose neither credentials in argv nor in YAML.
    if profile_headers:
        profile_path = args.output / "auth-profile.yaml"
        profile_path.write_text(
            yaml.safe_dump(
                {
                    "schema_version": 1,
                    "standard": "IEEE/ISO/IEC 39274-1-1-2025",
                    "profiles": {"authority_probe": {"headers": profile_headers}},
                }
            )
        )
        command += ["--auth-profile-file", str(profile_path.resolve())]
    result = subprocess.run(
        command, capture_output=True, text=True, check=False, env=private_env
    )
    (args.output / "report.json").write_text(result.stdout)
    (args.output / "stderr.txt").write_text(result.stderr)
    print(
        json.dumps(
            {
                "status": "passed" if result.returncode == 0 else "failed",
                "authority_queries": len(pointers),
                "evidence": str(args.output),
            }
        )
    )
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
