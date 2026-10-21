use super::{parse_multipart_headers, parse_multipart_parts};

#[test]
fn preserves_binary_boundary_prefixes_and_accepts_padding() {
    let content =
        b"data\n--edge\r\nembedded\n--edge--\r\n\r\n--edge-prefix\r\n--edge--suffix\0\xff";
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
fn accepts_lf_delimiters_when_no_complete_crlf_delimiter_exists() {
    let body =
        b"--edge\nContent-Type: text/plain\n\nvalue\r\n--edge-prefix\n--edge--suffix\n--edge--\n";
    let parts = parse_multipart_parts(body, "edge").expect("LF-delimited body");
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].body, b"value\r\n--edge-prefix\n--edge--suffix");
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
