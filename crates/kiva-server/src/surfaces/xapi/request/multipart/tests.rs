use super::{parse_multipart_headers, parse_multipart_parts};

#[test]
fn verifies_full_sha2_digests_and_rejects_tampering() {
    // FIPS SHA-2 known answers for the bytes "abc" (not truncated digests).
    for expected in [
        "23097d223405d8228642a477bda255b32aadbce4bda0b3f7e36c9da7",
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7",
        "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
    ] {
        assert!(super::attachment_hash_matches(b"abc", expected));
        assert!(super::attachment_hash_matches(
            b"abc",
            &expected.to_uppercase()
        ));
        assert!(!super::attachment_hash_matches(b"abd", expected));
        assert!(!super::attachment_hash_matches(b"abc", &expected[2..]));
        assert!(!super::attachment_hash_matches(
            b"abc",
            &"z".repeat(expected.len())
        ));
    }
}

#[test]
fn preserves_binary_non_delimiters_and_accepts_padding() {
    let content =
        b"data\n--edge\r\nembedded\n--edge--\r\n\r\n--other-prefix\r\n--other--suffix\0\xff";
    let mut body =
        b"preamble\r\n--edge \t\r\nContent-Type: application/octet-stream\r\n\r\n".to_vec();
    body.extend_from_slice(content);
    body.extend_from_slice(b"\r\n--edge-- \t\r\nepilogue");
    let parts = parse_multipart_parts(&body, "edge").expect("valid multipart body");
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].body, content);
    assert!(
        parse_multipart_parts(
            b"--edge\r\nContent-Type: text/plain\r\n\r\nx\r\n--edge--suffix",
            "edge"
        )
        .is_err()
    );
}

#[test]
fn rejects_lf_delimiters_and_chosen_boundary_prefix_collisions() {
    let body =
        b"--edge\nContent-Type: text/plain\n\nvalue\r\n--edge-prefix\n--edge--suffix\n--edge--\n";
    assert!(parse_multipart_parts(body, "edge").is_err());
    assert!(
        parse_multipart_parts(
            b"--edge\r\nContent-Type: text/plain\r\n\r\nx\r\n--edge-prefix\r\n--edge--\r\n",
            "edge"
        )
        .is_err()
    );
}

#[test]
fn unfolds_header_continuations_without_accepting_orphans() {
    let headers = parse_multipart_headers(
        b"Content-Type: application/json;\r\n charset=utf-8\r\nX-Test:\r\n\tvalue",
    )
    .expect("valid folded headers");
    assert_eq!(headers["content-type"], "application/json; charset=utf-8");
    assert_eq!(headers["x-test"], "value");
    assert!(parse_multipart_headers(b" orphan").is_err());
    assert!(parse_multipart_headers(b"missing-colon").is_err());
}
