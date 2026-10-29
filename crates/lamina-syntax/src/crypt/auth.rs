//! Password authentication: Algorithms 4–7 (R2–R4, ISO 32000-1 §7.6.3.4), the R5 checks
//! (Adobe Extension Level 3) and Algorithms 11–12 with 2.A for R6 (ISO 32000-2 §7.6.4.4).

use super::key::{self, PAD};
use super::{StandardSecurity, aes, rc4};

/// The 20 RC4 passes of Algorithms 3 and 5: pass `i` uses `key XOR i`. `rev` runs 19 → 0
/// (decryption) instead of 0 → 19.
fn rc4_passes(key: &[u8], data: &[u8], rev: bool) -> Vec<u8> {
    let mut out = data.to_vec();
    let rounds: Vec<u8> = if rev {
        (0..=19).rev().collect()
    } else {
        (0..=19).collect()
    };
    for i in rounds {
        let k: Vec<u8> = key.iter().map(|b| b ^ i).collect();
        out = rc4::apply(&k, &out);
    }
    out
}

/// Algorithms 4 and 5 compute `/U` from a candidate file key; Algorithm 6 compares it with the
/// stored one (all 32 bytes for R2, the first 16 for R3+).
fn user_matches_r2_r4(sec: &StandardSecurity<'_>, file_key: &[u8]) -> bool {
    if sec.revision == 2 {
        let u = rc4::apply(file_key, &PAD);
        sec.user.get(..32) == Some(&u[..])
    } else {
        let hash = key::md5(&[&PAD, sec.id0]);
        let u = rc4_passes(file_key, &hash, false);
        sec.user.get(..16) == u.get(..16)
    }
}

/// Algorithm 6: the file key if `password` is the user password (R2–R4).
pub(crate) fn user_r2_r4(sec: &StandardSecurity<'_>, password: &[u8]) -> Option<Vec<u8>> {
    let file_key = key::file_key_r2_r4(sec, &key::pad_password(password));
    user_matches_r2_r4(sec, &file_key).then_some(file_key)
}

/// Algorithm 7: the file key if `password` is the owner password (R2–R4). Decrypting `/O`
/// yields the padded user password, which is then checked as in Algorithm 6.
pub(crate) fn owner_r2_r4(sec: &StandardSecurity<'_>, password: &[u8]) -> Option<Vec<u8>> {
    let owner_key = key::owner_key_r2_r4(sec, password);
    let o = sec.owner.get(..32)?;
    let user = if sec.revision == 2 {
        rc4::apply(&owner_key, o)
    } else {
        rc4_passes(&owner_key, o, true)
    };
    let padded: [u8; 32] = user.try_into().ok()?;
    let file_key = key::file_key_r2_r4(sec, &padded);
    user_matches_r2_r4(sec, &file_key).then_some(file_key)
}

/// R5/R6 hash for this revision.
fn hash(sec: &StandardSecurity<'_>, password: &[u8], salt: &[u8], udata: &[u8]) -> [u8; 32] {
    if sec.revision == 5 {
        key::hash_r5(password, salt, udata)
    } else {
        key::hash_r6(password, salt, udata)
    }
}

/// The 32-byte file key from `/UE` or `/OE`: AES-256-CBC, zero IV, no padding.
fn unwrap_file_key(intermediate: &[u8; 32], wrapped: &[u8]) -> Option<Vec<u8>> {
    aes::cbc_decrypt_raw(intermediate, &[0; 16], wrapped.get(..32)?)
}

/// Algorithm 11 + 2.A: the file key if `password` (already prepared with SASLprep and truncated) is the user
/// password (R5/R6).
pub(crate) fn user_r5_r6(sec: &StandardSecurity<'_>, password: &[u8]) -> Option<Vec<u8>> {
    let (stored, validation, key_salt) = (sec.user.get(..32)?, sec.user.get(32..40)?, sec.user.get(40..48)?);
    if hash(sec, password, validation, &[]) != stored {
        return None;
    }
    unwrap_file_key(&hash(sec, password, key_salt, &[]), sec.user_encryption)
}

/// Algorithm 12 + 2.A: the file key if `password` is the owner password (R5/R6). The owner hashes
/// also take the 48-byte `/U`.
pub(crate) fn owner_r5_r6(sec: &StandardSecurity<'_>, password: &[u8]) -> Option<Vec<u8>> {
    let u = sec.user.get(..48)?;
    let (stored, validation, key_salt) = (sec.owner.get(..32)?, sec.owner.get(32..40)?, sec.owner.get(40..48)?);
    if hash(sec, password, validation, u) != stored {
        return None;
    }
    unwrap_file_key(&hash(sec, password, key_salt, u), sec.owner_encryption)
}
