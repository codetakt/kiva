"""Behavioral LRS vectors from the public source; independent of Kiva code."""

from __future__ import annotations

import copy
import hashlib

from generate_lrs_projection_cases import (
    IRI,
    case,
    dumps,
    expect,
    read_statement,
    request,
    set_pointer,
    step,
    uuid_for,
)

A = {"objectType": "Agent", "mbox": "mailto:projection@example.org"}
G = {
    "objectType": "Group",
    "mbox": "mailto:projection-group@example.org",
    "member": [A],
}
S = {"actor": A, "verb": {"id": IRI + "/verb"}, "object": {"id": IRI + "/activity"}}
DOCS = [
    (
        "state",
        "/activities/state",
        {"activityId": IRI, "agent": dumps(A)},
        "stateId",
        "4.1.6.2",
    ),
    ("agent-profile", "/agents/profile", {"agent": dumps(A)}, "profileId", "4.1.6.5"),
    (
        "activity-profile",
        "/activities/profile",
        {"activityId": IRI},
        "profileId",
        "4.1.6.6",
    ),
]


def resource_cases():
    cases = []
    original = {"keep": 1, "nested": {"old": True}}
    for family, path, scope, key, clause in DOCS:
        for method in ["PUT", "POST", "DELETE"]:
            for precondition in ["stale-match", "none-match-star"]:
                name = family + "-" + method.lower() + "-" + precondition
                query = dict(scope, **{key: name})
                write = request(
                    method,
                    path,
                    query,
                    {"nested": {"new": True}} if method != "DELETE" else None,
                )
                write["headers"].update(
                    {"If-Match": '"obsolete-etag"'}
                    if precondition == "stale-match"
                    else {"If-None-Match": "*"}
                )
                unchanged = expect(200, {"": {"equals": original}})
                unchanged["headers"].update(
                    ETag={"equals": "{{original_etag}}"},
                    **{"Last-Modified": {"equals": "{{original_modified}}"}},
                )
                cases.append(
                    case(
                        name,
                        [
                            "xapi-lrs-4.1.4-if-match",
                            "xapi-lrs-4.1.4-precondition-failed",
                        ],
                        steps=[
                            step(
                                "create",
                                request("PUT", path, query, original),
                                expect(204),
                            ),
                            step(
                                "read-original",
                                request("GET", path, query),
                                expect(
                                    200,
                                    {"": {"equals": original}},
                                    capture={
                                        "original_etag": {"header": "ETag"},
                                        "original_modified": {
                                            "header": "Last-Modified"
                                        },
                                    },
                                ),
                            ),
                            step("reject", write, expect(412)),
                            step("unchanged", request("GET", path, query), unchanged),
                        ],
                    )
                )
        for method in ["PUT", "POST", "DELETE"]:
            name = family + "-" + method.lower() + "-current-etag"
            query = dict(scope, **{key: name})
            write = request(
                method, path, query, {"changed": True} if method != "DELETE" else None
            )
            write["headers"]["If-Match"] = "{{etag}}"
            content = (
                {"changed": True} if method == "PUT" else dict(original, changed=True)
            )
            cases.append(
                case(
                    name,
                    ["xapi-lrs-4.1.4-if-match"],
                    steps=[
                        step(
                            "create", request("PUT", path, query, original), expect(204)
                        ),
                        step(
                            "etag",
                            request("GET", path, query),
                            expect(200, capture={"etag": {"header": "ETag"}}),
                        ),
                        step("conditional", write, expect(204)),
                        step(
                            "check",
                            request("GET", path, query),
                            expect(404)
                            if method == "DELETE"
                            else expect(200, {"": {"equals": content}}),
                        ),
                    ],
                )
            )
        name = family + "-unconditional-put-conflict"
        query = dict(scope, **{key: name})
        cases.append(
            case(
                name,
                ["xapi-lrs-4.1.4-put-conflict"],
                steps=[
                    step("create", request("PUT", path, query, original), expect(204)),
                    step(
                        "reject",
                        request("PUT", path, query, {"changed": True}),
                        expect(409, body={"non_empty": True}),
                    ),
                    step(
                        "unchanged",
                        request("GET", path, query),
                        expect(200, {"": {"equals": original}}),
                    ),
                ],
            )
        )
        for condition, value in [("If-Match", "*"), ("If-Match", '"missing"')]:
            name = (
                family
                + "-create-with-missing-precondition-"
                + value.replace('"', "").replace("*", "star")
            )
            query = dict(scope, **{key: name})
            write = request("PUT", path, query, original)
            write["headers"][condition] = value
            cases.append(
                case(
                    name,
                    ["xapi-lrs-4.1.4-precondition-failed"],
                    steps=[
                        step("reject", write, expect(412)),
                        step("absent", request("GET", path, query), expect(404)),
                    ],
                )
            )
        for invalid, content_type in [
            (b"null", "application/json"),
            (b"[]", "application/json"),
            (b"1", "application/json"),
            (b'"text"', "application/json"),
            (b"false", "application/json"),
            (b"{", "application/json"),
            (b"plain", "text/plain"),
        ]:
            name = (
                family
                + "-merge-reject-"
                + hashlib.sha256(invalid + content_type.encode()).hexdigest()[:10]
            )
            query = dict(scope, **{key: name})
            post = request("POST", path, query, raw_body_hex=invalid.hex())
            post["headers"]["Content-Type"] = content_type
            cases.append(
                case(
                    name,
                    [f"xapi-lrs-{clause}-{family}-post-merge-reject-invalid"],
                    steps=[
                        step(
                            "create",
                            request(
                                "PUT",
                                path,
                                query,
                                raw_body_hex=dumps(original).encode().hex(),
                                headers={
                                    "X-Experience-API-Version": "2.0.0",
                                    "Content-Type": "application/json",
                                },
                            ),
                            expect(204),
                        ),
                        step(
                            "snapshot",
                            request("GET", path, query),
                            expect(
                                200,
                                capture={
                                    "existing_etag": {"header": "ETag"},
                                    "existing_modified": {"header": "Last-Modified"},
                                },
                            ),
                        ),
                        step("reject", post, expect(400)),
                        step(
                            "unchanged",
                            request("GET", path, query),
                            expect(
                                200,
                                {"": {"equals": original}},
                                checks=[
                                    dict(
                                        kind="bytes", hex=dumps(original).encode().hex()
                                    )
                                ],
                                headers={
                                    "X-Experience-API-Version": {"equals": "2.0.0"},
                                    "ETag": {"equals": "{{existing_etag}}"},
                                    "Last-Modified": {
                                        "equals": "{{existing_modified}}"
                                    },
                                },
                            ),
                        ),
                    ],
                )
            )
        for existing in [b"[]", b"null", b'"scalar"']:
            name = (
                family
                + "-merge-invalid-existing-"
                + hashlib.sha256(existing).hexdigest()[:8]
            )
            query = dict(scope, **{key: name})
            put = request("PUT", path, query, raw_body_hex=existing.hex())
            put["headers"]["Content-Type"] = "application/json"
            cases.append(
                case(
                    name,
                    [f"xapi-lrs-{clause}-{family}-post-merge-reject-invalid"],
                    steps=[
                        step("create", put, expect(204)),
                        step(
                            "snapshot",
                            request("GET", path, query),
                            expect(
                                200,
                                capture={
                                    "existing_etag": {"header": "ETag"},
                                    "existing_modified": {"header": "Last-Modified"},
                                },
                            ),
                        ),
                        step(
                            "reject",
                            request("POST", path, query, {"change": 1}),
                            expect(400),
                        ),
                        step(
                            "unchanged",
                            request("GET", path, query),
                            expect(
                                200,
                                checks=[dict(kind="bytes", hex=existing.hex())],
                                headers={
                                    "X-Experience-API-Version": {"equals": "2.0.0"},
                                    "ETag": {"equals": "{{existing_etag}}"},
                                    "Last-Modified": {
                                        "equals": "{{existing_modified}}"
                                    },
                                },
                            ),
                        ),
                    ],
                )
            )
        # A parseable JSON body is still non-JSON when stored with another
        # media type. Reject the merge and preserve bytes plus its ETag.
        for media, existing in [
            ("text/plain", b'{"keep":1}'),
            ("application/octet-stream", bytes([0, 255, 254, 13, 10, 128, 1, 0])),
        ]:
            name = family + "-merge-existing-non-json-" + media.split("/")[1]
            query = dict(scope, **{key: name})
            put = request("PUT", path, query, raw_body_hex=existing.hex())
            put["headers"]["Content-Type"] = media
            preserved = expect(
                200,
                checks=[dict(kind="bytes", hex=existing.hex())],
                headers={"ETag": {"equals": "{{existing_etag}}"}},
            )
            cases.append(
                case(
                    name,
                    [f"xapi-lrs-{clause}-{family}-post-merge-reject-invalid"],
                    steps=[
                        step("create", put, expect(204)),
                        step(
                            "snapshot",
                            request("GET", path, query),
                            expect(
                                200,
                                checks=[dict(kind="bytes", hex=existing.hex())],
                                capture={"existing_etag": {"header": "ETag"}},
                            ),
                        ),
                        step(
                            "reject",
                            request("POST", path, query, {"changed": True}),
                            expect(400),
                        ),
                        step("unchanged", request("GET", path, query), preserved),
                    ],
                )
            )
        name = family + "-merge-replaces-top-level-object"
        query = dict(scope, **{key: name})
        cases.append(
            case(
                name,
                [f"xapi-lrs-{clause}-{family}-post-json-merge"],
                steps=[
                    step("create", request("PUT", path, query, original), expect(204)),
                    step(
                        "merge",
                        request(
                            "POST",
                            path,
                            query,
                            {"nested": {"new": True}, "null-is-document-data": None},
                        ),
                        expect(204),
                    ),
                    step(
                        "read",
                        request("GET", path, query),
                        expect(
                            200,
                            {
                                "": {
                                    "equals": {
                                        "keep": 1,
                                        "nested": {"new": True},
                                        "null-is-document-data": None,
                                    }
                                }
                            },
                        ),
                    ),
                ],
            )
        )
        for method in ["PUT"]:
            name = family + "-binary-" + method.lower()
            query = dict(scope, **{key: name})
            raw = bytes([0, 255, 254, 13, 10, 128, 1, 2, 0])
            put = request(method, path, query, raw_body_hex=raw.hex())
            put["headers"]["Content-Type"] = "application/octet-stream"
            put["headers"]["If-None-Match"] = "*"
            cases.append(
                case(
                    name,
                    ["xapi-lrs-4.1.6-document-no-prior-knowledge"],
                    steps=[
                        step("store", put, expect(204)),
                        step(
                            "read",
                            request("GET", path, query),
                            expect(
                                200,
                                checks=[
                                    dict(kind="bytes", hex=raw.hex()),
                                    dict(kind="header-time", header="Last-Modified"),
                                ],
                                headers={
                                    "Content-Type": {
                                        "contains": "application/octet-stream"
                                    },
                                    "ETag": {"present": True},
                                    "X-Experience-API-Version": {"equals": "2.0.0"},
                                },
                            ),
                        ),
                        step(
                            "head",
                            request("HEAD", path, query),
                            expect(200, body={"non_empty": False}),
                        ),
                        step("delete", request("DELETE", path, query), expect(204)),
                        step("missing", request("GET", path, query), expect(404)),
                    ],
                )
            )
        name = family + "-list-and-since"
        # Unique scope ensures the list assertion cannot pass due to another case.
        unique_scope = copy.deepcopy(scope)
        if "activityId" in unique_scope:
            unique_scope["activityId"] += "/" + name
        else:
            unique_scope["agent"] = dumps(
                dict(A, mbox="mailto:" + name + "@example.org")
            )
        steps = [
            step(
                "store-" + str(i),
                request(
                    "PUT", path, dict(unique_scope, **{key: name + str(i)}), {"i": i}
                ),
                expect(204),
            )
            for i in range(2)
        ]
        steps += [
            step(
                "list",
                request("GET", path, unique_scope),
                expect(
                    200,
                    {"": {"contains": name + "0"}},
                    checks=[
                        dict(
                            kind="json-shape",
                            shape={
                                "type": "array",
                                "items": {"type": "string"},
                                "min_items": 2,
                                "max_items": 2,
                            },
                        )
                    ],
                ),
            ),
            step(
                "future",
                request(
                    "GET", path, dict(unique_scope, since="2100-01-01T00:00:00.000Z")
                ),
                expect(200, {"": {"equals": []}}),
            ),
            step(
                "past",
                request(
                    "GET", path, dict(unique_scope, since="1900-01-01T00:00:00.000Z")
                ),
                expect(200, {"": {"contains": name + "1"}}),
            ),
        ]
        if family == "state":
            steps += [
                step("delete-all", request("DELETE", path, unique_scope), expect(204)),
                step(
                    "empty",
                    request("GET", path, unique_scope),
                    expect(200, {"": {"equals": []}}),
                ),
            ]
        cases.append(case(name, ["xapi-lrs-4.1.6-support-resources"], steps=steps))
        # Every scope dimension must isolate documents sharing an id.
        for dimension in [
            v
            for v in ["activityId", "agent", "registration"]
            if v in scope or v == "registration" and family == "state"
        ]:
            name = family + "-isolation-" + dimension
            q1 = dict(scope, **{key: name})
            q2 = copy.deepcopy(q1)
            if dimension == "agent":
                q2[dimension] = dumps(dict(A, mbox="mailto:other@example.org"))
            elif dimension == "registration":
                q1[dimension] = uuid_for(name + "-a")
                q2[dimension] = uuid_for(name + "-b")
            else:
                q2[dimension] += "/other"
            cases.append(
                case(
                    name,
                    ["xapi-lrs-4.1.6-support-resources"],
                    steps=[
                        step(
                            "first",
                            request("PUT", path, q1, {"value": "first"}),
                            expect(204),
                        ),
                        step(
                            "second",
                            request("PUT", path, q2, {"value": "second"}),
                            expect(204),
                        ),
                        step(
                            "read-first",
                            request("GET", path, q1),
                            expect(200, {"": {"equals": {"value": "first"}}}),
                        ),
                        step(
                            "read-second",
                            request("GET", path, q2),
                            expect(200, {"": {"equals": {"value": "second"}}}),
                        ),
                        step("delete-first", request("DELETE", path, q1), expect(204)),
                        step(
                            "second-survives",
                            request("GET", path, q2),
                            expect(200, {"": {"equals": {"value": "second"}}}),
                        ),
                    ],
                )
            )
    return cases


def parameter_cases():
    cases = []
    contexts = [
        (
            family,
            path,
            method,
            dict(scope, **{key: "parameter-probe"}),
            list(scope) + [key],
        )
        for family, path, scope, key, _ in DOCS
        for method in ["PUT", "POST", "GET", "HEAD", "DELETE"]
    ]
    contexts += [
        ("agents", "/agents", method, {"agent": dumps(A)}, ["agent"])
        for method in ["GET", "HEAD"]
    ]
    contexts += [
        ("activities", "/activities", method, {"activityId": IRI}, ["activityId"])
        for method in ["GET", "HEAD"]
    ]
    contexts += [
        (
            "statements",
            "/statements",
            "PUT",
            {"statementId": uuid_for("parameters")},
            ["statementId"],
        )
    ]
    for family, path, method, query, required in contexts:
        for field in required:
            optional_selector = (
                field == "profileId" and method in ["GET", "HEAD"]
            ) or (field == "stateId" and method in ["GET", "HEAD", "DELETE"])
            probes = ([] if optional_selector else [("missing", None)]) + [
                ("case", query[field])
            ]
            if field == "agent":
                probes += [
                    ("json-" + str(i), value)
                    for i, value in enumerate(
                        [
                            "null",
                            "[]",
                            "{}",
                            '"text"',
                            "1",
                            "false",
                            "{",
                            '{"mbox":"mailto:a@example.org","openid":"https://example.org/a"}',
                        ]
                    )
                ]
            if field == "activityId":
                probes += [
                    ("iri-" + str(i), value)
                    for i, value in enumerate(
                        ["", "no-scheme", "https://invalid host/a"]
                    )
                ]
            for suffix, value in probes:
                name = family + "-" + method.lower() + "-" + field + "-" + suffix
                wrong = dict(query)
                del wrong[field]
                if suffix == "case":
                    wrong[field.upper()] = value
                elif value is not None:
                    wrong[field] = value
                body = S if family == "statements" else {"probe": True}
                req = request(
                    method, path, wrong, body if method in ["PUT", "POST"] else None
                )
                cases.append(
                    case(
                        name,
                        ["xapi-lrs-4.1.6-request-parameter-validation"],
                        req=req,
                        exp=expect(400),
                    )
                )
    # An Agent query parameter carries the same structural table obligations
    # as a Statement Agent. Formats governed by the interpretation profile are
    # separate; these probes concern direct type/required/case/IFI duties.
    agent_contexts = [entry for entry in contexts if "agent" in entry[3]] + [
        ("statement-query", "/statements", "GET", {"agent": dumps(A)}, [])
    ]
    agent_nodes = [("extra", dict(A, unexpectedProperty=True))]
    for field in ["objectType", "name", "mbox", "mbox_sha1sum", "openid", "account"]:
        base = dict(A)
        if field in ["mbox", "mbox_sha1sum", "openid", "account"]:
            base.pop("mbox")
        for index, value in enumerate([None, False, 1, [], {}, "Agent"]):
            if value == "Agent" and field != "account":
                continue
            if field == "account" and value == {}:
                continue  # Complete-account probes follow below.
            agent_nodes.append(
                (field + "-type-" + str(index), dict(base, **{field: value}))
            )
        if field == "objectType":
            agent_nodes.append(("objectType-enum", dict(base, objectType="agent")))
        valid = {"account": {"homePage": IRI, "name": "account"}}.get(field, "Agent")
        wrong_case = dict(base)
        wrong_case.pop(field, None)
        wrong_case[field[0].upper() + field[1:]] = valid
        agent_nodes.append((field + "-key-case", wrong_case))
    for field in ["homePage", "name"]:
        for suffix, value in [
            ("missing", None),
            ("null", None),
            ("number", 3),
            ("array", []),
        ]:
            account = {"homePage": IRI, "name": "account"}
            if suffix == "missing":
                account.pop(field)
            else:
                account[field] = value
            agent_nodes.append(
                (
                    "account-" + field + "-" + suffix,
                    {"objectType": "Agent", "account": account},
                )
            )
    for family, path, method, query, _ in agent_contexts:
        for suffix, node in agent_nodes:
            name = family + "-" + method.lower() + "-agent-table-" + suffix
            req = request(
                method,
                path,
                dict(query, agent=dumps(node)),
                {"probe": True} if method in ["PUT", "POST"] else None,
            )
            cases.append(
                case(
                    name,
                    ["xapi-lrs-4.1.6-request-parameter-validation"],
                    req=req,
                    exp=expect(400),
                )
            )
    bools = ["related_agents", "related_activities", "ascending", "attachments"]
    formats = dict((key, ["TRUE", "False", "1", "0", "null", ""]) for key in bools)
    formats.update(
        limit=["-1", "1.5", "null", "bad"],
        format=["EXACT", "Canonical", "unknown", ""],
        agent=["null", "[]", "{}"],
        registration=["", "not-a-uuid"],
        since=["", "yesterday"],
        until=["", "yesterday"],
    )
    for field, values in formats.items():
        for index, value in enumerate(values):
            name = "statement-query-" + field + "-" + str(index)
            cases.append(
                case(
                    name,
                    ["xapi-lrs-4.1.6-request-parameter-validation"],
                    req=request("GET", "/statements", {field: value}),
                    exp=expect(400),
                )
            )
    for identifier in ["statementId", "voidedStatementId"]:
        for field in [
            "agent",
            "verb",
            "activity",
            "registration",
            "related_agents",
            "related_activities",
            "since",
            "until",
            "limit",
            "ascending",
        ]:
            value = dict(
                agent=dumps(A),
                verb=IRI,
                activity=IRI,
                registration=uuid_for("registration"),
                related_agents="false",
                related_activities="false",
                since="1900-01-01T00:00:00Z",
                until="2100-01-01T00:00:00Z",
                limit="1",
                ascending="false",
            )[field]
            name = "single-" + identifier + "-forbids-" + field
            cases.append(
                case(
                    name,
                    ["xapi-lrs-4.1.6.1-statement-get-single-query-limits"],
                    req=request(
                        "GET", "/statements", {identifier: uuid_for(name), field: value}
                    ),
                    exp=expect(400),
                )
            )
    return cases


def statement_cases():
    cases = []
    # The SubStatement table explicitly assigns an omitted timestamp to the
    # LRS. It does not define a SubStatement stored property or the exact
    # default instant, so check creation and UTC without inventing that value.
    name = "substatement-timestamp-omitted-is-populated"
    body = dict(
        copy.deepcopy(S),
        id=uuid_for(name),
        object=dict(copy.deepcopy(S), objectType="SubStatement"),
    )
    cases.append(
        case(
            name,
            [
                "xapi-lrs-4.2.2.3-substatement-properties",
                "xapi-lrs-4.2.7-timestamps-utc",
            ],
            steps=[
                step(
                    "store",
                    request("PUT", "/statements", {"statementId": body["id"]}, body),
                    expect(204),
                ),
                read_statement(
                    body["id"],
                    checks=[
                        dict(kind="json-time", pointer="/object/timestamp", utc=True)
                    ],
                ),
            ],
        )
    )
    # All scalar score constraints, with inclusive endpoints accepted.
    for index, score in enumerate(
        [
            {"scaled": -1},
            {"scaled": 1},
            {"raw": 1, "min": 1, "max": 2},
            {"raw": 2, "min": 1, "max": 2},
            {"raw": 0, "min": 1},
            {"raw": 2, "max": 1},
            {"min": 1, "max": 1},
            {"min": 2, "max": 1},
        ]
    ):
        name = "score-boundary-" + str(index)
        body = copy.deepcopy(S)
        body.update(id=uuid_for(name), result={"score": score})
        cases.append(
            case(
                name,
                ["xapi-lrs-4.2.2.4-score-properties"],
                req=request("PUT", "/statements", {"statementId": body["id"]}, body),
                exp=expect(204 if index < 4 else 400),
            )
        )
    # Default timestamp must equal stored, not merely exist.
    name = "lrs-populates-required-managed-fields"
    body = copy.deepcopy(S)
    body["stored"] = "2000-01-01T00:00:00.123Z"
    cases.append(
        case(
            name,
            [
                "xapi-lrs-4.2.4.2-create-statement-id",
                "xapi-lrs-4.2.4.2-timestamp-default",
                "xapi-lrs-4.2.4.2-stored-set-by-lrs",
                "xapi-lrs-4.2.4.2-authority-present",
                "xapi-lrs-4.2.4.3-version-retention",
            ],
            steps=[
                step(
                    "store",
                    request("POST", "/statements", body=body),
                    expect(
                        200,
                        checks=[
                            dict(
                                kind="json-shape",
                                shape={
                                    "type": "array",
                                    "min_items": 1,
                                    "max_items": 1,
                                    "items": {"type": "string"},
                                },
                            )
                        ],
                        capture={"id": {"pointer": "/0"}},
                    ),
                ),
                read_statement(
                    "{{id}}",
                    {
                        "/stored": {"not_equals": body["stored"]},
                        "/authority": {"present": True},
                        "/version": {"equals": "2.0.0"},
                    },
                    checks=[
                        dict(
                            kind="json-time",
                            pointer="/timestamp",
                            equals_pointer="/stored",
                            utc=True,
                        )
                    ],
                ),
            ],
        )
    )
    # Ordered POST IDs, missing id generation, batch rejection, empty results.
    bodies = [
        dict(copy.deepcopy(S), id=uuid_for("ordered-" + str(i))) for i in range(3)
    ]
    cases.append(
        case(
            "post-ids-preserve-request-order",
            ["xapi-lrs-4.1.3-json-statement-batches"],
            steps=[
                step(
                    "store",
                    request("POST", "/statements", body=bodies),
                    expect(200, {"": {"equals": [b["id"] for b in bodies]}}),
                ),
                *[
                    dict(read_statement(b["id"]), id="read-" + str(i))
                    for i, b in enumerate(bodies)
                ],
            ],
        )
    )
    for omitted in ["actor", "verb", "object"]:
        name = "batch-invalid-" + omitted
        good = dict(copy.deepcopy(S), id=uuid_for(name + "-good"))
        bad = dict(copy.deepcopy(S), id=uuid_for(name + "-bad"))
        bad.pop(omitted)
        cases.append(
            case(
                name,
                ["xapi-lrs-4.1.5-batch-rejection"],
                steps=[
                    step(
                        "reject",
                        request("POST", "/statements", body=[good, bad]),
                        expect(400),
                    ),
                    step(
                        "no-partial-write",
                        request("GET", "/statements", {"statementId": good["id"]}),
                        expect(404),
                    ),
                ],
            )
        )
    for method in ["PUT", "POST"]:
        for field, replacement in [
            ("actor", {"mbox": "mailto:other@example.org"}),
            ("verb", {"id": IRI + "/other-verb"}),
            ("object", {"id": IRI + "/other-object"}),
            ("result", {"duration": "PT9S"}),
        ]:
            name = "duplicate-" + method.lower() + "-immutable-" + field
            original = dict(
                copy.deepcopy(S), id=uuid_for(name), result={"duration": "PT1S"}
            )
            changed = dict(copy.deepcopy(original), **{field: replacement})
            query = {"statementId": original["id"]} if method == "PUT" else None
            cases.append(
                case(
                    name,
                    [
                        "xapi-lrs-4.2-statement-immutability-preserve",
                        "xapi-lrs-4.1.6.1-statement-"
                        + method.lower()
                        + "-duplicate-no-modification",
                    ],
                    steps=[
                        step(
                            "create",
                            request(
                                "PUT",
                                "/statements",
                                {"statementId": original["id"]},
                                original,
                            ),
                            expect(204),
                        ),
                        step(
                            "duplicate",
                            request(method, "/statements", query, changed),
                            expect(
                                409, status_in=[204, 200] if method == "POST" else [204]
                            ),
                        ),
                        read_statement(
                            original["id"], {"/" + field: {"equals": original[field]}}
                        ),
                    ],
                )
            )
    # Extensions are arbitrary JSON at all three allowed locations.
    for pointer in [
        "/result/extensions",
        "/context/extensions",
        "/object/definition/extensions",
    ]:
        for index, value in enumerate(
            [None, True, 12, 1.5, "", [], {}, {"nested": [None, {"unexpected": False}]}]
        ):
            name = (
                "extension-values-"
                + pointer.strip("/").replace("/", "-")
                + "-"
                + str(index)
            )
            body = set_pointer(copy.deepcopy(S), pointer, {IRI: value})
            body["id"] = uuid_for(name)
            cases.append(
                case(
                    name,
                    ["xapi-lrs-4.2.7-extension-values"],
                    steps=[
                        step(
                            "store",
                            request(
                                "PUT", "/statements", {"statementId": body["id"]}, body
                            ),
                            expect(204),
                        ),
                        read_statement(body["id"], {pointer: {"equals": {IRI: value}}}),
                    ],
                )
            )
    for role in ["parent", "grouping", "category", "other"]:
        for value in [{"id": IRI}, [{"id": IRI}], []]:
            name = (
                "context-activities-array-"
                + role
                + "-"
                + str(len(value))
                + ("single" if isinstance(value, dict) else "array")
            )
            body = dict(
                copy.deepcopy(S),
                id=uuid_for(name),
                context={"contextActivities": {role: value}},
            )
            returned = [value] if isinstance(value, dict) else value
            cases.append(
                case(
                    name,
                    ["xapi-lrs-4.2.4.2-context-activities-array"],
                    steps=[
                        step(
                            "store",
                            request(
                                "PUT", "/statements", {"statementId": body["id"]}, body
                            ),
                            expect(204),
                        ),
                        read_statement(
                            body["id"],
                            {
                                "/context/contextActivities/" + role: {
                                    "equals": returned
                                }
                            },
                        ),
                    ],
                )
            )
    return cases


def behavior_cases():
    from lrs_projection_responses import response_vectors

    return (
        resource_cases()
        + parameter_cases()
        + statement_cases()
        + filter_cases()
        + response_cases()
        + attachment_cases()
        + response_vectors()
    )


def filter_cases():
    cases = []

    def filter_ifi(kind, label):
        return {
            "mbox": "mailto:" + label + "@example.org",
            "mbox_sha1sum": hashlib.sha1(label.encode()).hexdigest(),
            "openid": IRI + "/query-agents/" + label,
            "account": {"homePage": IRI + "/query-accounts", "name": label},
        }[kind]

    agent_paths = [
        "/actor",
        "/object",
        "/context/instructor",
        "/context/team",
        "/context/contextAgents/0/agent",
        "/context/contextGroups/0/group",
        "/object/actor",
        "/object/object",
        "/object/context/instructor",
        "/object/context/team",
        "/object/context/contextAgents/0/agent",
        "/object/context/contextGroups/0/group",
    ]
    for pointer, ifi in [
        (p, k)
        for p in agent_paths
        for k in ["mbox", "mbox_sha1sum", "openid", "account"]
    ]:
        name = "agent-filter-" + pointer.strip("/").replace("/", "-")
        if ifi != "mbox":
            name += "-" + ifi
        target = {"objectType": "Agent", ifi: filter_ifi(ifi, name)}
        target_group = dict(G, member=[target])
        body = dict(copy.deepcopy(S), id=uuid_for(name))
        if pointer.startswith("/object/"):
            body["object"] = dict(objectType="SubStatement", **copy.deepcopy(S))
        if "/context/contextAgents/" in pointer:
            prefix = "/object" if pointer.startswith("/object/") else ""
            body = set_pointer(
                body,
                prefix + "/context/contextAgents",
                [{"objectType": "contextAgent", "agent": target}],
            )
        elif "/context/contextGroups/" in pointer:
            prefix = "/object" if pointer.startswith("/object/") else ""
            body = set_pointer(
                body,
                prefix + "/context/contextGroups",
                [{"objectType": "contextGroup", "group": target_group}],
            )
        else:
            body = set_pointer(
                body, pointer, target_group if pointer.endswith("/team") else target
            )
        cases.append(
            case(
                name,
                [
                    "xapi-lrs-4.1.6.1-statement-query-filters",
                    "xapi-lrs-4.1.6.1-statement-query-related-agents",
                ],
                steps=[
                    step(
                        "store",
                        request(
                            "PUT", "/statements", {"statementId": body["id"]}, body
                        ),
                        expect(204),
                    ),
                    read_statement(body["id"]),
                    step(
                        "default",
                        request(
                            "GET",
                            "/statements",
                            {
                                "agent": dumps(
                                    dict(target, name="query name need not match")
                                )
                            },
                            await_retrieval_ms=3000,
                        ),
                        expect(
                            200,
                            {
                                "/statements": {
                                    "contains"
                                    if pointer in ["/actor", "/object"]
                                    else "not_contains": {"id": body["id"]}
                                }
                            },
                        ),
                    ),
                    step(
                        "broad",
                        request(
                            "GET",
                            "/statements",
                            {"agent": dumps(target), "related_agents": "true"},
                            await_retrieval_ms=3000,
                        ),
                        expect(200, {"/statements": {"contains": {"id": body["id"]}}}),
                    ),
                ],
            )
        )
    for pointer, ifi in [
        (p, k)
        for p in ["/actor", "/object"]
        for k in ["mbox", "mbox_sha1sum", "openid", "account"]
    ]:
        name = "agent-filter-group-member-" + pointer.strip("/")
        if ifi != "mbox":
            name += "-" + ifi
        target = {"objectType": "Agent", ifi: filter_ifi(ifi, name)}
        group = {
            "objectType": "Group",
            ifi: filter_ifi(ifi, "group-" + name),
            "member": [target],
        }
        body = set_pointer(dict(copy.deepcopy(S), id=uuid_for(name)), pointer, group)
        cases.append(
            case(
                name,
                ["xapi-lrs-4.1.6.1-statement-query-filters"],
                steps=[
                    step(
                        "store",
                        request(
                            "PUT", "/statements", {"statementId": body["id"]}, body
                        ),
                        expect(204),
                    ),
                    read_statement(body["id"]),
                    step(
                        "member",
                        request(
                            "GET",
                            "/statements",
                            {"agent": dumps(target)},
                            await_retrieval_ms=3000,
                        ),
                        expect(200, {"/statements": {"contains": {"id": body["id"]}}}),
                    ),
                    step(
                        "identified-group",
                        request(
                            "GET",
                            "/statements",
                            {"agent": dumps(dict(group, name="query group"))},
                            await_retrieval_ms=3000,
                        ),
                        expect(200, {"/statements": {"contains": {"id": body["id"]}}}),
                    ),
                ],
            )
        )
    for pointer in [
        "/object",
        "/context/contextActivities/parent/0",
        "/context/contextActivities/grouping/0",
        "/context/contextActivities/category/0",
        "/context/contextActivities/other/0",
        "/object/object",
        "/object/context/contextActivities/parent/0",
        "/object/context/contextActivities/grouping/0",
        "/object/context/contextActivities/category/0",
        "/object/context/contextActivities/other/0",
    ]:
        name = "activity-filter-" + pointer.strip("/").replace("/", "-")
        activity = IRI + "/" + name
        body = dict(copy.deepcopy(S), id=uuid_for(name))
        if pointer.startswith("/object/"):
            body["object"] = dict(objectType="SubStatement", **copy.deepcopy(S))
        body = set_pointer(body, pointer, {"id": activity})
        cases.append(
            case(
                name,
                ["xapi-lrs-4.1.6.1-statement-query-related-activities"],
                steps=[
                    step(
                        "store",
                        request(
                            "PUT", "/statements", {"statementId": body["id"]}, body
                        ),
                        expect(204),
                    ),
                    read_statement(body["id"]),
                    step(
                        "default",
                        request(
                            "GET",
                            "/statements",
                            {"activity": activity},
                            await_retrieval_ms=3000,
                        ),
                        expect(
                            200,
                            {
                                "/statements": {
                                    "contains"
                                    if pointer == "/object"
                                    else "not_contains": {"id": body["id"]}
                                }
                            },
                        ),
                    ),
                    step(
                        "broad",
                        request(
                            "GET",
                            "/statements",
                            {"activity": activity, "related_activities": "true"},
                            await_retrieval_ms=3000,
                        ),
                        expect(200, {"/statements": {"contains": {"id": body["id"]}}}),
                    ),
                ],
            )
        )
    name = "statement-stored-time-boundaries"
    body = dict(
        copy.deepcopy(S),
        id=uuid_for(name),
        timestamp="2000-01-01T00:00:00.000Z",
        object={"id": IRI + "/" + name},
    )
    read = read_statement(body["id"])
    read["expect"]["capture"] = {"stored": {"pointer": "/stored"}}
    cases.append(
        case(
            name,
            ["xapi-lrs-4.1.6.1-statement-query-time-order-limit"],
            steps=[
                step(
                    "store",
                    request("PUT", "/statements", {"statementId": body["id"]}, body),
                    expect(204),
                ),
                read,
                step(
                    "since-exclusive",
                    request(
                        "GET",
                        "/statements",
                        {"activity": body["object"]["id"], "since": "{{stored}}"},
                    ),
                    expect(200, {"/statements": {"equals": []}}),
                ),
                step(
                    "until-inclusive",
                    request(
                        "GET",
                        "/statements",
                        {"activity": body["object"]["id"], "until": "{{stored}}"},
                    ),
                    expect(200, {"/statements": {"contains": {"id": body["id"]}}}),
                ),
                step(
                    "event-time-not-storage-time",
                    request(
                        "GET",
                        "/statements",
                        {"activity": body["object"]["id"], "until": body["timestamp"]},
                    ),
                    expect(200, {"/statements": {"equals": []}}),
                ),
            ],
        )
    )
    # Pagination fixture has exactly three immutable matches; no fixed cursor syntax.
    name = "statement-pagination"
    activity = IRI + "/" + name
    bodies = [
        dict(copy.deepcopy(S), id=uuid_for(name + str(i)), object={"id": activity})
        for i in range(3)
    ]
    steps = [
        step("store", request("POST", "/statements", body=bodies), expect(200))
    ] + [
        dict(read_statement(body["id"]), id="visible-" + str(i))
        for i, body in enumerate(bodies)
    ]
    for page in range(3):
        req = (
            request(
                "GET",
                "/statements",
                {"activity": activity, "limit": "1", "ascending": "true"},
            )
            if page == 0
            else request(
                "GET", "/statements", follow_more="{{more_" + str(page - 1) + "}}"
            )
        )
        exp = expect(
            200,
            checks=[
                dict(
                    kind="json-shape",
                    pointer="/statements",
                    shape={
                        "type": "array",
                        "min_items": 1,
                        "max_items": 1,
                        "items": {
                            "type": "object",
                            "required": ["id"],
                            "properties": {
                                "id": {"enum": [body["id"] for body in bodies]}
                            },
                        },
                    },
                ),
                dict(
                    kind="header-time",
                    header="Last-Modified",
                    max_pointer="/statements",
                ),
            ],
        )
        exp["capture"] = {
            "page_id_" + str(page): {"pointer": "/statements/0/id", "as_json": True}
        }
        for previous in range(page):
            exp["checks"].append(
                dict(
                    kind="json-capture-equal",
                    pointer="/statements/0/id",
                    capture="page_id_" + str(previous),
                    not_equal=True,
                )
            )
        if page < 2:
            exp["capture"]["more_" + str(page)] = {"pointer": "/more"}
            exp["json"] = {"pointers": {"/more": {"not_equals": ""}}}
        steps.append(step("page-" + str(page), req, exp))
    cases.append(
        case(name, ["xapi-lrs-4.1.6.1-statement-query-time-order-limit"], steps=steps)
    )
    # References inherit non-time filters recursively; context references do not.
    name = "statement-ref-transitive-and-context"
    activity = IRI + "/" + name
    base = dict(copy.deepcopy(S), id=uuid_for(name + "base"), object={"id": activity})
    first = dict(
        copy.deepcopy(S),
        id=uuid_for(name + "first"),
        object={"objectType": "StatementRef", "id": base["id"]},
    )
    second = dict(
        copy.deepcopy(S),
        id=uuid_for(name + "second"),
        object={"objectType": "StatementRef", "id": first["id"]},
    )
    context = dict(
        copy.deepcopy(S),
        id=uuid_for(name + "context"),
        context={"statement": {"objectType": "StatementRef", "id": base["id"]}},
    )
    steps = [
        step(
            "store",
            request("POST", "/statements", body=[base, first, second, context]),
            expect(200),
        )
    ] + [
        dict(read_statement(body["id"]), id="visible-" + str(i))
        for i, body in enumerate([base, first, second, context])
    ]
    for index, body in enumerate([base, first, second]):
        steps.append(
            step(
                "inherits-" + str(index),
                request("GET", "/statements", {"activity": activity}),
                expect(200, {"/statements": {"contains": {"id": body["id"]}}}),
            )
        )
    steps.append(
        step(
            "context-does-not-inherit",
            request("GET", "/statements", {"activity": activity}),
            expect(200, {"/statements": {"not_contains": {"id": context["id"]}}}),
        )
    )
    cases.append(case(name, ["xapi-lrs-4.1.6.1-statementref-filtering"], steps=steps))
    return cases


def response_cases():
    cases = []
    account = {
        "type": "object",
        "required": ["homePage", "name"],
        "properties": {"homePage": {"type": "string"}, "name": {"type": "string"}},
        "additional_properties": False,
    }
    person = {
        "type": "object",
        "required": ["objectType"],
        "properties": {
            "objectType": {"enum": ["Person"]},
            **{
                key: {
                    "type": "array",
                    "items": account if key == "account" else {"type": "string"},
                }
                for key in ["name", "mbox", "mbox_sha1sum", "openid", "account"]
            },
        },
        "additional_properties": False,
    }
    for field, value in [
        ("mbox", "mailto:person-probe@example.org"),
        ("mbox_sha1sum", "0123456789abcdef0123456789abcdef01234567"),
        ("openid", IRI + "/person"),
        ("account", {"homePage": IRI + "/account", "name": "person"}),
    ]:
        cases.append(
            case(
                "person-response-" + field,
                ["xapi-lrs-4.1.6.3-person-arrays"],
                req=request(
                    "GET",
                    "/agents",
                    {
                        "agent": dumps(
                            {"objectType": "Agent", field: value, "name": "人"}
                        )
                    },
                ),
                exp=expect(
                    200,
                    {"/" + field: {"contains": value}},
                    checks=[dict(kind="json-shape", shape=person)],
                ),
            )
        )
    about = {
        "type": "object",
        "required": ["version"],
        "properties": {
            "version": {"type": "array", "min_items": 1, "items": {"type": "string"}},
            "extensions": {"type": "object"},
        },
    }
    for version in ["2.0.0", "1.0.3", "2.1.0", "99.99.99"]:
        req = request("GET", "/about")
        req["headers"]["X-Experience-API-Version"] = version
        cases.append(
            case(
                "about-response-shape-" + version.replace(".", "-"),
                [
                    "xapi-lrs-4.1.6.7-about-version",
                    "xapi-lrs-4.1.6.7-about-version-header-not-rejected",
                ],
                req=req,
                exp=expect(200, checks=[dict(kind="json-shape", shape=about)]),
            )
        )
    # Omitted optional Verb display must remain omitted in exact mode.
    name = "exact-omitted-verb-display"
    body = dict(copy.deepcopy(S), id=uuid_for(name))
    cases.append(
        case(
            name,
            ["xapi-lrs-4.1.6.1-statement-format-exact"],
            steps=[
                step(
                    "store",
                    request("PUT", "/statements", {"statementId": body["id"]}, body),
                    expect(204),
                ),
                read_statement(body["id"], {"/verb/display": {"present": False}}),
            ],
        )
    )
    # Credential mapping comparison accepts arbitrary valid authority shapes.
    name = "authority-mapping-is-deterministic"
    first = dict(copy.deepcopy(S), id=uuid_for(name + "first"))
    second = dict(copy.deepcopy(S), id=uuid_for(name + "second"))
    cases.append(
        case(
            name,
            ["xapi-lrs-4.2.4.2-authority-deterministic"],
            steps=[
                step(
                    "store-first",
                    request(
                        "PUT",
                        "/statements",
                        {"statementId": first["id"]},
                        first,
                        auth_profile="statement_writer_a",
                    ),
                    expect(204),
                ),
                dict(
                    read_statement(first["id"]),
                    id="capture-authority",
                    expect=expect(
                        200,
                        capture={
                            "authority": {"pointer": "/authority", "as_json": True}
                        },
                    ),
                ),
                step(
                    "store-second",
                    request(
                        "PUT",
                        "/statements",
                        {"statementId": second["id"]},
                        second,
                        auth_profile="statement_writer_a",
                    ),
                    expect(204),
                ),
                read_statement(
                    second["id"],
                    checks=[
                        dict(
                            kind="json-capture-equal",
                            pointer="/authority",
                            capture="authority",
                        )
                    ],
                ),
            ],
        )
    )
    # Reordered object members remain valid; JSON ordering has no meaning.
    name = "statement-member-order-is-unrestricted"
    body = dict(copy.deepcopy(S), id=uuid_for(name))
    reordered = {key: body[key] for key in reversed(body)}
    alphabetical = dict(body, id=uuid_for(name + "-alphabetical"))
    cases.append(
        case(
            name,
            ["xapi-lrs-4.2.4.2-return-serialization"],
            steps=[
                step("store-alphabetical-order", request(
                    "PUT", "/statements", {"statementId": alphabetical["id"]},
                    raw_body=dumps({key: alphabetical[key] for key in sorted(alphabetical)}),
                    headers={"Content-Type": "application/json", "X-Experience-API-Version": "2.0.0"},
                ), expect(204)),
                step("store-reversed-order", request(
                    "PUT", "/statements", {"statementId": body["id"]},
                    raw_body=dumps(reordered),
                    headers={"Content-Type": "application/json", "X-Experience-API-Version": "2.0.0"},
                ), expect(204)),
                read_statement(body["id"]),
            ],
        )
    )
    return cases


def attachment_cases():
    cases = []
    data = b"portable attachment\r\nwith a second line\x00end"
    digest = hashlib.sha256(data).hexdigest()
    metadata = {
        "usageType": IRI + "/attachment",
        "display": {"en": "attachment"},
        "contentType": "text/plain",
        "length": len(data),
        "sha2": digest,
    }
    for method in ["PUT", "POST"]:
        for mode in [
            "json-file-url",
            "multipart-no-attachments",
            "multipart-file-url",
            "multipart-upload",
            "multipart-shared",
        ]:
            if method == "PUT" and mode == "multipart-shared":
                continue
            name = "attachment-" + method.lower() + "-" + mode
            body = dict(copy.deepcopy(S), id=uuid_for(name))
            if mode != "multipart-no-attachments":
                body["attachments"] = [
                    dict(
                        metadata,
                        **(
                            {"fileUrl": IRI + "/attachment-location"}
                            if "file-url" in mode
                            else {}
                        ),
                    )
                ]
            bodies = (
                [body, dict(copy.deepcopy(body), id=uuid_for(name + "second"))]
                if mode == "multipart-shared"
                else (body if method == "PUT" else [body])
            )
            query = {"statementId": body["id"]} if method == "PUT" else None
            req = request(method, "/statements", query, bodies)
            boundary = "lrs-conformance-attachment"
            if mode.startswith("multipart"):
                raw = (
                    b"--"
                    + boundary.encode()
                    + b"\r\nContent-Type: application/json\r\n\r\n"
                    + dumps(bodies).encode()
                    + b"\r\n"
                )
                if mode in ["multipart-upload", "multipart-shared"]:
                    raw += (
                        b"--"
                        + boundary.encode()
                        + b"\r\nContent-Type: text/plain\r\nContent-Transfer-Encoding: binary\r\nX-Experience-API-Hash: "
                        + digest.encode()
                        + b"\r\nContent-Length: "
                        + str(len(data)).encode()
                        + b"\r\n\r\n"
                        + data
                        + b"\r\n"
                    )
                raw += b"--" + boundary.encode() + b"--\r\n"
                req.pop("body")
                req["raw_body_hex"] = raw.hex()
                req["headers"]["Content-Type"] = "multipart/mixed; boundary=" + boundary
            steps = [
                step("store", req, expect(204 if method == "PUT" else 200)),
                read_statement(body["id"]),
            ]
            get = request(
                "GET",
                "/statements",
                {"statementId": body["id"], "attachments": "true"},
                await_retrieval_ms=5000,
            )
            steps.append(
                step(
                    "multipart-return",
                    get,
                    expect(
                        200,
                        checks=[
                            dict(
                                kind="multipart",
                                expected_data=[data.decode("utf-8")]
                                if mode in ["multipart-upload", "multipart-shared"]
                                else [],
                            )
                        ],
                    ),
                )
            )
            get = request(
                "GET",
                "/statements",
                {"statementId": body["id"], "attachments": "false"},
            )
            exp = expect(
                200,
                checks=[
                    dict(
                        kind="json-shape",
                        shape={
                            "type": "object",
                            "required": ["id", "actor", "verb", "object"],
                        },
                    )
                ],
            )
            exp["headers"]["Content-Type"] = {"contains": "application/json"}
            steps.append(step("json-return", get, exp))
            cases.append(
                case(
                    name,
                    [
                        "xapi-lrs-4.1.3-multipart-accept-valid",
                        "xapi-lrs-4.1.3-attachment-transmission-criteria",
                        "xapi-lrs-4.1.6.1-statement-get-attachments-true",
                        "xapi-lrs-4.1.6.1-statement-get-attachments-false",
                    ],
                    steps=steps,
                )
            )
        for fault in [
            "first-media-type",
            "missing-hash",
            "wrong-hash",
            "wrong-content-type",
            "wrong-length",
            "missing-part",
            "json-without-file-url",
        ]:
            name = "attachment-" + method.lower() + "-reject-" + fault
            body = dict(copy.deepcopy(S), id=uuid_for(name), attachments=[metadata])
            query = {"statementId": body["id"]} if method == "PUT" else None
            if fault == "json-without-file-url":
                req = request(method, "/statements", query, body)
            else:
                boundary = "lrs-conformance-invalid-attachment"
                lines = [
                    "--" + boundary,
                    "Content-Type: "
                    + (
                        "text/plain"
                        if fault == "first-media-type"
                        else "application/json"
                    ),
                    "",
                ]
                raw = (
                    "\r\n".join(lines).encode()
                    + b"\r\n"
                    + dumps(body).encode()
                    + b"\r\n"
                )
                if fault != "missing-part":
                    lines = [
                        "--" + boundary,
                        "Content-Type: "
                        + (
                            "image/png"
                            if fault == "wrong-content-type"
                            else "text/plain"
                        ),
                        "Content-Transfer-Encoding: binary",
                    ]
                    if fault != "missing-hash":
                        lines.append(
                            "X-Experience-API-Hash: "
                            + ("0" * 64 if fault == "wrong-hash" else digest)
                        )
                    lines.append(
                        "Content-Length: "
                        + str(len(data) + 1 if fault == "wrong-length" else len(data))
                    )
                    raw += "\r\n".join(lines).encode() + b"\r\n\r\n" + data + b"\r\n"
                raw += ("--" + boundary + "--\r\n").encode()
                req = request(method, "/statements", query, raw_body_hex=raw.hex())
                req["headers"]["Content-Type"] = "multipart/mixed; boundary=" + boundary
            cases.append(
                case(
                    name,
                    [
                        "xapi-lrs-4.1.3-attachment-header-property-match",
                        "xapi-lrs-4.1.3-multipart-reject-unmatched",
                    ],
                    req=req,
                    exp=expect(400),
                )
            )
    return cases
