"""Finite source-derived witnesses for the post-R1 coverage gaps.

Conditional clocks, canonical inventories and semantic explanation review stay
in the procedure executor: submitted metadata is not a canonical inventory.
"""

import copy

from generate_lrs_projection_cases import IRI, case, dumps, expect, read_statement, request, set_pointer, step, uuid_for
from lrs_projection_behavior import A, DOCS, S, filter_cases


def followup_cases():
    cases = []
    for method in ["PUT", "POST"]:
        name = "statement-unknown-parameter-with-valid-selectors-" + method.lower()
        body = dict(copy.deepcopy(S), id=uuid_for(name))
        query = {"statementId": body["id"]} if method == "PUT" else {}
        invalid = request(method, "/statements", dict(query, unknownParameter="value"), body)
        cases.append(case(name, ["xapi-lrs-4.1.5-unknown-parameters"], steps=[
            step("valid-write", request("PUT", "/statements", {"statementId": body["id"]}, body), expect(204)),
            step("unknown-key", invalid, expect(400)),
            read_statement(body["id"]),
        ]))

    _, path, scope, key, _ = DOCS[0]
    for method in ["GET", "HEAD", "PUT", "POST", "DELETE"]:
        for invalid in [False, True]:
            if invalid and method == "PUT":
                continue  # The R1 PUT witness remains in place.
            name = "state-registration-" + ("invalid" if invalid else "valid") + "-with-valid-selectors-" + method.lower()
            good = dict(scope, stateId=name, registration=uuid_for(name))
            selected = dict(good, registration="not-a-uuid") if invalid else good
            probe = request(method, path, selected, {"after": True} if method in ["PUT", "POST"] else None)
            probe["headers"]["If-Match"] = "*"
            cases.append(case(name, ["xapi-lrs-4.1.6-support-resources"], steps=[
                step("valid-registration-create", request("PUT", path, good, {"before": True}), expect(204)),
                step("registration-probe", probe, expect(400 if invalid else 200 if method in ["GET", "HEAD"] else 204)),
                *([step("unchanged", request("GET", path, good), expect(200, {"": {"equals": {"before": True}}}))] if invalid else []),
            ]))

    # All existing Agent placements and IFIs receive a disjoint non-match.
    for original in filter_cases():
        if not original["id"].startswith("agent-filter-"):
            continue
        name = original["id"] + "-excludes-unrelated"
        first = original["steps"][0]["request"]
        body = dict(copy.deepcopy(first["body"]), id=uuid_for(name), verb={"id": IRI + "/" + name})
        unrelated = dict(copy.deepcopy(S), id=uuid_for(name + "-unrelated"), verb=copy.deepcopy(body["verb"]), actor={"mbox": "mailto:" + name + "-unrelated@example.org"})
        query = copy.deepcopy(original["steps"][-1]["request"]["query"])
        query["verb"] = body["verb"]["id"]
        query["related_agents"] = "true"
        read_match, read_nonmatch = read_statement(body["id"]), read_statement(unrelated["id"])
        read_match["id"], read_nonmatch["id"] = "read-match", "read-nonmatch"
        cases.append(case(name, ["xapi-lrs-4.1.6.1-statement-query-related-agents"], steps=[
            step("store-match", request("PUT", "/statements", {"statementId": body["id"]}, body), expect(204)),
            step("store-nonmatch", request("PUT", "/statements", {"statementId": unrelated["id"]}, unrelated), expect(204)),
            read_match,
            read_nonmatch,
            step("exact-filter-membership", request("GET", "/statements", query, await_retrieval_ms=1000), expect(200, {"/statements": {"contains": {"id": body["id"]}}}, checks=[{"kind": "json-shape", "pointer": "/statements", "shape": {"type": "array", "min_items": 1, "max_items": 1}}])),
        ]))
        if original["id"].startswith("agent-filter-group-member-"):
            member_case = copy.deepcopy(cases[-1])
            member_case["id"] = original["id"] + "-member-excludes-unrelated"
            replacements = {body["id"]: uuid_for(member_case["id"]), unrelated["id"]: uuid_for(member_case["id"] + "-unrelated"), body["verb"]["id"]: IRI + "/" + member_case["id"]}
            def fresh(value):
                if isinstance(value, dict):
                    return {key: fresh(item) for key, item in value.items()}
                if isinstance(value, list):
                    return [fresh(item) for item in value]
                return replacements.get(value, value) if isinstance(value, str) else value
            member_case = fresh(member_case)
            member_case["steps"][-1]["request"]["query"]["agent"] = original["steps"][2]["request"]["query"]["agent"]
            cases.append(member_case)

    agent = {"objectType": "Agent", "mbox": "mailto:ids-followup@example.org", "name": "display must disappear"}
    activity = {"id": IRI + "/ids-followup", "definition": {"name": {"en": "display"}}}
    group = {"objectType": "Group", "member": [agent], "name": "group display"}
    actor_shape = {"type": "object", "required": ["mbox"], "properties": {"objectType": {"enum": ["Agent"]}, "mbox": {"enum": [agent["mbox"]]}}, "additional_properties": False}
    group_shape = {"type": "object", "required": ["objectType", "member"], "properties": {"objectType": {"enum": ["Group"]}, "member": {"type": "array", "min_items": 1, "max_items": 1, "items": actor_shape}}, "additional_properties": False}
    activity_shape = {"type": "object", "required": ["id"], "properties": {"id": {"type": "string"}, "objectType": {"enum": ["Activity"]}}, "additional_properties": False}
    placements = [
        ("/object", agent, actor_shape),
        ("/object", group, group_shape),
        ("/actor", group, group_shape),
        ("/context/instructor", agent, actor_shape),
        ("/context/team", group, group_shape),
        ("/context/contextAgents/0/agent", agent, actor_shape),
        ("/context/contextGroups/0/group", group, group_shape),
        ("/object", activity, activity_shape),
        ("/context/contextActivities/category/0", activity, activity_shape),
        ("/verb", {"id": IRI + "/verb", "display": {"en": "display"}}, {"type": "object", "required": ["id"], "properties": {"id": {"type": "string"}}, "additional_properties": False}),
    ]
    for index, (pointer, value, shape) in enumerate(placements):
        for nested in [False, True]:
            name = "statement-list-format-ids-recursive-" + str(index) + ("-substatement" if nested else "-statement")
            body = copy.deepcopy(S)
            if "contextAgents" in pointer:
                body = set_pointer(body, "/context/contextAgents", [{"objectType": "contextAgent", "agent": agent}])
            elif "contextGroups" in pointer:
                body = set_pointer(body, "/context/contextGroups", [{"objectType": "contextGroup", "group": group}])
            body = set_pointer(body, pointer, value)
            if nested:
                body = dict(copy.deepcopy(S), object={"objectType": "SubStatement", **body})
            body.update(id=uuid_for(name), verb={"id": IRI + "/" + name, "display": {"en": "display"}})
            cases.append(case(name, ["xapi-lrs-4.1.6.1-statement-format-ids"], steps=[
                step("store", request("PUT", "/statements", {"statementId": body["id"]}, body), expect(204)),
                read_statement(body["id"]),
                step("ids-list", request("GET", "/statements", {"verb": body["verb"]["id"], "format": "ids"}), expect(200, {"/statements": {"contains": {"id": body["id"]}}}, checks=[{"kind": "json-shape", "pointer": "/statements/0" + ("/object" if nested else "") + pointer, "shape": shape}])),
            ]))

    person = {"type": "object", "required": ["objectType"], "properties": {
        "objectType": {"enum": ["Person"]}, "name": {"type": "array", "items": {"type": "string"}},
        **{field: {"type": "array", "items": {"type": "string", "format": fmt}} for field, fmt in [("mbox", "mailto-iri"), ("mbox_sha1sum", "sha1"), ("openid", "uri")]},
        "account": {"type": "array", "items": {"type": "object", "required": ["homePage", "name"], "properties": {"homePage": {"type": "string", "format": "irl"}, "name": {"type": "string"}}, "additional_properties": False}},
    }, "additional_properties": False}
    for field, value in [("mbox", "mailto:person-followup@example.org"), ("mbox_sha1sum", "0123456789abcdef0123456789abcdef01234567"), ("openid", "https://example.org/person-followup"), ("account", {"homePage": "https://example.org/account-followup", "name": "learner"})]:
        cases.append(case("person-response-all-identifiers-" + field, ["xapi-lrs-4.1.6.3-person-arrays"], req=request("GET", "/agents", {"agent": dumps({field: value})}), exp=expect(200, {"/" + field: {"contains": value}}, checks=[{"kind": "json-shape", "shape": person}])))
    return cases
