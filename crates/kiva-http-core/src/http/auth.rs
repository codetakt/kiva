/// Error returned while parsing a Bearer authorization header value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BearerHeaderError {
    /// The header used a scheme other than Bearer.
    UnsupportedScheme,
    /// The header used Bearer without a token.
    EmptyToken,
}

/// Returns whether an `X-Experience-API-Version` value is supported.
#[must_use]
pub const fn xapi_version_is_supported(value: &str) -> bool {
    matches!(value.as_bytes(), b"2.0" | b"2.0.0")
}

/// Extracts the Bearer token from an optional Authorization header value.
///
/// ASCII/UTF-8 validation belongs to the HTTP adapter. This function only
/// classifies an already decoded header value.
pub fn bearer_token(value: Option<&str>) -> Result<Option<&str>, BearerHeaderError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let Some(token_start) = bearer_token_start(Some(value))? else {
        return Ok(None);
    };
    Ok(Some(&value[token_start..]))
}

/// Returns the byte offset where the Bearer token starts.
///
/// This deliberately avoids `str::strip_prefix` so Kani can verify the
/// classifier without entering unbounded standard-library slice internals.
pub fn bearer_token_start(value: Option<&str>) -> Result<Option<usize>, BearerHeaderError> {
    const PREFIX: &[u8; 7] = b"Bearer ";

    let Some(value) = value else {
        return Ok(None);
    };
    let bytes = value.as_bytes();
    if bytes.len() < PREFIX.len() {
        return Err(BearerHeaderError::UnsupportedScheme);
    }

    let mut index = 0;
    while index < PREFIX.len() {
        if bytes[index] != PREFIX[index] {
            return Err(BearerHeaderError::UnsupportedScheme);
        }
        index += 1;
    }

    if bytes.len() == PREFIX.len() {
        return Err(BearerHeaderError::EmptyToken);
    }
    Ok(Some(PREFIX.len()))
}
