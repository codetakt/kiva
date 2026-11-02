//! Structured media fields shared by JSON metadata and transport admission.

use std::collections::BTreeMap;

/// A parsed Internet media type, with decoded parameter values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaType {
    /// Case-normalized type and subtype, separated by a slash.
    pub essence: String,
    /// Case-normalized parameter names and decoded values.
    pub parameters: BTreeMap<String, String>,
}

/// A malformed or unsupported structured media field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid or unsupported structured MIME field")]
pub struct MediaTypeError;

struct Field<'a> {
    remaining: &'a [u8],
    mime: bool,
}

impl<'a> Field<'a> {
    fn new(text: &'a str, mime: bool) -> Result<Self, MediaTypeError> {
        if !mime && text.contains(['\r', '\n'])
            || mime && (!text.is_ascii() || text.contains("\r\n"))
        {
            return Err(MediaTypeError);
        }
        Ok(Self {
            remaining: text.as_bytes(),
            mime,
        })
    }

    fn take(&mut self) -> Result<u8, MediaTypeError> {
        let (byte, rest) = self.remaining.split_first().ok_or(MediaTypeError)?;
        self.remaining = rest;
        Ok(*byte)
    }

    fn skip(&mut self) -> Result<(), MediaTypeError> {
        loop {
            while matches!(self.remaining.first(), Some(b' ' | b'\t')) {
                self.take()?;
            }
            if !self.mime || self.remaining.first() != Some(&b'(') {
                return Ok(());
            }
            self.take()?;
            let mut depth = 1_usize;
            while depth != 0 {
                match self.take()? {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    b'\\' => {
                        self.take()?;
                    }
                    b'\r' => return Err(MediaTypeError),
                    _ => {}
                }
            }
        }
    }

    fn token(&mut self) -> Result<String, MediaTypeError> {
        let length = self
            .remaining
            .iter()
            .take_while(|&&c| {
                c.is_ascii_graphic()
                    && !b"()<>@,;:\\\"/[]?=".contains(&c)
                    && (self.mime || !b"{}".contains(&c))
            })
            .count();
        if length == 0 {
            return Err(MediaTypeError);
        }
        let value =
            String::from_utf8(self.remaining[..length].to_vec()).map_err(|_| MediaTypeError)?;
        self.remaining = &self.remaining[length..];
        Ok(value)
    }

    fn literal(&mut self, expected: u8) -> Result<(), MediaTypeError> {
        if self.mime {
            self.skip()?;
        }
        if self.take()? != expected {
            return Err(MediaTypeError);
        }
        if self.mime {
            self.skip()?;
        }
        Ok(())
    }

    fn value(&mut self) -> Result<String, MediaTypeError> {
        if self.remaining.first() != Some(&b'"') {
            return self.token();
        }
        self.take()?;
        let mut value = Vec::new();
        loop {
            match self.take()? {
                b'"' => return String::from_utf8(value).map_err(|_| MediaTypeError),
                b'\\' => {
                    let byte = self.take()?;
                    if !byte.is_ascii() {
                        return Err(MediaTypeError);
                    }
                    value.push(byte);
                }
                byte => {
                    if self.mime && byte == b'\r' {
                        return Err(MediaTypeError);
                    }
                    if !self.mime && (byte < 32 && byte != b'\t' || byte == 127) {
                        return Err(MediaTypeError);
                    }
                    value.push(byte);
                }
            }
        }
    }
}

/// Parses an unfolded MIME media type, or an HTTP envelope value when `mime` is false.
/// Duplicate parameters are rejected under Kiva's declared admission policy.
pub fn parse_media_type(text: &str, mime: bool) -> Result<MediaType, MediaTypeError> {
    let mut field = Field::new(text, mime)?;
    field.skip()?;
    let kind = field.token()?.to_ascii_lowercase();
    field.literal(b'/')?;
    let subtype = field.token()?.to_ascii_lowercase();
    let mut parameters = BTreeMap::new();
    loop {
        field.skip()?;
        if field.remaining.is_empty() {
            break;
        }
        if field.take()? != b';' {
            return Err(MediaTypeError);
        }
        field.skip()?;
        let name = field.token()?.to_ascii_lowercase();
        field.literal(b'=')?;
        let value = field.value()?;
        // Kiva rejects ambiguous duplicate parameters. This is a declared
        // admission policy, not an inferred universal MIME equivalence rule.
        if parameters.insert(name, value).is_some() {
            return Err(MediaTypeError);
        }
    }
    Ok(MediaType {
        essence: format!("{kind}/{subtype}"),
        parameters,
    })
}

/// Validates a MIME Content-Transfer-Encoding value under the xAPI binary-only contract.
pub fn validate_binary_transfer_encoding(text: &str) -> Result<(), MediaTypeError> {
    let mut field = Field::new(text, true)?;
    field.skip()?;
    if !field.token()?.eq_ignore_ascii_case("binary") {
        return Err(MediaTypeError);
    }
    field.skip()?;
    if field.remaining.is_empty() {
        Ok(())
    } else {
        Err(MediaTypeError)
    }
}
