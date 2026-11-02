use axum::http::StatusCode;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde_json::{Value, json};

use crate::{core::signature_policy::SignaturePolicy, surfaces::xapi::model::AttachmentRecord};

use super::{
    base64_url_decode, compact_jws_segments, validate_signed_statement_attachment,
    verify_certificate_signature,
};

#[test]
fn jws_base64url_requires_complete_quanta_and_zero_unused_bits() {
    for (encoded, decoded) in [("Zg", "f"), ("Zm8", "fo"), ("Zm9v", "foo")] {
        assert_eq!(
            base64_url_decode(encoded).expect("canonical base64url"),
            decoded.as_bytes()
        );
    }
    for invalid in ["A", "Zm9vA", "Zh", "Zm9", "Zg==", "Zm+v", "Zm/v", "Z g"] {
        assert!(base64_url_decode(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn signed_payload_id_is_validated_before_exclusion() {
    let received = json!({
        "id": "00000000-0000-4000-8000-000000000001",
        "actor": {"mbox": "mailto:signer@example.org"},
        "verb": {"id": "https://example.org/experienced"},
        "object": {"id": "https://example.org/activity"}
    });
    let mut signed = received.clone();
    signed.as_object_mut().expect("object").remove("id");
    assert!(validate_signed_statement_attachment(&received, &sign(&signed), &policy()).is_ok());
    // Valid IDs remain excluded from logical signature equivalence.
    signed["id"] = json!("00000000-0000-4000-8000-000000000002");
    assert!(validate_signed_statement_attachment(&received, &sign(&signed), &policy()).is_ok());
    for id in [
        json!("not-a-uuid"),
        json!("00000000000040008000000000000002"),
        json!(42),
        Value::Null,
    ] {
        signed["id"] = id;
        let error = validate_signed_statement_attachment(&received, &sign(&signed), &policy())
            .expect_err("malformed signed ID must be rejected");
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
    }
}

fn sign(payload: &Value) -> AttachmentRecord {
    let key = EncodingKey::from_rsa_pem(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/fixtures/signing/synthetic-key.pem"
    )))
    .expect("fixture key");
    let mut header = Header::new(Algorithm::RS256);
    header.x5c = Some(vec![base64_certificate(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/fixtures/signing/synthetic-certificate.der"
    )))]);
    let jws = jsonwebtoken::encode(&header, payload, &key).expect("signed fixture");
    // Verify the certificate/signature even for the malformed-ID negative controls.
    let segments = compact_jws_segments(&jws).expect("compact JWS");
    verify_certificate_signature(
        &serde_json::to_value(header).expect("header"),
        &segments,
        &policy(),
    )
    .expect("valid signature");
    AttachmentRecord {
        content_type: "application/octet-stream".to_owned(),
        body: jws.into_bytes(),
    }
}

fn base64_certificate(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::new();
    for chunk in bytes.chunks(3) {
        let bits = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for (index, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            encoded.push(if index > chunk.len() {
                '='
            } else {
                char::from(ALPHABET[((bits >> shift) & 63) as usize])
            });
        }
    }
    encoded
}

fn policy() -> SignaturePolicy {
    SignaturePolicy::from_json(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/controls/kiva_certificate_policy.json"
    )))
    .expect("explicit local fixture trust and status")
}
