/// Returns whether a comma-separated `ETag` precondition list matches an `ETag`.
#[must_use]
pub fn etag_list_matches(value: &str, etag: &str) -> bool {
    let value = value.as_bytes();
    let etag = etag.as_bytes();
    let mut cursor = 0;

    while cursor <= value.len() {
        let segment_start = cursor;
        while cursor < value.len() && value[cursor] != b',' {
            cursor += 1;
        }

        let mut start = segment_start;
        let mut end = cursor;
        while start < end && is_http_optional_whitespace(value[start]) {
            start += 1;
        }
        while end > start && is_http_optional_whitespace(value[end - 1]) {
            end -= 1;
        }

        if candidate_is_star(value, start, end) || candidate_equals(value, start, end, etag) {
            return true;
        }
        if cursor == value.len() {
            return false;
        }
        cursor += 1;
    }

    false
}

const fn is_http_optional_whitespace(byte: u8) -> bool {
    byte == b' ' || byte == b'\t'
}

fn candidate_is_star(value: &[u8], start: usize, end: usize) -> bool {
    end > start && end - start == 1 && value[start] == b'*'
}

fn candidate_equals(value: &[u8], start: usize, end: usize, expected: &[u8]) -> bool {
    if end - start != expected.len() {
        return false;
    }

    let mut offset = 0;
    while offset < expected.len() {
        if value[start + offset] != expected[offset] {
            return false;
        }
        offset += 1;
    }
    true
}
