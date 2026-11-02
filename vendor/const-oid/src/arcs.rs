//! Integer arcs within an OID hierarchy.

use crate::{Error, ObjectIdentifier, Result};

/// An individual OID arc. This u32 domain includes RFC 5280's mandatory range.
pub type Arc = u32;
pub(crate) const ARC_MAX_FIRST: Arc = 2;
pub(crate) const ARC_MAX_SECOND: Arc = 39;

/// Iterator over all arcs, including both components of the first subidentifier.
pub struct Arcs<'a> {
    oid: &'a ObjectIdentifier,
    cursor: usize,
    second: Option<Arc>,
}

impl<'a> Arcs<'a> {
    pub(crate) fn new(oid: &'a ObjectIdentifier) -> Self {
        Self {
            oid,
            cursor: 0,
            second: None,
        }
    }

    pub(crate) fn try_next(&mut self) -> Result<Option<Arc>> {
        if let Some(second) = self.second.take() {
            return Ok(Some(second));
        }
        if self.cursor == self.oid.as_bytes().len() {
            return Ok(None);
        }
        let first = self.cursor == 0;
        let maximum = u64::from(Arc::MAX).saturating_add(if first { 80 } else { 0 });
        let mut value = 0_u64;
        let mut leading = true;
        loop {
            let byte = *self.oid.as_bytes().get(self.cursor).ok_or(Error::Base128)?;
            if leading && byte == 128 {
                return Err(Error::Base128);
            }
            leading = false;
            self.cursor = checked_add!(self.cursor, 1);
            value = value
                .checked_mul(128)
                .and_then(|v| v.checked_add(u64::from(byte & 127)))
                .filter(|v| *v <= maximum)
                .ok_or(Error::ArcTooBig)?;
            if byte & 128 == 0 {
                break;
            }
        }
        if first {
            let (root, second) = if value < 40 {
                (0, value)
            } else if value < 80 {
                (1, value.saturating_sub(40))
            } else {
                (2, value.saturating_sub(80))
            };
            self.second = Some(Arc::try_from(second).map_err(|_| Error::ArcTooBig)?);
            Ok(Some(root))
        } else {
            Ok(Some(Arc::try_from(value).map_err(|_| Error::ArcTooBig)?))
        }
    }
}

impl Iterator for Arcs<'_> {
    type Item = Arc;
    fn next(&mut self) -> Option<Arc> {
        // Constructors validate all arcs before exposing the infallible iterator.
        self.try_next().expect("OID malformed")
    }
}
