//! Complete attachment digests selected by the supplied hash length.

use sha2::{Digest, Sha224, Sha256, Sha384, Sha512};

pub(super) fn attachment_hash_matches(body: &[u8], expected: &str) -> bool {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    // DC-MIME incorporates FIPS 180-2 with Change Notice 1. Select the
    // complete digest by its encoded length; never truncate a longer digest.
    let digest = match expected.len() {
        56 => Sha224::digest(body).to_vec(),
        64 => Sha256::digest(body).to_vec(),
        96 => Sha384::digest(body).to_vec(),
        128 => Sha512::digest(body).to_vec(),
        _ => return false,
    };
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push(char::from(HEX[usize::from(byte >> 4)]));
        hex.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    hex.eq_ignore_ascii_case(expected)
}
