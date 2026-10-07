"""Distinguishing witnesses for the 2026-10-06 evaluation; source-derived oracles."""

from __future__ import annotations

import copy
import hashlib

from generate_lrs_projection_cases import IRI, case, dumps, expect, read_statement, request, step, uuid_for
from lrs_projection_behavior import A, DOCS, S


def evaluation_repair_cases():
    cases = []
    for family, path, scope, key, _ in DOCS:
        for method in ["PUT", "POST", "DELETE"]:
            name = family + "-captured-stale-etag-" + method.lower()
            query = dict(scope, **{key: name})
            update = request("PUT", path, query, {"revision": 2})
            update["headers"]["If-Match"] = "{{old_tag}}"
            stale = request(method, path, query, {"revision": 3} if method != "DELETE" else None)
            stale["headers"]["If-Match"] = "{{old_tag}}"
            cases.append(case(name, ["xapi-lrs-4.1.4-if-match", "xapi-lrs-4.1.4-precondition-failed"], steps=[
                step("create", request("PUT", path, query, {"revision": 1}), expect(204)),
                step("fetch-real-tag", request("GET", path, query), expect(200, capture={"old_tag": {"header": "ETag"}})),
                step("update-with-real-tag", update, expect(204)),
                step("reject-previously-valid-tag", stale, expect(412)),
                step("preserve-current-document", request("GET", path, query), expect(200, {"": {"equals": {"revision": 2}}})),
            ]))
        name = family + "-single-delete-preserves-sibling"
        selected = dict(scope, **{key: name + "-selected"})
        sibling = dict(scope, **{key: name + "-sibling"})
        delete = request("DELETE", path, selected)
        delete["headers"]["If-Match"] = "*"
        cases.append(case(name, ["xapi-lrs-4.1.6-support-resources"], steps=[
            step("create-selected", request("PUT", path, selected, {"selected": True}), expect(204)),
            step("create-sibling", request("PUT", path, sibling, {"sibling": True}), expect(204)),
            step("delete-only-selected", delete, expect(204)),
            step("selected-gone", request("GET", path, selected), expect(404)),
            step("sibling-unchanged", request("GET", path, sibling), expect(200, {"": {"equals": {"sibling": True}}})),
        ]))
        name = family + "-post-creates-absent-document"
        query = dict(scope, **{key: name})
        cases.append(case(name, ["xapi-lrs-4.1.6-support-resources"], steps=[
            step("initially-absent", request("GET", path, query), expect(404)),
            step("post-create", request("POST", path, query, {"created": True}), expect(204)),
            step("read-created", request("GET", path, query), expect(200, {"": {"equals": {"created": True}}})),
        ]))
        name = family + "-post-respects-sender-media-type"
        query = dict(scope, **{key: name})
        non_json = request("POST", path, query, {"new": True})
        non_json["headers"]["Content-Type"] = "text/plain"
        cases.append(case(name, ["xapi-lrs-4.1.6-support-resources"], steps=[
            step("create-json", request("PUT", path, query, {"original": True}), expect(204)),
            step("reject-non-json-post", non_json, expect(400)),
            step("preserve-json", request("GET", path, query), expect(200, {"": {"equals": {"original": True}}})),
        ]))
        for method in ["POST", "DELETE"]:
            name = family + "-" + method.lower() + "-if-match-absent-document"
            query = dict(scope, **{key: name})
            write = request(method, path, query, {} if method == "POST" else None)
            write["headers"]["If-Match"] = "*"
            cases.append(case(name, ["xapi-lrs-4.1.4-precondition-failed"], steps=[
                step("initially-absent", request("GET", path, query), expect(404)),
                step("failed-precondition", write, expect(412)),
                step("still-absent", request("GET", path, query), expect(404)),
            ]))
        # Keep every required selector. An ignored unknown key must produce a
        # successful operation, rather than a missing-selector rejection.
        for method in ["GET", "HEAD", "PUT", "POST", "DELETE"]:
            name = family + "-unknown-parameter-with-valid-selectors-" + method.lower()
            query = dict(scope, **{key: name})
            invalid = request(method, path, dict(query, unknownParameter="value"), {"after": True} if method in ["PUT", "POST"] else None)
            invalid["headers"]["If-Match"] = "*"
            cases.append(case(name, ["xapi-lrs-4.1.6-support-resources"], steps=[
                step("create", request("PUT", path, query, {"before": True}), expect(204)),
                step("unknown-key", invalid, expect(400)),
                step("unchanged", request("GET", path, query), expect(200, {"": {"equals": {"before": True}}})),
            ]))

    for label, path, query in [("agents", "/agents", {"agent": dumps(A)}), ("activities", "/activities", {"activityId": IRI}), ("about", "/about", {})]:
        for method in ["GET", "HEAD"]:
            cases.append(case(label + "-unknown-parameter-with-valid-selectors-" + method.lower(), ["xapi-lrs-4.1.6-support-resources"], req=request(method, path, dict(query, unknownParameter="value")), exp=expect(400)))

    name = "statement-duplicate-id-batch-leaves-no-state"
    body = dict(copy.deepcopy(S), id=uuid_for(name))
    query = {"statementId": body["id"]}
    cases.append(case(name, ["xapi-lrs-4.1.6.1-statement-post-duplicate-batch-id"], steps=[
        step("initially-absent", request("GET", "/statements", query), expect(404)),
        step("reject-batch", request("POST", "/statements", body=[body, body]), expect(400)),
        step("no-partial-write", request("GET", "/statements", query), expect(404)),
    ]))
    for name, raw in [("statement-rejects-truncated-json", b'{"actor":'), ("statement-rejects-invalid-utf8-json", dumps(dict(S, actor=dict(A, name="INVALID_UTF8"))).encode().replace(b"INVALID_UTF8", b"\xff"))]:
        cases.append(case(name, ["xapi-lrs-4.2.1-statement-type-validation"], req=request("PUT", "/statements", {"statementId": uuid_for(name)}, raw_body_hex=raw.hex(), headers={"Content-Type": "application/json", "X-Experience-API-Version": "2.0.0"}), exp=expect(400)))
    for value in [75, "75"]:
        name = "statement-score-numeric-" + ("number" if isinstance(value, int) else "string")
        body = dict(copy.deepcopy(S), id=uuid_for(name), result={"score": {"raw": value}})
        cases.append(case(name, ["xapi-lrs-4.2.1-statement-type-validation"], req=request("PUT", "/statements", {"statementId": body["id"]}, body), exp=expect(204 if isinstance(value, int) else 400)))

    for corrupt in [False, True]:
        name = "attachment-data-sha2-" + ("mismatch" if corrupt else "match")
        data = b"actual attachment bytes"
        digest = hashlib.sha256(data).hexdigest()
        body = dict(copy.deepcopy(S), id=uuid_for(name), attachments=[{"usageType": IRI, "display": {"en": "sha2 control"}, "contentType": "text/plain", "length": len(data), "sha2": digest}])
        boundary = "lrs-actual-data-sha2"
        raw = ("\r\n".join(["--" + boundary, "Content-Type: application/json", "", dumps(body), "--" + boundary, "Content-Type: text/plain", "Content-Transfer-Encoding: binary", "X-Experience-API-Hash: " + digest, ""]) + "\r\n").encode() + (b"X" + data[1:] if corrupt else data) + ("\r\n--" + boundary + "--\r\n").encode()
        cases.append(case(name, ["xapi-lrs-4.1.3-attachment-sha2-match"], req=request("PUT", "/statements", {"statementId": body["id"]}, raw_body_hex=raw.hex(), headers={"Content-Type": "multipart/mixed; boundary=" + boundary, "X-Experience-API-Version": "2.0.0"}), exp=expect(400 if corrupt else 204)))

    name = "statement-related-agents-excludes-unrelated"
    bodies = [dict(copy.deepcopy(S), id=uuid_for(name + str(i)), actor=dict(A, mbox="mailto:" + name + str(i) + "@example.org")) for i in range(2)]
    steps = [step("store-" + str(i), request("PUT", "/statements", {"statementId": b["id"]}, b), expect(204)) for i, b in enumerate(bodies)]
    steps += [dict(read_statement(b["id"]), id="visible-" + str(i)) for i, b in enumerate(bodies)]
    steps.append(step("filter-excludes-unrelated", request("GET", "/statements", {"agent": dumps(bodies[0]["actor"]), "related_agents": "true"}), expect(200, {"/statements": {"contains": {"id": bodies[0]["id"]}}}, checks=[{"kind": "json-shape", "pointer": "/statements", "shape": {"type": "array", "min_items": 1, "max_items": 1}}])))
    cases.append(case(name, ["xapi-lrs-4.1.6.1-statement-query-filters"], steps=steps))

    name = "statement-list-format-ids-removes-actor-name"
    body = dict(copy.deepcopy(S), id=uuid_for(name), actor=dict(A, name="must not survive ids"), verb={"id": IRI + "/" + name})
    cases.append(case(name, ["xapi-lrs-4.1.6.1-statement-format-ids"], steps=[
        step("store", request("PUT", "/statements", {"statementId": body["id"]}, body), expect(204)), read_statement(body["id"]),
        step("ids-list", request("GET", "/statements", {"verb": body["verb"]["id"], "format": "ids"}), expect(200, {"/statements": {"contains": {"id": body["id"]}}}, checks=[{"kind": "json-shape", "pointer": "/statements/0/actor", "shape": {"type": "object", "required": ["mbox"], "properties": {"mbox": {"type": "string"}, "objectType": {"enum": ["Agent"]}}, "additional_properties": False}}])),
    ]))
    for fmt in ["exact", "ids", "canonical"]:
        name = "statement-voided-id-allows-format-" + fmt
        target = dict(copy.deepcopy(S), id=uuid_for(name))
        void = dict(copy.deepcopy(S), id=uuid_for(name + "-void"), verb={"id": "http://adlnet.gov/expapi/verbs/voided"}, object={"objectType": "StatementRef", "id": target["id"]})
        cases.append(case(name, ["xapi-lrs-4.1.6.1-statement-query-filters"], steps=[
            step("store", request("PUT", "/statements", {"statementId": target["id"]}, target), expect(204)), read_statement(target["id"]),
            step("void", request("PUT", "/statements", {"statementId": void["id"]}, void), expect(204)),
            step("fetch-voided-with-allowed-parameters", request("GET", "/statements", {"voidedStatementId": target["id"], "format": fmt, "attachments": "false"}, await_retrieval_ms=5000), expect(200, {"/id": {"equals": target["id"]}})),
        ]))
    name = "state-registration-invalid-with-valid-selectors"
    family, path, scope, key, _ = DOCS[0]
    cases.append(case(name, ["xapi-lrs-4.1.6-support-resources"], req=request("PUT", path, dict(scope, stateId=name, registration="not-a-uuid"), {}), exp=expect(400)))
    return cases
