//! Key derivation for the standard security handler: Algorithm 2 (R2–R4 file key), the
//! per-object key of §7.6.2 Algorithm 1, Algorithm 3's owner key, and the R5 / R6 password
//! hashes (Adobe Extension Level 3's single SHA-256, ISO 32000-2 Algorithm 2.B).

use md5::{Digest, Md5};
use sha2::{Sha256, Sha384, Sha512};

use super::StandardSecurity;
use super::aes;

/// The 32-byte padding string of Algorithm 2 step a (ISO 32000-1 §7.6.3.3).
pub(crate) const PAD: [u8; 32] = [
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA, 0x01, 0x08, 0x2E, 0x2E, 0x00,
    0xB6, 0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53, 0x69, 0x7A,
];

/// MD5 over the concatenation of `parts`.
pub(crate) fn md5(parts: &[&[u8]]) -> [u8; 16] {
    let mut h = Md5::new();
    for part in parts {
        h.update(part);
    }
    h.finalize().into()
}

fn sha256(parts: &[&[u8]]) -> [u8; 32] {
    let mut h = Sha256::new();
    for part in parts {
        h.update(part);
    }
    h.finalize().into()
}

/// A password truncated or padded to 32 bytes with [`PAD`] (Algorithm 2 step a).
pub(crate) fn pad_password(password: &[u8]) -> [u8; 32] {
    let password = prefix(password, 32);
    let mut out = [0u8; 32];
    for (slot, b) in out.iter_mut().zip(password.iter().chain(PAD.iter())) {
        *slot = *b;
    }
    out
}

/// The first `n` bytes of `bytes` (all of them if shorter).
pub(crate) fn prefix(bytes: &[u8], n: usize) -> &[u8] {
    bytes.get(..n).unwrap_or(bytes)
}

/// Algorithm 2: the R2–R4 file key from the padded user password.
pub(crate) fn file_key_r2_r4(sec: &StandardSecurity<'_>, padded_user: &[u8; 32]) -> Vec<u8> {
    let n = sec.key_bytes();
    let p = sec.permissions.to_le_bytes();
    let no_metadata: &[u8] = if sec.revision >= 4 && !sec.encrypt_metadata {
        &[0xFF; 4]
    } else {
        &[]
    };
    let mut h = md5(&[padded_user, prefix(sec.owner, 32), &p, sec.id0, no_metadata]);
    if sec.revision >= 3 {
        for _ in 0..50 {
            h = md5(&[prefix(&h, n)]);
        }
    }
    prefix(&h, n).to_vec()
}

/// Algorithm 3 steps a–d: the RC4 key that encrypts `/O`, from the owner password.
pub(crate) fn owner_key_r2_r4(sec: &StandardSecurity<'_>, owner_password: &[u8]) -> Vec<u8> {
    let n = sec.key_bytes();
    let mut h = md5(&[&pad_password(owner_password)]);
    if sec.revision >= 3 {
        for _ in 0..50 {
            h = md5(&[&h]);
        }
    }
    prefix(&h, n).to_vec()
}

/// Algorithm 1: the key for object `num gen` from the R2–R4 file key (`aes` adds `sAlT`).
///
/// RC4 uses the first `min(n + 5, 16)` bytes of the hash. AES-128 always uses all 16: it has no
/// shorter key, so a file key under 11 bytes (an AESV2 filter with a short `/Length`) would
/// otherwise make every object undecryptable; pdf.js and qpdf use the full hash too.
pub(crate) fn object_key(file_key: &[u8], num: u32, gen_nr: u16, aes: bool) -> Vec<u8> {
    let num = num.to_le_bytes();
    let gen_nr = gen_nr.to_le_bytes();
    let salt: &[u8] = if aes { b"sAlT" } else { &[] };
    let h = md5(&[file_key, prefix(&num, 3), &gen_nr, salt]);
    let len = if aes { 16 } else { (file_key.len() + 5).min(16) };
    prefix(&h, len).to_vec()
}

/// The R5 password hash (Adobe Extension Level 3): one SHA-256.
pub(crate) fn hash_r5(password: &[u8], salt: &[u8], udata: &[u8]) -> [u8; 32] {
    sha256(&[password, salt, udata])
}

/// Algorithm 2.B (ISO 32000-2 §7.6.4.3.4): the R6 password hash. `udata` is the 48-byte `/U`
/// for owner checks and empty for user checks.
///
/// Terminates: from round 287 on, `round - 32 ≥ 255` holds for every last byte.
pub(crate) fn hash_r6(password: &[u8], salt: &[u8], udata: &[u8]) -> [u8; 32] {
    // `k` holds the last hash (32, 48 or 64 bytes); `buf` is K1, then E in place. Both are
    // reused across rounds.
    let mut k = [0u8; 64];
    let mut k_len = 32;
    k[..32].copy_from_slice(&sha256(&[password, salt, udata]));
    let mut buf: Vec<u8> = Vec::with_capacity(64 * (password.len() + 64 + udata.len()));
    let mut round: usize = 0;
    loop {
        round += 1;
        let current = prefix(&k, k_len);
        buf.clear();
        for _ in 0..64 {
            buf.extend_from_slice(password);
            buf.extend_from_slice(current);
            buf.extend_from_slice(udata);
        }
        let (key, iv) = k.split_at(16);
        let iv: [u8; 16] = prefix(iv, 16).try_into().unwrap_or_default();
        // 64 repetitions make `buf` a whole number of blocks, and `key` is 16 bytes.
        if aes::cbc_encrypt_in_place(key, &iv, &mut buf).is_none() {
            break;
        }
        // The first 16 bytes as a big-endian number mod 3 equals their byte sum mod 3 (256 ≡ 1).
        let selector = buf.iter().take(16).map(|b| u32::from(*b)).sum::<u32>() % 3;
        k_len = match selector {
            0 => copy_into(&mut k, &Sha256::digest(&buf)),
            1 => copy_into(&mut k, &Sha384::digest(&buf)),
            _ => copy_into(&mut k, &Sha512::digest(&buf)),
        };
        let last = usize::from(buf.last().copied().unwrap_or_default());
        if round >= 64 && last + 32 <= round {
            break;
        }
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(prefix(&k, 32));
    out
}

/// Copies `src` to the front of `dst` and returns its length.
fn copy_into(dst: &mut [u8; 64], src: &[u8]) -> usize {
    for (d, s) in dst.iter_mut().zip(src) {
        *d = *s;
    }
    src.len().min(64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_password_appends_the_padding_string() {
        let padded = pad_password(b"ab");
        assert_eq!((&padded[..2], &padded[2..]), (&b"ab"[..], &PAD[..30]));
    }

    #[test]
    fn pad_password_of_empty_is_the_padding_string() {
        assert_eq!(pad_password(b""), PAD);
    }

    #[test]
    fn pad_password_truncates_at_32_bytes() {
        assert_eq!(pad_password(&[b'x'; 40]), [b'x'; 32]);
    }

    #[test]
    fn object_key_is_at_most_16_bytes() {
        assert_eq!(object_key(&[1; 16], 7, 0, true).len(), 16);
    }

    #[test]
    fn object_key_is_n_plus_5_bytes_for_40_bit_keys() {
        assert_eq!(object_key(&[1; 5], 7, 0, false).len(), 10);
    }

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    // Known answers computed independently with pypdf 6.21.3 (`AlgV5.calculate_hash`).
    #[test]
    fn hash_r6_matches_pypdf_for_a_user_check() {
        let salt = [0x83, 0xF1, 0x8F, 0x45, 0x39, 0x17, 0x04, 0x24];
        assert_eq!(
            hash_r6(b"user", &salt, &[]).to_vec(),
            hex("045cb569199d3f61c45663a4cbb0765857c7519dd6985a0e8fa06ef106dc769d")
        );
    }

    #[test]
    fn hash_r6_matches_pypdf_for_an_owner_check() {
        let udata: Vec<u8> = (0..48).collect();
        let salt: Vec<u8> = (0..8).collect();
        assert_eq!(
            hash_r6(b"owner", &salt, &udata).to_vec(),
            hex("f77bbaa82ba46eca0e3997ad8c91b9fa4a3f83750e13a950f901abef1f69b683")
        );
    }

    #[test]
    fn hash_r5_is_one_sha_256() {
        let salt: Vec<u8> = (0..8).collect();
        assert_eq!(
            hash_r5(b"user", &salt, &[]).to_vec(),
            hex("f90940351d2eddc7a5d9bf15695a020054f9854261486d8a51767ad0089e4c26")
        );
    }
}
