//! OID encoder with `const` support.

use crate::{
    arcs::{ARC_MAX_FIRST, ARC_MAX_SECOND},
    Arc, Error, ObjectIdentifier, Result,
};

#[derive(Debug)]
pub(crate) struct Encoder {
    state: State,
    bytes: [u8; ObjectIdentifier::MAX_SIZE],
    cursor: usize,
}

#[derive(Debug)]
enum State {
    Initial,
    FirstArc(Arc),
    Body,
}

impl Encoder {
    pub(crate) const fn new() -> Self {
        Self {
            state: State::Initial,
            bytes: [0; ObjectIdentifier::MAX_SIZE],
            cursor: 0,
        }
    }

    pub(crate) const fn extend(oid: ObjectIdentifier) -> Self {
        Self {
            state: State::Body,
            bytes: oid.bytes,
            cursor: oid.length as usize,
        }
    }

    #[allow(clippy::integer_arithmetic)]
    pub(crate) const fn arc(mut self, arc: Arc) -> Result<Self> {
        match self.state {
            State::Initial => {
                if arc > ARC_MAX_FIRST {
                    return Err(Error::ArcInvalid { arc });
                }
                self.state = State::FirstArc(arc);
                Ok(self)
            }
            State::FirstArc(first) => {
                if first < 2 && arc > ARC_MAX_SECOND {
                    return Err(Error::ArcInvalid { arc });
                }
                self.state = State::Body;
                // The combined subidentifier can exceed u32 even though both
                // individual arcs fit. Encode it with the same base128 grammar.
                self.subidentifier(first as u64 * 40 + arc as u64)
            }
            State::Body => self.subidentifier(arc as u64),
        }
    }

    #[allow(clippy::integer_arithmetic)]
    const fn subidentifier(mut self, value: u64) -> Result<Self> {
        let mut width = 1;
        let mut remaining = value;
        while remaining >= 128 {
            width += 1;
            remaining >>= 7;
        }
        let end = checked_add!(self.cursor, width);
        if end > ObjectIdentifier::MAX_SIZE {
            return Err(Error::Length);
        }
        let mut at = 0;
        while at < width {
            let shift = (width - at - 1) * 7;
            let continued = if at + 1 < width { 128 } else { 0 };
            self.bytes[self.cursor + at] = ((value >> shift) as u8 & 127) | continued;
            at += 1;
        }
        self.cursor = end;
        Ok(self)
    }

    pub(crate) const fn finish(self) -> Result<ObjectIdentifier> {
        if matches!(self.state, State::Body) {
            Ok(ObjectIdentifier {
                bytes: self.bytes,
                length: self.cursor as u8,
            })
        } else {
            Err(Error::NotEnoughArcs)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Encoder;
    use hex_literal::hex;

    /// OID `1.2.840.10045.2.1` encoded as ASN.1 BER/DER
    const EXAMPLE_OID_BER: &[u8] = &hex!("2A8648CE3D0201");

    #[test]
    fn encode() {
        let encoder = Encoder::new();
        let encoder = encoder.arc(1).unwrap();
        let encoder = encoder.arc(2).unwrap();
        let encoder = encoder.arc(840).unwrap();
        let encoder = encoder.arc(10045).unwrap();
        let encoder = encoder.arc(2).unwrap();
        let encoder = encoder.arc(1).unwrap();
        assert_eq!(&encoder.bytes[..encoder.cursor], EXAMPLE_OID_BER);
    }
}
