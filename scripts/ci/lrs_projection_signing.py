"""Synthetic RSA/JWS vectors; the checked-in key is public fixture data."""

from __future__ import annotations

import base64
import copy
import hashlib
import subprocess

from generate_lrs_projection_cases import (
    ROOT,
    IRI,
    case,
    dumps,
    expect,
    request,
    uuid_for,
)
from lrs_projection_behavior import A, G, S


def b64(value):
    return base64.urlsafe_b64encode(value).rstrip(b"=").decode("ascii")


def signing_cases():
    cases = []
    fixtures = ROOT / "tests/conformance/fixtures/signing"
    certificate = base64.b64encode(
        (fixtures / "synthetic-certificate.der").read_bytes()
    ).decode("ascii")
    requirements = [
        "xapi-lrs-4.2.6-signed-statement-attachment",
        "xapi-lrs-4.2.6-jws-compact",
        "xapi-lrs-4.2.6-jws-payload",
        "xapi-lrs-4.2.6-jws-algorithm",
        "xapi-lrs-4.2.6-verify-signature",
    ]

    def emit(
        name,
        algorithm="RS256",
        cert=False,
        mutate_payload=None,
        mutate_jws=None,
        content_type="application/octet-stream",
        status=204,
        payload_bytes=None,
        header_override=None,
    ):
        received = dict(copy.deepcopy(S), id=uuid_for(name))
        if name.endswith("group-order"):
            received["actor"] = dict(
                G, member=[A, dict(A, mbox="mailto:second@example.org")]
            )
        if name.endswith("duration"):
            received["result"] = {"duration": "PT1.231S"}
        payload = copy.deepcopy(received)
        if mutate_payload:
            mutate_payload(payload)
        header = {"alg": algorithm}
        if cert:
            header["x5c"] = [certificate]
        if header_override:
            header.update(header_override)
        encoded = (
            b64(dumps(header).encode())
            + "."
            + b64(
                payload_bytes if payload_bytes is not None else dumps(payload).encode()
            )
        )
        digest = {"RS256": "sha256", "RS384": "sha384", "RS512": "sha512"}.get(
            algorithm, "sha256"
        )
        signed = subprocess.run(
            [
                "openssl",
                "dgst",
                "-" + digest,
                "-sign",
                str(fixtures / "synthetic-key.pem"),
            ],
            input=encoded.encode(),
            capture_output=True,
            check=True,
        ).stdout
        jws = encoded + "." + b64(signed)
        if mutate_jws:
            jws = mutate_jws(jws)
        attachment_data = jws.encode("utf-8")
        digest = hashlib.sha256(attachment_data).hexdigest()
        received["attachments"] = [
            {
                "usageType": "http://adlnet.gov/expapi/attachments/signature",
                "display": {"en": "synthetic signature"},
                "contentType": content_type,
                "length": len(attachment_data),
                "sha2": digest,
            }
        ]
        boundary = "lrs-conformance-signing"
        raw = [
            "--" + boundary,
            "Content-Type: application/json",
            "",
            dumps(received),
            "--" + boundary,
            "Content-Type: " + content_type,
            "Content-Transfer-Encoding: binary",
            "X-Experience-API-Hash: " + digest,
            "",
            jws,
            "--" + boundary + "--",
            "",
        ]
        req = request(
            "PUT",
            "/statements",
            {"statementId": received["id"]},
            raw_body_hex="\r\n".join(raw).encode().hex(),
        )
        req["headers"]["Content-Type"] = "multipart/mixed; boundary=" + boundary
        cases.append(
            case(
                name,
                requirements
                if status == 204
                else [
                    "xapi-lrs-4.2.6-reject-malformed-signature",
                    "xapi-lrs-4.2.6-verify-signature",
                ],
                req=req,
                exp=expect(status),
            )
        )

    for algorithm in ["RS256", "RS384", "RS512"]:
        for cert in [False, True]:
            emit(
                "signature-" + algorithm + ("-x5c" if cert else "-without-certificate"),
                algorithm,
                cert,
            )
    emit(
        "signature-tampered-x5c",
        cert=True,
        mutate_jws=lambda jws: jws[:-8] + ("A" if jws[-8] != "A" else "B") + jws[-7:],
        status=400,
    )
    emit(
        "signature-invalid-certificate",
        header_override={"x5c": ["invalid-certificate"]},
        status=400,
    )
    for algorithm in ["HS256", "ES256", "none", "rs256", "unknown"]:
        emit("signature-disallowed-" + algorithm, algorithm, status=400)
    for name, transform in [
        ("compact-segments", lambda value: "not.compact"),
        ("compact-extra-segment", lambda value: value + ".extra"),
        ("json-serialization", lambda value: '{"payload":"invalid"}'),
        ("invalid-base64", lambda value: "!." + value.split(".", 1)[1]),
        (
            "empty-payload",
            lambda value: value.split(".")[0] + ".." + value.split(".")[2],
        ),
    ]:
        emit("signature-" + name, mutate_jws=transform, status=400)
    for index, payload in enumerate(
        [b"{", b"null", b"[]", b'"scalar"', b"\xff", b"{}"]
    ):
        emit(
            "signature-invalid-payload-" + str(index), payload_bytes=payload, status=400
        )
    for field in ["actor", "verb", "object"]:
        emit(
            "signature-incomplete-payload-" + field,
            mutate_payload=lambda payload, field=field: payload.pop(field),
            status=400,
        )
    emit(
        "signature-wrong-actor",
        mutate_payload=lambda payload: payload.update(
            actor=dict(A, mbox="mailto:wrong@example.org")
        ),
        status=400,
    )
    emit(
        "signature-wrong-verb",
        mutate_payload=lambda payload: payload.update(verb={"id": IRI + "/different"}),
        status=400,
    )
    emit(
        "signature-wrong-object",
        mutate_payload=lambda payload: payload.update(
            object={"id": IRI + "/different"}
        ),
        status=400,
    )
    emit("signature-wrong-media-type", content_type="text/plain", status=400)
    excluded = {
        "attachments": lambda payload: payload.update(
            attachments=[
                {
                    "usageType": IRI + "/opaque",
                    "display": {"en": "opaque"},
                    "contentType": "text/plain",
                    "length": 0,
                    "sha2": hashlib.sha256(b"").hexdigest(),
                    "fileUrl": IRI + "/opaque",
                }
            ]
        ),
        "id": lambda payload: payload.update(id=uuid_for("other-payload-id")),
        "authority": lambda payload: payload.update(
            authority=dict(A, mbox="mailto:other-authority@example.org")
        ),
        "stored": lambda payload: payload.update(stored="2000-01-01T00:00:00.123Z"),
        "timestamp": lambda payload: payload.update(
            timestamp="2001-01-01T00:00:00.123Z"
        ),
        "version": lambda payload: payload.update(version="1.0.3"),
        "verb-display": lambda payload: payload["verb"].update(
            display={"en": "another display"}
        ),
        "activity-definition": lambda payload: payload["object"].update(
            definition={"name": {"en": "another definition"}}
        ),
        "group-order": lambda payload: payload["actor"]["member"].reverse(),
        "duration": lambda payload: payload["result"].update(duration="PT1.232S"),
        "case": lambda payload: payload["actor"].update(
            mbox=payload["actor"]["mbox"].upper()
        ),
    }
    for field, transform in excluded.items():
        emit(
            "signature-equivalence-excludes-" + field,
            cert=True,
            mutate_payload=transform,
        )
        if field == "case":
            # M02 excludes only this conflicted equivalence oracle. Header
            # assertions and the other cryptographic cases remain required.
            cases[-1]["expect"]["status_classification"] = "diagnostic"
    return cases
