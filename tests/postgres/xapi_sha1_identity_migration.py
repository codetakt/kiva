#!/usr/bin/env python3
"""Exercise the actual SHA-1 migration against legacy keys and conflicting scopes."""
import json
from pathlib import Path
import subprocess
import sys
import uuid

ROOT = Path(__file__).resolve().parents[2]
MIGRATION = ROOT / "db/migrations/20261030000000_xapi_sha1_identity.sql"
PSQL = ["psql", "-X", "-qAt", "-v", "ON_ERROR_STOP=1",
        "-h", sys.argv[1], "-p", sys.argv[2], "-U", sys.argv[3], "-d", sys.argv[4]]


def sql(text, success=True):
    result = subprocess.run(PSQL, input=text, text=True, capture_output=True)
    if success and result.returncode:
        raise AssertionError(result.stderr)
    return result


def quote(value):
    return "'" + value.replace("'", "''") + "'"


def binary(value):
    return "decode('" + value.encode().hex() + "','hex')"


def part(label, value):
    return f"{label}:{len(value.encode())}:{value}|"


def key(kind, activity, ifi, registration, document_id):
    return (part("kind", kind)
            + (part("activity_id", activity) if kind == "state" else "")
            + (part("agent", ifi) if kind != "activity-profile" else "")
            + (part("registration", registration or "none") if kind == "state" else "")
            + part("document_id", document_id))


def snapshot(table):
    # JSON text is preserved verbatim, including NUL escapes in unrelated fields.
    if table == "xapi_documents":
        fields = "tenant_id,resource_kind,encode(document_key,'hex') AS key,encode(document_id,'hex') AS id,activity_id,agent::text,registration,content_type,content_json::text,encode(content_bytes,'hex') AS bytes,etag,updated_at"
    else:
        fields = "tenant_id,statement_id,encode(actor_ifi,'hex') AS key,statement::text,stored_at,opaque_values::text"
    return json.loads(sql(f"SELECT coalesce(json_agg(t ORDER BY tenant_id,key),'[]') FROM (SELECT {fields} FROM kiva.{table} WHERE tenant_id LIKE 'sha1-migration-%') t;").stdout)


lower = "abcdef0123456789abcdef0123456789abcdef01"
upper = lower.upper()
mixed = "aBcDeF0123456789aBcDeF0123456789aBcDeF01"
activity = "https://example.org/日本語/agent:53:mbox_sha1sum:" + upper
registration = "abcdef01-abcd-4abc-8abc-abcdef012345"
expected_keys = {}
inserts = []


def document(tenant, kind, digest, index, reg=None):
    ifi = "mbox_sha1sum:" + digest
    doc_id = f"{index}\0|agent:53:mbox_sha1sum:{upper}|\\u0000日本語"
    old_key = key(kind, activity, ifi, reg, doc_id)
    new_key = key(kind, activity, "mbox_sha1sum:" + digest.lower(), reg, doc_id)
    expected_keys[(tenant, kind, old_key.encode().hex())] = new_key.encode().hex()
    agent = json.dumps({"mbox_sha1sum": digest, "name": "nul\0\\u0000"})
    inserts.append(f"INSERT INTO kiva.xapi_documents(tenant_id,resource_kind,document_key,document_id,activity_id,agent,registration,content_type,content_json,etag) VALUES ({quote(tenant)},{quote(kind)},{binary(old_key)},{binary(doc_id)},{quote(activity)},{quote(agent)}::json,{quote(reg) if reg else 'NULL'},'application/json','{{\"payload\":\"\\u0000日本語\"}}','\"legacy\"');")


for tenant in ["sha1-migration-one", "sha1-migration-two"]:
    for kind in ["state", "agent-profile"]:
        for index, digest in enumerate([lower, upper, mixed]):
            for reg in ([None, registration] if kind == "state" else [None]):
                document(tenant, kind, digest, index, reg)
    for ifi in ["mbox_sha1sum:" + upper, "mbox:mailto:SHA@example.org", "account:https://example.org:ABC"]:
        statement = json.dumps({"actor": {"name": "\0"}, "opaque": "\\u0000"})
        inserts.append(f"INSERT INTO kiva.xapi_statements(tenant_id,statement_id,actor_ifi,statement) VALUES ({quote(tenant)},'{uuid.uuid4()}',{binary(ifi)},{quote(statement)}::json);")
    # A case-sensitive IFI and key-looking document contents are not normalized.
    other_key = key("agent-profile", activity, "mbox:mailto:SHA@example.org", None, "agent:53:mbox_sha1sum:" + upper + "|")
    inserts.append(f"INSERT INTO kiva.xapi_documents(tenant_id,resource_kind,document_key,document_id,agent,content_type,content_bytes,etag) VALUES ({quote(tenant)},'agent-profile',{binary(other_key)},{binary('agent:53:mbox_sha1sum:' + upper + '|')},'{{\"mbox\":\"mailto:SHA@example.org\"}}','application/octet-stream',decode('0000ff','hex'),'\"bytes\"');")

# A preexisting split address must fail before modifying either table.
document("sha1-migration-collision", "state", lower, "collision")
document("sha1-migration-collision", "state", upper, "collision")
sql("\n".join(inserts))
before_failure = {table: snapshot(table) for table in ["xapi_documents", "xapi_statements"]}
failure = sql(MIGRATION.read_text(), success=False)
assert failure.returncode != 0 and "SHA-1 Agent identity collision" in failure.stderr, failure.stderr
assert before_failure == {table: snapshot(table) for table in before_failure}, "failed preflight changed data"
sql("DELETE FROM kiva.xapi_documents WHERE tenant_id='sha1-migration-collision';")
before = {table: snapshot(table) for table in before_failure}
sql(MIGRATION.read_text())
after = {table: snapshot(table) for table in before_failure}
for row in before["xapi_documents"]:
    row["key"] = expected_keys.get((row["tenant_id"], row["resource_kind"], row["key"]), row["key"])
for row in before["xapi_statements"]:
    value = bytes.fromhex(row["key"]).decode()
    if value.startswith("mbox_sha1sum:"):
        row["key"] = value.lower().encode().hex()
order = lambda rows: sorted(rows, key=lambda row: (row["tenant_id"], row["key"]))
assert all(order(before[table]) == order(after[table]) for table in before), "migration changed more than identity keys"
sql(MIGRATION.read_text())
assert after == {table: snapshot(table) for table in before}, "migration is not idempotent"
sql("DELETE FROM kiva.xapi_documents WHERE tenant_id LIKE 'sha1-migration-%'; DELETE FROM kiva.xapi_statements WHERE tenant_id LIKE 'sha1-migration-%';")
print(json.dumps({"documents": len(after['xapi_documents']), "statements": len(after['xapi_statements']),
                  "collision_preserves_all_data": True, "only_keys_changed": True, "idempotent": True}))
