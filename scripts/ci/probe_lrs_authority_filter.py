#!/usr/bin/env python3
"""Run the authority filter procedure against a declared disposable LRS.

Uses the returned authority, including members of any Group. Header
files are private credentials and are never included in evidence. outcome.json
is authoritative: passed/failed/incomplete correspond to exit 0/1/2.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import time
import uuid
from http.client import HTTPException
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode
from urllib.request import Request, urlopen

import yaml
from lrs_preservation import statement_response_headers, canonical_statement_id, standard_uuid
from report_issued_test_results import unresolved_http_checks

from generate_lrs_projection_cases import (
    IRI,
    case,
    dumps,
    expect,
    read_statement,
    request,
    step,
)


def query_observations(steps, row):
    """Keep known failures and incomplete runner subchecks independently."""
    checks = completed_query_observations(steps, row)
    try:
        unresolved = unresolved_http_checks(row)
    except ValueError as error:
        unresolved = [str(error)]
    if unresolved:
        checks.append({'id': 'runner-unresolved-checks', 'outcome': 'not-run',
                       'reason': '; '.join(unresolved)})
    return checks


def completed_query_observations(steps, row):
    """Preserve the unexecuted suffix of the runner's stop-on-first-error case.

    Only an unambiguous step-prefixed report proves which preceding steps ran.
    Unknown report shapes leave the queries unobserved rather than completing
    them from the case's aggregate failed outcome.
    """
    checks = [{'id': entry['id'], 'outcome': 'not-run',
               'reason': 'Required authority query was not observed'} for entry in steps[1:]]
    if row.get('outcome') == 'passed':
        for check in checks:
            check.update(outcome='passed', reason='Required authority query completed')
        return checks
    outcome = row.get('outcome')
    reasons = row.get('failures' if outcome == 'failed' else 'not_run_reasons')
    if (outcome not in ('failed', 'not-run') or not isinstance(reasons, list)
            or not reasons or not all(isinstance(reason, str) for reason in reasons)):
        return checks
    stopped = [i for i, entry in enumerate(steps)
               if all(reason.startswith('step ' + entry['id'] + ': ') for reason in reasons)]
    if len(stopped) != 1:
        return checks
    for i, check in enumerate(checks, 1):
        if i < stopped[0]:
            check.update(outcome='passed', reason='Query completed before the reported stopping step')
        elif i == stopped[0]:
            # Incompleteness is added separately by query_observations so a
            # known failure is never overwritten by an unavailable subcheck.
            check.update(outcome='failed' if outcome == 'failed' else 'not-run',
                         reason='; '.join(reasons))
    return checks


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--endpoint", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--runner", type=Path, required=True)
    parser.add_argument("--headers-file", type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    observations = []
    observation_counts = {}
    query_checks = [{'id': 'authority-queries-' + mode, 'outcome': 'not-run',
                     'reason': 'Authority fixture was not established; required query was not run'}
                    for mode in ('default', 'false', 'true')]

    def finish(status, reason):
        checks = observations + query_checks
        if any(row['outcome'] == 'failed' for row in checks):
            status = 'failed'
        elif status == 'passed' and any(row['outcome'] == 'not-run' for row in checks):
            status = 'incomplete'
        result = {'schema_version': 1, 'status': status, 'reason': reason, 'checks': checks,
                  'execution_complete': all(row['outcome'] in ('passed', 'failed') for row in checks)}
        (args.output / 'outcome.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result))
        return {'passed': 0, 'failed': 1, 'incomplete': 2}[status]
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
        def observe(kind, **fields):
            identity='bootstrap-'+method.lower()+'-'+kind
            observation_counts[identity]=observation_counts.get(identity,0)+1
            observations.append({'id':identity+'-'+str(observation_counts[identity]),
                                 'method':method, **fields})
        req = Request(
            args.endpoint.rstrip("/") + "/statements?" + urlencode(query),
            data=dumps(value).encode() if value else None,
            headers=headers,
            method=method,
        )
        try:
            response = urlopen(req, timeout=30)
        except HTTPError as error:
            response = error
        except (URLError, OSError, TimeoutError):
            observe('transport', outcome='not-run', reason='bootstrap transport unavailable')
            return None, b''
        with response:
            if (method == 'PUT' and response.status != 204) or (method == 'GET' and response.status not in (200, 404)):
                observe('status', outcome='failed', reason='unexpected bootstrap status', status=response.status)
            version, consistent = statement_response_headers(dict(response.headers.items()))
            for name, result in [('version', version), ('consistent-through', consistent)]:
                observe(name, status=response.status, header=name,
                        outcome='not-run' if result is None else 'passed' if result else 'failed')
            try:
                return response.status, response.read()
            except (OSError, TimeoutError, HTTPException):
                observe('body', outcome='not-run', reason='bootstrap body unavailable')
                return None, b''

    def observed_failure():
        return any(row['outcome'] == 'failed' for row in observations)

    status, _ = http("PUT", {"statementId": identifier}, body)
    if observed_failure():
        return finish('failed', 'mandatory bootstrap response header violation')
    if status is None:
        return finish('incomplete', 'initial write could not be observed')
    if status != 204:
        return finish('failed', 'valid initial PUT did not return 204')
    deadline = time.monotonic() + 5
    while True:
        status, raw = http("GET", {"statementId": identifier})
        if observed_failure():
            return finish('failed', 'mandatory visibility response header violation; no retry')
        if status is None:
            return finish('incomplete', 'visibility transport could not complete')
        if status not in (200, 404):
            return finish('failed', 'unexpected positive-read status')
        if status == 200 or time.monotonic() >= deadline:
            break
        time.sleep(0.1)
    if status != 200:
        return finish('incomplete', 'accepted write not visible within harness window; no normative deadline')
    try:
        stored = json.loads(raw)
    except (ValueError, UnicodeError):
        return finish('failed', 'visible Statement is not valid JSON')
    if (not isinstance(stored, dict) or not standard_uuid(stored.get('id'))
            or canonical_statement_id(stored['id']) != canonical_statement_id(identifier)):
        return finish('failed', 'positive read does not identify the admitted Statement')
    (args.output / "setup.json").write_text(
        json.dumps({"request": body, "retrieved": stored}, indent=2) + "\n"
    )
    authority = stored.get("authority", {})
    if not isinstance(authority, dict):
        return finish('failed', 'returned authority is not an object')
    members = authority.get('member', []) if authority.get('objectType') == 'Group' else []
    if not isinstance(members, list) or any(not isinstance(member, dict) for member in members):
        return finish('failed', 'returned authority Group members are not Agent objects')
    identifiers = ["mbox", "mbox_sha1sum", "openid", "account"]
    pointers = ["/authority"] if any(key in authority for key in identifiers) else []
    if authority.get("objectType") == "Group":
        pointers += [
            "/authority/member/" + str(i)
            for i, member in enumerate(members)
            if any(key in member for key in identifiers)
        ]
    if not pointers:
        (args.output / "fixture-status.json").write_text(
            '{"status":"not-run","reason":"no identifiable authority or member; record applicability judgment"}\n'
        )
        return finish('incomplete', 'no applicable identifiable authority/member fixture')
    if any(member.get("mbox") == actor["mbox"] for member in [authority, *members]):
        return finish('incomplete', 'authority is not disjoint from actor')
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
                    request("GET", "/statements", filtered, collect_statement_pages=100),
                    expect(200, pointers_expect),
                )
            )
    query_checks = query_observations(steps, {})
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
    try:
        result = subprocess.run(command, capture_output=True, text=True, check=False, env=private_env)
    except OSError:
        return finish('incomplete', 'child runner unavailable')
    (args.output / "report.json").write_text(result.stdout)
    (args.output / "stderr.txt").write_text(result.stderr)
    try:
        report = json.loads(result.stdout)
        rows = [entry for suite in report['suites'] for entry in suite['cases']]
    except (ValueError, KeyError, TypeError):
        return finish('incomplete', 'child runner supplied no usable observation report')
    if (len(rows) != 1 or not isinstance(rows[0], dict)
            or rows[0].get('id') != 'related-agents-returned-authority'):
        return finish('incomplete', 'child runner supplied an unexpected applicable case set')
    row = rows[0]
    if row.get('outcome') == 'passed' and (report.get('status') != 'passed' or result.returncode != 0):
        return finish('incomplete', 'child runner completion evidence is inconsistent')
    query_checks = query_observations(steps, row)
    if row.get('outcome') == 'failed':
        return finish('failed', 'child runner observed an assertion failure')
    if row.get('outcome') != 'passed':
        return finish('incomplete', 'child runner did not complete the required applicable case')
    return finish('passed', 'bootstrap and applicable authority queries checked')


if __name__ == "__main__":
    raise SystemExit(main())
