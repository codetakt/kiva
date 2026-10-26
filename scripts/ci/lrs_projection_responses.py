"""Recursive response and timestamp vectors, based on the public LRS source."""

from __future__ import annotations

import copy
import hashlib
import itertools

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


def identity_shape(value):
    """Minimum identifying data; explicit default objectType is also accepted."""
    kind = value.get("objectType", "Activity")
    fields = {
        "objectType": {"type": "string", "enum": [kind]},
    }
    required = []
    if kind in ["Activity", "Verb"]:
        required.append("id")
        fields["id"] = {"type": "string", "enum": [value["id"]]}
        if kind == "Verb":
            fields.pop("objectType")
    else:
        for field in ["mbox", "mbox_sha1sum", "openid", "account"]:
            if field in value:
                required.append(field)
                fields[field] = {"enum": [value[field]]}
        if kind == "Group":
            required.append("objectType")
            if not any(field in value for field in required if field != "objectType"):
                required.append("member")
                fields["member"] = {
                    "type": "array",
                    "min_items": len(value["member"]),
                    "max_items": len(value["member"]),
                    "items": identity_shape(value["member"][0]),
                }
    return {
        "type": "object",
        "required": required,
        "properties": fields,
        "additional_properties": False,
    }


def format_cases():
    cases = []
    for ifi, value in [
        ("mbox", "mailto:recursive-format@example.org"),
        ("mbox_sha1sum", "0123456789abcdef0123456789abcdef01234567"),
        ("openid", IRI + "/recursive-agent"),
        ("account", {"homePage": IRI + "/recursive-account", "name": "learner"}),
    ]:
        agent = {"objectType": "Agent", ifi: value, "name": "learner 人"}
        identified = {
            "objectType": "Group",
            "account": {"homePage": IRI + "/recursive-group", "name": "group"},
            "name": "identified group",
            "member": [agent],
        }
        anonymous = {
            "objectType": "Group",
            "name": "anonymous group",
            "member": [agent],
        }
        for object_kind in [
            "activity",
            "agent",
            "identified-group",
            "anonymous-group",
            "substatement",
        ]:
            name = "statement-recursive-formats-" + ifi + "-" + object_kind
            activity = {
                "objectType": "Activity",
                "id": IRI + "/recursive-activity/" + name,
                "definition": {
                    "name": {"en": "activity", "ja": "学習"},
                    "description": {"en": "description", "ja": "説明"},
                    "type": IRI + "/type",
                },
            }
            verb = {
                "id": IRI + "/recursive-verb/" + name,
                "display": {"en": "did", "ja": "実行"},
            }
            context = {
                "instructor": agent,
                "team": identified,
                "contextAgents": [
                    {
                        "objectType": "contextAgent",
                        "agent": agent,
                        "relevantTypes": [IRI + "/relationship"],
                    }
                ],
                "contextGroups": [
                    {
                        "objectType": "contextGroup",
                        "group": anonymous,
                        "relevantTypes": [IRI + "/relationship"],
                    }
                ],
                "contextActivities": {
                    role: [activity]
                    for role in ["parent", "grouping", "category", "other"]
                },
            }
            obj = {
                "activity": activity,
                "agent": agent,
                "identified-group": identified,
                "anonymous-group": anonymous,
                "substatement": {
                    "objectType": "SubStatement",
                    "actor": anonymous,
                    "verb": verb,
                    "object": activity,
                    "context": context,
                },
            }[object_kind]
            body = {
                "id": uuid_for(name),
                "actor": identified,
                "verb": verb,
                "object": obj,
                "context": context,
            }
            entities = {"/actor": identified, "/verb": verb}
            identities = {
                "/actor": identity_shape(identified),
                "/verb": identity_shape(dict(verb, objectType="Verb")),
            }
            contexts = [("/context", context)]
            if object_kind == "substatement":
                # The complete context is inside the SubStatement; keep the
                # request below the generic suite's declared size fixture.
                body.pop("context")
                contexts = []
                entities.update(
                    {
                        "/object/actor": anonymous,
                        "/object/verb": verb,
                        "/object/object": activity,
                    }
                )
                identities.update(
                    {
                        "/object/actor": identity_shape(anonymous),
                        "/object/verb": identity_shape(dict(verb, objectType="Verb")),
                        "/object/object": identity_shape(activity),
                    }
                )
                contexts.append(("/object/context", context))
            else:
                entities["/object"] = obj
                identities["/object"] = identity_shape(obj)
            for pointer, ctx in contexts:
                for field in ["instructor", "team"]:
                    entities[pointer + "/" + field] = ctx[field]
                    identities[pointer + "/" + field] = identity_shape(ctx[field])
                for field, child in [
                    ("contextAgents", "agent"),
                    ("contextGroups", "group"),
                ]:
                    location = pointer + "/" + field + "/0/" + child
                    entities[location] = ctx[field][0][child]
                    identities[location] = identity_shape(ctx[field][0][child])
                for role in ["parent", "grouping", "category", "other"]:
                    location = pointer + "/contextActivities/" + role + "/0"
                    entities[location] = activity
                    identities[location] = identity_shape(activity)
            for fmt in [None, "exact", "ids", "canonical"]:
                vector_name = name + "-" + (fmt or "default")
                vector_body = dict(copy.deepcopy(body), id=uuid_for(vector_name))
                read = read_statement(vector_body["id"])
                read["id"] = "read-" + (fmt or "default")
                if fmt:
                    read["request"]["query"]["format"] = fmt
                if fmt in [None, "exact"]:
                    requirements = ["xapi-lrs-4.1.6.1-statement-format-exact"]
                    read["expect"]["json"] = {
                        "pointers": {
                            "/id": {"equals": vector_body["id"]},
                            **{
                                pointer: {"equals": entity}
                                for pointer, entity in entities.items()
                            },
                        }
                    }
                elif fmt == "ids":
                    requirements = ["xapi-lrs-4.1.6.1-statement-format-ids"]
                    read["expect"]["checks"].extend(
                        dict(kind="json-shape", pointer=pointer, shape=shape)
                        for pointer, shape in identities.items()
                    )
                else:
                    requirements = [
                        "xapi-lrs-4.1.6.1-statement-format-canonical-agents",
                        "xapi-lrs-4.1.6.1-canonical-language-map-one-language",
                    ]
                    read["expect"]["json"] = {
                        "pointers": {
                            "/id": {"equals": vector_body["id"]},
                            **{
                                pointer: {"equals": entity}
                                for pointer, entity in entities.items()
                                if entity.get("objectType") in ["Agent", "Group"]
                            },
                        }
                    }
                    read["expect"]["checks"].append(dict(kind="language-maps"))
                cases.append(
                    case(
                        vector_name,
                        requirements,
                        steps=[
                            step(
                                "store",
                                request(
                                    "PUT",
                                    "/statements",
                                    {"statementId": vector_body["id"]},
                                    vector_body,
                                ),
                                expect(204),
                            ),
                            read,
                        ],
                    )
                )
    # Group member order is explicitly outside immutability. Exercise more
    # than one member without imposing an array order on the response.
    members = [
        {"objectType": "Agent", "mbox": "mailto:member-one@example.org", "name": "One"},
        {"objectType": "Agent", "openid": IRI + "/member-two", "name": "Two"},
    ]
    for fmt in [None, "exact", "ids", "canonical"]:
        name = "statement-recursive-formats-multiple-group-members-" + (
            fmt or "default"
        )
        body = {
            "id": uuid_for(name),
            "actor": {"objectType": "Group", "name": "Two members", "member": members},
            "verb": {"id": IRI + "/did"},
            "object": {"id": IRI + "/multi-member-activity"},
        }
        if fmt == "ids":
            choices = [
                [
                    {key: value for key, value in member.items() if key != "name"},
                    {
                        key: value
                        for key, value in member.items()
                        if key not in ["name", "objectType"]
                    },
                ]
                for member in members
            ]
            arrays = [
                list(order)
                for selection in itertools.product(*choices)
                for order in itertools.permutations(selection)
            ]
            group_shape = {
                "type": "object",
                "required": ["objectType", "member"],
                "additional_properties": False,
                "properties": {
                    "objectType": {"enum": ["Group"]},
                    "member": {"enum": arrays},
                },
            }
        else:
            group_shape = {
                "type": "object",
                "required": ["objectType", "name", "member"],
                "properties": {
                    "objectType": {"enum": ["Group"]},
                    "name": {"enum": ["Two members"]},
                    "member": {"enum": [members, list(reversed(members))]},
                },
            }
        read = read_statement(body["id"])
        if fmt:
            read["request"]["query"]["format"] = fmt
        read["expect"]["checks"].append(
            dict(kind="json-shape", pointer="/actor", shape=group_shape)
        )
        requirement = {
            "ids": "xapi-lrs-4.1.6.1-statement-format-ids",
            "canonical": "xapi-lrs-4.1.6.1-statement-format-canonical-agents",
        }.get(fmt, "xapi-lrs-4.1.6.1-statement-format-exact")
        cases.append(
            case(
                name,
                [requirement],
                steps=[
                    step(
                        "store",
                        request(
                            "PUT", "/statements", {"statementId": body["id"]}, body
                        ),
                        expect(204),
                    ),
                    read,
                ],
            )
        )
    return cases


def timestamp_cases():
    cases = []
    for location in ["top", "substatement"]:
        for index, timestamp in enumerate(
            [
                "2025-01-01T09:00:00.123+09:00",
                "2024-12-31T18:30:00.987-05:30",
                "2025-01-01T09:00:00.123456789+09:00",
                "2025-01-01T09:00:00.999987+09:00",
            ]
        ):
            name = "statement-timestamp-precision-" + location + "-" + str(index)
            base = {
                "actor": {"mbox": "mailto:timestamp-precision@example.org"},
                "verb": {"id": IRI + "/verb"},
                "object": {"id": IRI + "/activity"},
            }
            body = dict(copy.deepcopy(base), id=uuid_for(name))
            if location == "top":
                body["timestamp"] = timestamp
                pointer = "/timestamp"
            else:
                body["object"] = dict(
                    copy.deepcopy(base), objectType="SubStatement", timestamp=timestamp
                )
                pointer = "/object/timestamp"
            cases.append(
                case(
                    name,
                    ["xapi-lrs-4.2.7-timestamps-utc"],
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
                            checks=[
                                dict(
                                    kind="json-time",
                                    pointer=pointer,
                                    expected=timestamp,
                                    utc=True,
                                    allow_truncation_or_rounding=True,
                                )
                            ],
                        ),
                    ],
                )
            )
    return cases


def mime_cases():
    cases = []
    for variation in [
        "quoted-boundary",
        "preamble-epilogue",
        "folded-headers",
        "mixed-case-headers",
        "binary-delimiter-like",
    ]:
        name = "attachment-rfc-framing-" + variation
        boundary = "independent-mime-vector-outer"
        data = (
            b"\xff\x00\x80\r\n--independent-mime-vector-not-a-delimiter\r\nend"
            if variation == "binary-delimiter-like"
            else b"portable\x00binary\xffbytes"
        )
        digest = hashlib.sha256(data).hexdigest()
        body = {
            "id": uuid_for(name),
            "actor": {"mbox": "mailto:mime-rfc@example.org"},
            "verb": {"id": IRI + "/verb"},
            "object": {"id": IRI + "/activity"},
            "attachments": [
                {
                    "usageType": IRI + "/attachment",
                    "display": {"en": "binary"},
                    "contentType": "application/octet-stream",
                    "length": len(data),
                    "sha2": digest,
                }
            ],
        }
        json_header = "Content-Type: application/json"
        attachment_headers = [
            "Content-Type: application/octet-stream",
            "Content-Transfer-Encoding: binary",
            "X-Experience-API-Hash: " + digest,
        ]
        if variation == "folded-headers":
            json_header = "Content-Type:\r\n application/json"
            attachment_headers[-1] = "X-Experience-API-Hash:\r\n " + digest
        if variation == "mixed-case-headers":
            json_header = "cOnTeNt-TyPe: APPLICATION/JSON"
            attachment_headers = [
                "cOnTeNt-TyPe: application/octet-stream",
                "cOnTeNt-TrAnSfEr-EnCoDiNg: BINARY",
                "x-eXpErIeNcE-aPi-HaSh: " + digest,
            ]
        raw = (
            (
                "--"
                + boundary
                + "\r\n"
                + json_header
                + "\r\n\r\n"
                + dumps(body)
                + "\r\n--"
                + boundary
                + "\r\n"
                + "\r\n".join(attachment_headers)
                + "\r\n\r\n"
            ).encode()
            + data
            + ("\r\n--" + boundary + "--\r\n").encode()
        )
        if variation == "preamble-epilogue":
            raw = b"MIME preamble\r\n" + raw + b"MIME epilogue\r\n"
        req = request(
            "PUT", "/statements", {"statementId": body["id"]}, raw_body_hex=raw.hex()
        )
        req["headers"]["Content-Type"] = "multipart/mixed; boundary=" + (
            '"' + boundary + '"' if variation == "quoted-boundary" else boundary
        )
        get = request(
            "GET",
            "/statements",
            {"statementId": body["id"], "attachments": "true"},
            await_retrieval_ms=5000,
        )
        cases.append(
            case(
                name,
                [
                    "xapi-lrs-4.1.3-multipart-accept-valid",
                    "xapi-lrs-4.1.6.1-statement-get-attachments-true",
                ],
                steps=[
                    step("store", req, expect(204)),
                    read_statement(
                        body["id"], {"/attachments": {"equals": body["attachments"]}}
                    ),
                    step(
                        "retrieve",
                        get,
                        expect(
                            200,
                            checks=[
                                dict(kind="multipart", expected_data_hex=[data.hex()])
                            ],
                        ),
                    ),
                ],
            )
        )
    return cases


def response_vectors():
    return format_cases() + timestamp_cases() + mime_cases()
