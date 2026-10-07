"""Focused vectors for mandatory findings in the 2026-10-05 cross-review."""

from __future__ import annotations

import copy
import hashlib
from itertools import permutations

from generate_lrs_projection_cases import (
    IRI,
    case,
    dumps,
    expect,
    read_statement,
    request,
    step,
    uuid_for,
)
from lrs_projection_behavior import A, DOCS, S


def review_round_two_cases():
    """N1/N2: each selector must distinguish otherwise matching fixtures."""
    cases = []
    for family, path, _, key, _ in DOCS:
        for registered in [False, True] if family == "state" else [False]:
            name = (
                family
                + "-list-scope"
                + (
                    "-registered"
                    if registered
                    else "-all-registrations"
                    if family == "state"
                    else ""
                )
            )
            actor = dict(A, mbox="mailto:" + name + "@example.org")
            scope = {"agent": dumps(actor)} if family != "activity-profile" else {}
            if family != "agent-profile":
                scope["activityId"] = IRI + "/" + name
            reg = uuid_for(name + "-registration")
            selected = dict(scope, **({"registration": reg} if registered else {}))
            # State has four ids in this Activity/Agent: two registration A,
            # one registration B, and one unregistered. Profiles have two ids.
            local = [
                dict(
                    scope,
                    **{key: name + "-target-" + str(i)},
                    **({"registration": reg} if family == "state" else {}),
                )
                for i in range(2)
            ]
            if family == "state":
                local += [
                    dict(
                        scope,
                        stateId=name + "-other-registration",
                        registration=uuid_for(name + "-other"),
                    ),
                    dict(scope, stateId=name + "-unregistered"),
                ]
            outsiders = []
            for dimension in [
                field for field in ["agent", "activityId"] if field in scope
            ]:
                other = dict(scope, **{key: name + "-outside-" + dimension})
                other[dimension] = (
                    dumps(dict(A, mbox="mailto:outside-" + name + "@example.org"))
                    if dimension == "agent"
                    else IRI + "/outside-" + name
                )
                if family == "state":
                    other["registration"] = reg
                outsiders.append(other)
            expected_ids = [
                entry[key]
                for entry in local
                if not registered or entry.get("registration") == reg
            ]
            steps = [
                step(
                    "create-" + str(i),
                    request("PUT", path, query, {"scope": name, "value": i}),
                    expect(204),
                )
                for i, query in enumerate(local + outsiders)
            ]
            # List order is unspecified. Enumerate the 2 or 4 element
            # permutations to require every id exactly once and no outsider.
            steps.append(
                step(
                    "list-only-selected-scope",
                    request("GET", path, selected),
                    expect(
                        200,
                        checks=[
                            dict(
                                kind="json-shape",
                                shape={
                                    "type": "array",
                                    "enum": [
                                        list(order)
                                        for order in permutations(expected_ids)
                                    ],
                                },
                            )
                        ],
                    ),
                )
            )
            cases.append(case(name, ["xapi-lrs-4.1.6-support-resources"], steps=steps))

    def filtered_read(label, query, identifier):
        return step(
            label,
            request("GET", "/statements", query, await_retrieval_ms=5000),
            expect(
                200,
                {"/statements": {"contains": {"id": identifier}}},
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
                                "properties": {"id": {"enum": [identifier]}},
                            },
                        },
                    )
                ],
            ),
        )

    for field in ["verb", "registration"]:
        name = "statement-query-isolates-" + field
        base = dict(
            copy.deepcopy(S),
            actor=dict(A, mbox="mailto:" + name + "@example.org"),
            verb={"id": IRI + "/" + name},
            object={"id": IRI + "/" + name},
            context={"registration": uuid_for(name + "-same")},
        )
        values = [
            IRI + "/" + name + "/" + str(i)
            if field == "verb"
            else uuid_for(name + "-" + str(i))
            for i in range(2)
        ]
        bodies = []
        for i, value in enumerate(values + ([None] if field == "registration" else [])):
            body = dict(copy.deepcopy(base), id=uuid_for(name + "-body-" + str(i)))
            if field == "verb":
                body["verb"]["id"] = value
            elif value:
                body["context"]["registration"] = value
            else:
                body["context"].pop("registration")
            bodies.append(body)
        steps = [
            step(
                "store-" + str(i),
                request("PUT", "/statements", {"statementId": body["id"]}, body),
                expect(204),
            )
            for i, body in enumerate(bodies)
        ]
        for i, body in enumerate(bodies):
            read = read_statement(body["id"])
            read["id"] = "visible-" + str(i)
            steps.append(read)
        steps += [
            filtered_read("filter-" + str(i), {field: value}, bodies[i]["id"])
            for i, value in enumerate(values)
        ]
        cases.append(
            case(name, ["xapi-lrs-4.1.6.1-statement-query-filters"], steps=steps)
        )

    # In a conjunction, each negative fixture differs in exactly one selector.
    name = "statement-query-conjunction-isolates-each-filter"
    body = dict(
        copy.deepcopy(S),
        id=uuid_for(name),
        actor=dict(A, mbox="mailto:" + name + "@example.org"),
        verb={"id": IRI + "/" + name + "/verb"},
        object={"id": IRI + "/" + name + "/activity"},
        context={"registration": uuid_for(name + "-registration")},
    )
    query = {
        "agent": dumps(body["actor"]),
        "verb": body["verb"]["id"],
        "activity": body["object"]["id"],
        "registration": body["context"]["registration"],
    }
    bodies = [body]
    queries = [query]
    for field in query:
        other = dict(copy.deepcopy(body), id=uuid_for(name + "-" + field))
        if field == "agent":
            other["actor"]["mbox"] = "mailto:other-" + name + "@example.org"
            value = dumps(other["actor"])
        elif field == "registration":
            other["context"]["registration"] = value = uuid_for(
                name + "-other-registration"
            )
        else:
            other["verb" if field == "verb" else "object"]["id"] += "/other"
            value = other["verb" if field == "verb" else "object"]["id"]
        bodies.append(other)
        queries.append(dict(query, **{field: value}))
    steps = [
        step(
            "store-" + str(i),
            request("PUT", "/statements", {"statementId": entry["id"]}, entry),
            expect(204),
        )
        for i, entry in enumerate(bodies)
    ]
    for i, entry in enumerate(bodies):
        read = read_statement(entry["id"])
        read["id"] = "visible-" + str(i)
        steps.append(read)
    steps += [
        filtered_read("conjunction-" + str(i), selectors, entry["id"])
        for i, (selectors, entry) in enumerate(zip(queries, bodies, strict=True))
    ]
    cases.append(case(name, ["xapi-lrs-4.1.6.1-statement-query-filters"], steps=steps))
    return cases


def review_repair_cases():
    cases = []
    # DELETE with registration selects that registration; absent registration
    # selects all registrations for this Activity/Agent (recorded judgment).
    for registered in [False, True]:
        name = "state-delete-all-" + (
            "registered" if registered else "all-registrations"
        )
        scope = {"activityId": IRI + "/" + name, "agent": dumps(A)}
        reg = uuid_for(name + "-registration")
        deleted_scope = dict(scope, **({"registration": reg} if registered else {}))
        targets = [dict(deleted_scope, stateId=name + "-" + str(i)) for i in range(2)]
        if not registered:
            targets += [
                dict(
                    scope,
                    stateId=name + "-registered-" + str(i),
                    registration=uuid_for(name + str(i)),
                )
                for i in range(2)
            ]
        survivors = [
            dict(
                deleted_scope,
                stateId=name,
                agent=dumps(dict(A, mbox="mailto:other-delete-scope@example.org")),
            ),
            dict(deleted_scope, stateId=name, activityId=IRI + "/other-" + name),
        ]
        if registered:
            survivors += [
                dict(scope, stateId=name),
                dict(scope, stateId=name, registration=uuid_for(name + "-other")),
            ]
        steps = []
        for i, query in enumerate(targets + survivors):
            raw = (name + "-document-" + str(i)).encode()
            steps.append(
                step(
                    "create-" + str(i),
                    request(
                        "PUT",
                        "/activities/state",
                        query,
                        raw_body_hex=raw.hex(),
                        headers={
                            "Content-Type": "application/octet-stream",
                            "X-Experience-API-Version": "2.0.0",
                        },
                    ),
                    expect(204),
                )
            )
        steps.append(
            step(
                "delete-context",
                request("DELETE", "/activities/state", deleted_scope),
                expect(204),
            )
        )
        for i, query in enumerate(targets):
            steps.append(
                step(
                    "target-absent-" + str(i),
                    request("GET", "/activities/state", query),
                    expect(404),
                )
            )
        for i, query in enumerate(survivors, len(targets)):
            steps.append(
                step(
                    "other-context-survives-" + str(i),
                    request("GET", "/activities/state", query),
                    expect(
                        200,
                        checks=[
                            dict(
                                kind="bytes",
                                hex=(name + "-document-" + str(i)).encode().hex(),
                            )
                        ],
                    ),
                )
            )
        cases.append(case(name, ["xapi-lrs-4.1.6-support-resources"], steps=steps))

    # Every specified parameter name is case sensitive, including optional
    # registration and list-only since. Use legal values to isolate key case.
    values = dict(
        statementId=uuid_for("query-case"),
        voidedStatementId=uuid_for("query-case"),
        agent=dumps(A),
        verb=IRI,
        activity=IRI,
        registration=uuid_for("query-registration"),
        related_activities="true",
        related_agents="true",
        since="2000-01-01T00:00:00Z",
        until="2100-01-01T00:00:00Z",
        limit="1",
        format="exact",
        attachments="false",
        ascending="true",
    )
    contexts = [("statements", "/statements", {}, values, ["GET", "HEAD"])]
    contexts += [
        (
            "state-registration",
            "/activities/state",
            {"activityId": IRI, "agent": dumps(A)},
            {"registration": values["registration"]},
            ["PUT", "POST", "GET", "HEAD", "DELETE"],
        )
    ]
    contexts += [
        (
            "state-list",
            "/activities/state",
            {"activityId": IRI, "agent": dumps(A)},
            {"since": values["since"]},
            ["GET", "HEAD"],
        )
    ]
    contexts += [
        (
            "agent-profile-list",
            "/agents/profile",
            {"agent": dumps(A)},
            {"since": values["since"]},
            ["GET", "HEAD"],
        ),
        (
            "activity-profile-list",
            "/activities/profile",
            {"activityId": IRI},
            {"since": values["since"]},
            ["GET", "HEAD"],
        ),
    ]
    for family, path, scope, fields, methods in contexts:
        for field, value in fields.items():
            for method in methods:
                query = dict(scope, **{field[0].upper() + field[1:]: value})
                if method in ["PUT", "POST"]:
                    query["stateId"] = "case-probe"
                name = "parameter-case-" + family + "-" + method.lower() + "-" + field
                cases.append(
                    case(
                        name,
                        ["xapi-lrs-4.1.5-parameter-case"],
                        req=request(
                            method,
                            path,
                            query,
                            {"probe": True} if method in ["PUT", "POST"] else None,
                        ),
                        exp=expect(400),
                    )
                )

    for mixed in [False, True]:
        name = "post-generated-ids-" + ("mixed-order" if mixed else "all-omitted-order")
        bodies = [
            dict(copy.deepcopy(S), object={"id": IRI + "/" + name + "/" + str(i)})
            for i in range(3)
        ]
        if mixed:
            bodies[1]["id"] = uuid_for(name)
        capture = {"id_" + str(i): {"pointer": "/" + str(i)} for i in range(3)}
        checks = [
            dict(
                kind="json-shape",
                shape={
                    "type": "array",
                    "min_items": 3,
                    "max_items": 3,
                    "items": {"type": "string", "format": "uuid"},
                },
            )
        ]
        steps = [
            step(
                "post-batch",
                request("POST", "/statements", body=bodies),
                expect(
                    200,
                    {"/1": {"equals": bodies[1]["id"]}} if mixed else None,
                    checks=checks,
                    capture=capture,
                ),
            )
        ]
        for i, body in enumerate(bodies):
            read = read_statement(
                "{{id_" + str(i) + "}}",
                {"/object/id": {"equals": body["object"]["id"]}},
            )
            read["id"] = "read-position-" + str(i)
            steps.append(read)
        cases.append(
            case(
                name,
                [
                    "xapi-lrs-4.1.3-json-statement-batches",
                    "xapi-lrs-4.2.4.2-create-statement-id",
                ],
                steps=steps,
            )
        )

    # Distinct content and metadata expose both missing parts and array reorder.
    data = [b"first attachment\x00\r\n", b"second different attachment\xff\x80"]
    for multipart in [False, True]:
        name = "attachment-two-" + ("multipart" if multipart else "file-urls")
        metadata = [
            dict(
                usageType=IRI + "/attachment/" + str(i),
                display={"en": "attachment " + str(i)},
                contentType="application/octet-stream",
                length=len(raw),
                sha2=hashlib.sha256(raw).hexdigest(),
                **({} if multipart else {"fileUrl": IRI + "/file/" + str(i)}),
            )
            for i, raw in enumerate(data)
        ]
        body = dict(
            copy.deepcopy(S),
            id=uuid_for(name),
            verb={"id": IRI + "/" + name},
            attachments=metadata,
        )
        write = request("PUT", "/statements", {"statementId": body["id"]}, body)
        if multipart:
            boundary = "two-distinct-attachments"
            raw = (
                b"--"
                + boundary.encode()
                + b"\r\nContent-Type: application/json\r\n\r\n"
                + dumps(body).encode()
                + b"\r\n"
            )
            for content, meta in zip(data, metadata, strict=True):
                raw += (
                    b"--"
                    + boundary.encode()
                    + b"\r\nContent-Type: application/octet-stream\r\nContent-Transfer-Encoding: binary\r\nX-Experience-API-Hash: "
                    + meta["sha2"].encode()
                    + b"\r\n\r\n"
                    + content
                    + b"\r\n"
                )
            raw += b"--" + boundary.encode() + b"--\r\n"
            write.pop("body")
            write["raw_body_hex"] = raw.hex()
            write["headers"]["Content-Type"] = "multipart/mixed; boundary=" + boundary
        steps = [
            step("store", write, expect(204)),
            read_statement(body["id"], {"/attachments": {"equals": metadata}}),
        ]
        if multipart:
            for suffix, query in [
                ("single", {"statementId": body["id"]}),
                ("result", {"verb": body["verb"]["id"]}),
            ]:
                steps.append(
                    step(
                        "all-parts-" + suffix,
                        request(
                            "GET",
                            "/statements",
                            dict(query, attachments="true"),
                            await_retrieval_ms=5000,
                        ),
                        expect(
                            200,
                            checks=[
                                dict(
                                    kind="multipart",
                                    expected_data_hex=[raw.hex() for raw in data],
                                )
                            ],
                        ),
                    )
                )
        cases.append(
            case(
                name,
                [
                    "xapi-lrs-4.1.3-attachment-transmission-criteria",
                    "xapi-lrs-4.1.6.1-statement-get-attachments-true",
                    "xapi-lrs-4.2-statement-immutability-preserve",
                ],
                steps=steps,
            )
        )
    return cases
