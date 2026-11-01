//! RC4 (ARC4), used by the `/V2` crypt filter and by Algorithms 3–7 of the standard security
//! handler (ISO 32000-1 §7.6.2, §7.6.3). A broken cipher: Lamina PDF only *reads* it, so this
//! module is crate-private and nothing public offers RC4 for writing.

/// RC4 keystream applied to `data` under `key` (encryption and decryption are the same
/// operation). An empty key is treated as a single zero byte, so no input can panic.
#[expect(
    clippy::indexing_slicing,
    reason = "every index is a u8 (or 0..256) into a 256-entry table, so always in bounds"
)]
pub(crate) fn apply(key: &[u8], data: &[u8]) -> Vec<u8> {
    let key: &[u8] = if key.is_empty() { &[0] } else { key };
    let mut s: [u8; 256] = core::array::from_fn(|i| u8::try_from(i).unwrap_or_default());
    let mut j: u8 = 0;
    for (i, k) in (0..256).zip(key.iter().cycle()) {
        j = j.wrapping_add(s[i]).wrapping_add(*k);
        s.swap(i, usize::from(j));
    }
    let (mut i, mut j) = (0u8, 0u8);
    data.iter()
        .map(|byte| {
            i = i.wrapping_add(1);
            j = j.wrapping_add(s[usize::from(i)]);
            s.swap(usize::from(i), usize::from(j));
            let k = s[usize::from(s[usize::from(i)].wrapping_add(s[usize::from(j)]))];
            byte ^ k
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::apply;

    // RFC 6229-style published test vectors (Wikipedia's "Key"/"Plaintext" set).
    #[test]
    fn apply_matches_the_key_plaintext_vector() {
        assert_eq!(
            apply(b"Key", b"Plaintext"),
            [0xBB, 0xF3, 0x16, 0xE8, 0xD9, 0x40, 0xAF, 0x0A, 0xD3]
        );
    }

    #[test]
    fn apply_matches_the_secret_attack_at_dawn_vector() {
        assert_eq!(
            apply(b"Secret", b"Attack at dawn"),
            [
                0x45, 0xA0, 0x1F, 0x64, 0x5F, 0xC3, 0x5B, 0x38, 0x35, 0x52, 0x54, 0x4B, 0x9B, 0xF5
            ]
        );
    }

    #[test]
    fn apply_twice_is_the_identity() {
        assert_eq!(apply(b"k", &apply(b"k", b"round trip")), b"round trip");
    }

    #[test]
    fn apply_tolerates_an_empty_key() {
        assert_eq!(apply(b"", b"abc").len(), 3);
    }
}
