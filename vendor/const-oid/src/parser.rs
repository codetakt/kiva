//! OID string parser with `const` support.

use crate::{encoder::Encoder, Arc, Error, ObjectIdentifier, Result};

#[derive(Debug)]
pub(crate) struct Parser {
    encoder: Encoder,
}

impl Parser {
    #[allow(clippy::integer_arithmetic)]
    pub(crate) const fn parse(s: &str) -> Result<Self> {
        let bytes = s.as_bytes();
        if bytes.is_empty() {
            return Err(Error::Empty);
        }
        let mut encoder = Encoder::new();
        let mut current: Arc = 0;
        let mut digits = false;
        let mut at = 0;
        while at < bytes.len() {
            let byte = bytes[at];
            match byte {
                b'0'..=b'9' => {
                    current = match current.checked_mul(10) {
                        Some(value) => match value.checked_add((byte - b'0') as Arc) {
                            Some(value) => value,
                            None => return Err(Error::ArcTooBig),
                        },
                        None => return Err(Error::ArcTooBig),
                    };
                    digits = true;
                }
                b'.' if digits => {
                    if at + 1 == bytes.len() {
                        return Err(Error::TrailingDot);
                    }
                    encoder = match encoder.arc(current) {
                        Ok(value) => value,
                        Err(error) => return Err(error),
                    };
                    current = 0;
                    digits = false;
                }
                actual => return Err(Error::DigitExpected { actual }),
            }
            at += 1;
        }
        match encoder.arc(current) {
            Ok(encoder) => Ok(Self { encoder }),
            Err(error) => Err(error),
        }
    }

    pub(crate) const fn finish(self) -> Result<ObjectIdentifier> {
        self.encoder.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::Parser;
    use crate::Error;

    #[test]
    fn parse() {
        let oid = Parser::parse("1.23.456").unwrap().finish().unwrap();
        assert_eq!(oid, "1.23.456".parse().unwrap());
    }

    #[test]
    fn reject_empty_string() {
        assert_eq!(Parser::parse("").err().unwrap(), Error::Empty);
    }

    #[test]
    fn reject_non_digits() {
        assert_eq!(
            Parser::parse("X").err().unwrap(),
            Error::DigitExpected { actual: b'X' }
        );

        assert_eq!(
            Parser::parse("1.2.X").err().unwrap(),
            Error::DigitExpected { actual: b'X' }
        );
    }

    #[test]
    fn reject_trailing_dot() {
        assert_eq!(Parser::parse("1.23.").err().unwrap(), Error::TrailingDot);
    }
}
