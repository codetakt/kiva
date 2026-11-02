//! Negotiated error representations without changing the original status or headers.

use axum::{
    body::{Body, to_bytes},
    extract::Request,
    http::{HeaderMap, HeaderValue, Method, header},
    middleware::Next,
    response::Response,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Representation {
    Json,
    Text,
    Html,
}

impl Representation {
    const fn media(self) -> &'static str {
        match self {
            Self::Json => "application/json",
            Self::Text => "text/plain",
            Self::Html => "text/html",
        }
    }

    const fn content_type(self) -> &'static str {
        match self {
            Self::Json => "application/json",
            Self::Text => "text/plain; charset=utf-8",
            Self::Html => "text/html; charset=utf-8",
        }
    }
}

pub(super) async fn negotiate(request: Request, next: Next) -> Response {
    let selected = select(request.headers());
    let head = request.method() == Method::HEAD;
    let response = next.run(request).await;
    if !response.status().is_client_error() && !response.status().is_server_error() {
        return response;
    }
    let (mut parts, body) = response.into_parts();
    parts
        .headers
        .append(header::VARY, HeaderValue::from_static("Accept"));
    if head {
        parts.headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static(selected.content_type()),
        );
        parts.headers.remove(header::CONTENT_LENGTH);
        return Response::from_parts(parts, Body::empty());
    }
    // These are already constructed application error bodies. A second,
    // smaller ceiling would silently discard valid cause explanations.
    let bytes = to_bytes(body, usize::MAX).await.unwrap_or_default();
    let message = serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| {
            if bytes.is_empty() {
                parts
                    .status
                    .canonical_reason()
                    .unwrap_or("Request failed")
                    .to_owned()
            } else {
                String::from_utf8_lossy(&bytes).into_owned()
            }
        });
    let rendered = match selected {
        Representation::Json => {
            if serde_json::from_slice::<serde_json::Value>(&bytes).is_ok() {
                bytes.to_vec()
            } else {
                serde_json::json!({"error": message}).to_string().into_bytes()
            }
        }
        Representation::Text => format!("{message}\n").into_bytes(),
        Representation::Html => format!(
            "<!doctype html><html><head><meta charset=\"utf-8\"><title>Request failed</title></head><body><p>{}</p></body></html>",
            escape_html(&message)
        ).into_bytes(),
    };
    parts.headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(selected.content_type()),
    );
    parts.headers.remove(header::CONTENT_LENGTH);
    Response::from_parts(parts, Body::from(rendered))
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn select(headers: &HeaderMap) -> Representation {
    let ranges = headers
        .get_all(header::ACCEPT)
        .iter()
        .filter_map(|h| h.to_str().ok())
        .flat_map(|h| split_quoted(h, b','))
        .filter_map(parse_range)
        .collect::<Vec<_>>();
    // RFC 7231 permits ignoring an unsatisfiable Accept. Keep the cause-specific
    // xAPI status, with JSON as the documented fallback, rather than replacing
    // e.g. a required 409 or 400 with 406.
    [
        Representation::Json,
        Representation::Text,
        Representation::Html,
    ]
    .into_iter()
    .filter_map(|representation| {
        let quality = ranges
            .iter()
            .filter_map(|(media, q, parameters)| {
                let specificity = if media == representation.media() {
                    2
                } else if media == "*/*" {
                    0
                } else if media
                    .strip_suffix("/*")
                    .is_some_and(|kind| representation.media().starts_with(&format!("{kind}/")))
                {
                    1
                } else {
                    return None;
                };
                Some(((specificity, *parameters), *q))
            })
            .max_by_key(|(specificity, q)| (*specificity, *q))
            .map_or(0, |(_, q)| q);
        (quality > 0).then_some((quality, representation))
    })
    .fold(None, |best: Option<(u16, Representation)>, candidate| {
        if best.is_none_or(|old| candidate.0 > old.0) {
            Some(candidate)
        } else {
            best
        }
    })
    .map_or(Representation::Json, |(_, representation)| representation)
}

fn parse_range(input: &str) -> Option<(String, u16, u8)> {
    let mut pieces = split_quoted(input, b';').into_iter();
    let media = pieces.next()?.trim().to_ascii_lowercase();
    let mut quality = 1000;
    let mut parameters = 0;
    let mut after_quality = false;
    for piece in pieces {
        if after_quality && !piece.contains('=') {
            continue;
        }
        let (name, value) = piece.trim().split_once('=')?;
        if name.trim().eq_ignore_ascii_case("q") {
            if after_quality {
                return None;
            }
            quality = parse_quality(value.trim())?;
            after_quality = true;
        } else if !after_quality {
            if !name.trim().eq_ignore_ascii_case("charset")
                || !value.trim().trim_matches('"').eq_ignore_ascii_case("utf-8")
            {
                return None;
            }
            parameters = 1;
        }
    }
    Some((media, quality, parameters))
}

fn split_quoted(input: &str, delimiter: u8) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    let mut escaped = false;
    for (index, byte) in input.bytes().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if quoted && byte == b'\\' {
            escaped = true;
            continue;
        }
        if byte == b'"' {
            quoted = !quoted;
        }
        if byte == delimiter && !quoted {
            parts.push(&input[start..index]);
            start = index + 1;
        }
    }
    if !quoted && !escaped {
        parts.push(&input[start..]);
    }
    parts
}

fn parse_quality(value: &str) -> Option<u16> {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if fraction.len() > 3 || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    match whole {
        "1" if fraction.bytes().all(|b| b == b'0') => Some(1000),
        "0" => format!("{fraction:0<3}").parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accept_quality_exclusions_and_specificity() {
        for (accept, expected) in [
            ("text/plain", Representation::Text),
            ("text/html", Representation::Html),
            (
                "text/html;q=0.8, application/json;q=0.2",
                Representation::Html,
            ),
            (
                "text/*;q=1, text/html;q=0, application/json;q=0.5",
                Representation::Text,
            ),
            (
                "text/html;charset=utf-8;q=1, text/html;q=0",
                Representation::Html,
            ),
            ("text/html;charset=iso-8859-1", Representation::Json),
            ("*/*;q=0", Representation::Json),
            ("text/html;q=1.001", Representation::Json),
            ("text/plain;q=0.001, text/html;q=0", Representation::Text),
        ] {
            let mut headers = HeaderMap::new();
            headers.insert(header::ACCEPT, accept.parse().unwrap());
            assert_eq!(select(&headers), expected, "{accept}");
        }
        assert_eq!(escape_html("<&\"'>"), "&lt;&amp;&quot;&#39;&gt;");
    }
}
