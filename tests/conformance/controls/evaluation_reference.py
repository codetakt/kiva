#!/usr/bin/env python3
"""Bounded HTTP reference for the evaluation repair controls.

This model does not read cases, expectations, Kiva code, or run reports.
Its rules come from PDF 4.1.4 (conditional document writes), 4.1.6.2/.5/.6
(document scope, POST merge and single DELETE), 4.1.6.1 (batch atomicity,
filters and formats), 4.2.1/4.2.4.2 (JSON and primitive validation),
4.1.3.4 (attachment transport), and 5.2.7.5 -> ISO 8601 / RFC3339 4.3 with RFC9557 2.1-2.3.
The disputed -00:00 boundary permits rejection or acceptance with UTC output.
It is deliberately incomplete outside those finite witness conditions.
No reference result is a target LRS conformance result.
"""

import argparse
import copy
import datetime
import email.policy
import email.utils
import threading
import hashlib
import json
import re
import uuid
from email.parser import BytesParser
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qsl, urlsplit

DOCS = {"/activities/state": ("stateId", {"activityId", "agent", "registration", "stateId", "since"}), "/agents/profile": ("profileId", {"agent", "profileId", "since"}), "/activities/profile": ("profileId", {"activityId", "profileId", "since"})}
KNOWN = {**{p: allowed for p, (_, allowed) in DOCS.items()}, "/agents": {"agent"}, "/activities": {"activityId"}, "/about": set(), "/statements": {"statementId", "voidedStatementId", "agent", "verb", "activity", "registration", "related_agents", "related_activities", "format", "attachments", "limit", "since", "until", "ascending"}}


def serve(port, mutation, trace):
    docs, statements, voided = {}, {}, set()
    fixture = {"clock": None, "hide_new": False, "hidden": set()}

    def now():
        return fixture["clock"] or datetime.datetime.now(datetime.timezone.utc).isoformat().replace("+00:00", "Z")

    def instant(value):
        return datetime.datetime.fromisoformat(value.replace("Z", "+00:00"))

    def modified(value):
        return email.utils.format_datetime(instant(value), usegmt=True)

    def same_ifi(left, right):
        return any(key in left and key in right and left[key] == right[key] for key in ["mbox", "mbox_sha1sum", "openid", "account"])

    def has_agent(value, agent):
        if isinstance(value, dict):
            return same_ifi(value, agent) or any(has_agent(v, agent) for v in value.values())
        return isinstance(value, list) and any(has_agent(v, agent) for v in value)

    def ids(value):
        if isinstance(value, list):
            return [ids(v) for v in value]
        if not isinstance(value, dict):
            return value
        result = {k: ids(v) for k, v in value.items()}
        if any(k in value for k in ["mbox", "mbox_sha1sum", "openid", "account"]) or value.get("objectType") == "Group":
            result.pop("name", None)
        elif "id" in value and "actor" not in value:
            result.pop("definition", None)
            result.pop("display", None)
        return result

    def canonical(value, header):
        def select(mapping):
            if not mapping:
                return mapping
            ranges = []
            for position, item in enumerate((header or "*").split(",")):
                parts = item.strip().split(";")
                q = next((float(p.strip()[2:]) for p in parts[1:] if p.strip().startswith("q=")), 1.0)
                ranges.append((-q, position, parts[0].lower()))
            for minus_q, _, language in sorted(ranges):
                if minus_q == 0:
                    continue
                keys = [k for k in mapping if language == "*" or k.lower() == language or k.lower().startswith(language + "-")]
                if keys:
                    return {keys[0]: mapping[keys[0]]}
            key = next(iter(mapping))
            return {key: mapping[key]}
        if isinstance(value, list):
            return [canonical(v, header) for v in value]
        if not isinstance(value, dict):
            return value
        result = {k: canonical(v, header) for k, v in value.items()}
        definition = result.get("definition")
        if isinstance(definition, dict):
            for key in ["name", "description"]:
                if key in definition:
                    definition[key] = select(definition[key])
            for key in ["choices", "scale", "source", "target", "steps"]:
                for component in definition.get(key, []):
                    if "description" in component:
                        component["description"] = select(component["description"])
        return result

    def invalid(value):
        if not isinstance(value, dict):
            return True
        # Structural invalidity is independent of optional interaction checks.
        if value.get("result", {}).get("score", {}).get("raw") is not None:
            raw = value["result"]["score"]["raw"]
            if not isinstance(raw, (int, float)) or isinstance(raw, bool):
                return mutation != "accept-numeric-string"
        for context in [value.get("context", {}), value.get("object", {}).get("context", {})]:
            for field in ["contextAgents", "contextGroups"]:
                for item in context.get(field, []):
                    if "relevantTypes" in item and item["relevantTypes"] == [] and mutation != "accept-empty-relevant-types":
                        return True
        for obj in [value.get("object", {}), value.get("object", {}).get("object", {})]:
            definition = obj.get("definition", {})
            typ = definition.get("interactionType")
            allowed = {"choice": {"choices"}, "sequencing": {"choices"}, "likert": {"scale"}, "matching": {"source", "target"}, "performance": {"steps"}}.get(typ, set())
            if any(key in definition and key not in allowed for key in ["choices", "scale", "source", "target", "steps"]):
                return True
        return False

    class Handler(BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"

        def log_message(self, *_):
            pass

        def handle_request(self):
            url = urlsplit(self.path)
            query = dict(parse_qsl(url.query, keep_blank_values=True))
            raw = self.rfile.read(int(self.headers.get("Content-Length", 0)))
            result, extra = self.dispatch(self.command, url.path, query, raw)
            status, body = result
            encoded = json.dumps(body, separators=(",", ":")).encode() if body is not None else b""
            self.send_response(status)
            self.send_header("X-Experience-API-Version", "2.0.0")
            watermark = min((statements[k]["stored"] for k in fixture["hidden"]), default=now())
            self.send_header("X-Experience-API-Consistent-Through", watermark)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(encoded)))
            for key, value in extra.items():
                self.send_header(key, value)
            self.end_headers()
            if self.command != "HEAD":
                self.wfile.write(encoded)
            with trace.open("a") as file:
                file.write(json.dumps({"method": self.command, "path": url.path, "query": query, "request_body_hex": raw.hex(), "response_status": status}) + "\n")

        def dispatch(self, method, path, query, raw):
            def reply(status, body=None, headers=None):
                return (status, body), headers or {}
            if path == "/_control" and method == "POST":
                control = json.loads(raw)
                if "clock" in control:
                    fixture["clock"] = control["clock"]
                if "hide_new" in control:
                    fixture["hide_new"] = control["hide_new"]
                if control.get("release"):
                    fixture["hidden"].clear()
                return reply(204)
            version = self.headers.get("X-Experience-API-Version", "2.0.0")
            if version != "2.0.0":
                return reply(400, {"message": "Unsupported request version " + version})
            if path not in KNOWN or set(query) - KNOWN[path]:
                return reply(400)
            if path in DOCS:
                selector, _ = DOCS[path]
                if "registration" in query and not re.fullmatch(r"[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}", query["registration"]):
                    return reply(400)
                scoped = tuple(sorted((k, v) for k, v in query.items() if k != "since"))
                current = docs.get((path, scoped))
                tag = '"' + hashlib.sha256(current[0]).hexdigest() + '"' if current else None
                if method in ["GET", "HEAD"]:
                    if selector not in query:
                        scope = dict(query)
                        return reply(200, [dict(k)[selector] for (p, k) in docs if p == path and all(dict(k).get(a) == b for a, b in scope.items() if a != "since") and ("since" not in scope or instant(docs[(p,k)][2]) > instant(scope["since"]))])
                    if current is None:
                        return reply(404)
                    return reply(200, json.loads(current[0]), {"ETag": tag, "Last-Modified": modified(current[2])})
                match = self.headers.get("If-Match")
                if match and (current is None or match not in ["*", tag]):
                    return reply(412)
                if method == "DELETE":
                    if selector not in query and path == "/activities/state":
                        selected = [(p, k) for p, k in docs if p == path and all(dict(k).get(a) == b for a, b in query.items())]
                        for document_key in selected:
                            docs.pop(document_key)
                        return reply(204)
                    docs.pop((path, scoped), None)
                    return reply(204)
                if method == "POST" and (self.headers.get_content_type() != "application/json" or (current and current[1] != "application/json")):
                    return reply(400)
                if method == "PUT" and current and not match and not self.headers.get("If-None-Match"):
                    return reply(409, {"message": "Check current state and use If-Match with the current ETag."})
                try:
                    document = json.loads(raw)
                except (ValueError, UnicodeError):
                    return reply(400)
                if method == "POST":
                    if not isinstance(document, dict):
                        return reply(400)
                    document = {**(json.loads(current[0]) if current else {}), **document}
                docs[(path, scoped)] = (json.dumps(document, separators=(",", ":")).encode(), self.headers.get_content_type(), now())
                return reply(204)
            if path == "/about":
                return reply(200, {"version": ["2.0.0"]})
            if path == "/agents":
                actor = json.loads(query["agent"])
                person = {"objectType": "Person", **{key: [value] for key, value in actor.items() if key != "objectType"}}
                if mutation in ["person-valid-enrichment", "person-openid-unclosed-literal", "person-homepage-unclosed-literal", "person-mbox-malformed-percent"]:
                    person.setdefault("mbox", []).append("mailto:directory%GG@example.org" if mutation == "person-mbox-malformed-percent" else "mailto:directory@example.org")
                    person.setdefault("openid", []).append("https://[" if mutation == "person-openid-unclosed-literal" else "https://[2001:db8::1]/person")
                    person.setdefault("account", []).append({"homePage": "https://[" if mutation == "person-homepage-unclosed-literal" else "https://[2001:db8::1]/directory", "name": "directory"})
                return reply(200, person)
            if path == "/activities":
                activity = {"objectType": "Activity", "id": query["activityId"]}
                available = {}
                for statement in statements.values():
                    obj = statement.get("object", {})
                    if obj.get("id") == query["activityId"]:
                        for key, value in obj.get("definition", {}).items():
                            if isinstance(value, dict) and isinstance(available.get(key), dict):
                                available[key].update(value)
                            else:
                                available[key] = copy.deepcopy(value)
                if available:
                    activity["definition"] = available
                return reply(200, activity)
            if method in ["GET", "HEAD"]:
                identifier = query.get("statementId", query.get("voidedStatementId"))
                if identifier:
                    value = statements.get(identifier)
                    if value is None or identifier in fixture["hidden"] or ("statementId" in query and identifier in voided):
                        return reply(404)
                    projected = ids(value) if query.get("format") == "ids" else canonical(value, self.headers.get("Accept-Language")) if query.get("format") == "canonical" else value
                    return reply(200, projected, {"Last-Modified": modified(value["stored"])})
                values = [copy.deepcopy(value) for identifier, value in statements.items() if identifier not in voided and identifier not in fixture["hidden"]]
                if "verb" in query:
                    values = [v for v in values if v["verb"]["id"] == query["verb"]]
                if "agent" in query:
                    actor = json.loads(query["agent"])
                    values = [v for v in values if has_agent(v if query.get("related_agents") == "true" else {"actor": v["actor"], "object": v.get("object", {}) if v.get("object", {}).get("objectType") in ["Agent", "Group"] else {}}, actor)]
                if query.get("format") == "ids":
                    values = [ids(value) for value in values]
                elif query.get("format") == "canonical":
                    values = [canonical(value, self.headers.get("Accept-Language")) for value in values]
                if "since" in query:
                    values = [v for v in values if instant(v["stored"]) > instant(query["since"])]
                if "until" in query:
                    values = [v for v in values if instant(v["stored"]) <= instant(query["until"])]
                values.sort(key=lambda v: instant(v["stored"]), reverse=query.get("ascending") != "true")
                return reply(200, {"statements": values, "more": ""})
            content_type = self.headers.get_content_type()
            attachment_parts = None
            json_raw = raw
            if content_type == "multipart/mixed":
                message = BytesParser(policy=email.policy.default).parsebytes(("Content-Type: " + self.headers["Content-Type"] + "\r\nMIME-Version: 1.0\r\n\r\n").encode() + raw)
                parts = list(message.iter_parts())
                if not parts:
                    return reply(400)
                json_raw = parts[0].get_payload(decode=True)
                attachment_parts = {p["X-Experience-API-Hash"]: p.get_payload(decode=True) for p in parts[1:]}
            try:
                value = json.loads(json_raw)
            except (ValueError, UnicodeError):
                return reply(400 if mutation != "accept-invalid-json" else 204)
            batch = value if isinstance(value, list) else [value]
            if any(invalid(v) for v in batch):
                return reply(400)
            identifiers = [v.get("id", query.get("statementId", str(uuid.uuid4()))) for v in batch]
            if len(identifiers) != len(set(identifiers)):
                return reply(400)
            if mutation == "reject-property-order" and isinstance(value, dict) and list(value) != sorted(value):
                return reply(400)
            for item in batch:
                if item.get("version") == "1.1.0" and mutation == "reject-advisory-version":
                    return reply(400)
                for obj in [item, item.get("object", {})]:
                    for attachment in obj.get("attachments", []):
                        if attachment_parts is None and not attachment.get("fileUrl") and mutation != "accept-json-without-file-url":
                            return reply(400)
                        if attachment_parts is not None and not attachment.get("fileUrl"):
                            data = attachment_parts.get(attachment["sha2"])
                            if data is None or (hashlib.sha256(data).hexdigest() != attachment["sha2"] and mutation != "accept-data-sha-mismatch"):
                                return reply(400)
                if item.get("timestamp", "").endswith("-00:00") and mutation == "reject-unknown-local-offset":
                    return reply(400)
            for item, identifier in zip(batch, identifiers):
                if identifier in statements:
                    continue
                stored = copy.deepcopy(item)
                stored.update(id=identifier, authority={"objectType": "Agent", "mbox": "mailto:reference@example.org"}, stored=now(), version="2.0.0")
                timestamp = stored.get("timestamp", stored["stored"])
                stored["timestamp"] = datetime.datetime.fromisoformat(timestamp.replace("Z", "+00:00")).astimezone(datetime.timezone.utc).isoformat().replace("+00:00", "Z")
                if item.get("timestamp", "").endswith("-00:00") and mutation == "unknown-local-offset-not-normalized":
                    stored["timestamp"] = item["timestamp"]
                statements[identifier] = stored
                if fixture["hide_new"]:
                    fixture["hidden"].add(identifier)
                if item["verb"]["id"] == "http://adlnet.gov/expapi/verbs/voided":
                    voided.add(item["object"]["id"])
            return reply(204 if method == "PUT" else 200, None if method == "PUT" else identifiers)

        do_GET = do_HEAD = do_PUT = do_POST = do_DELETE = handle_request

    ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("port", type=int)
    parser.add_argument("--mutation", default="none")
    parser.add_argument("--trace", required=True, type=Path)
    args = parser.parse_args()
    serve(args.port, args.mutation, args.trace)
