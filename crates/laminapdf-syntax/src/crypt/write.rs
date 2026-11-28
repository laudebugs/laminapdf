//! Writing the standard security handler at revision 6 (ISO 32000-2 §7.6.4: Algorithms 8–10
//! with 2.B) and encrypting strings and streams (§7.6.3). AES-256 only: nothing here writes RC4,
//! AES-128 or revisions 2–5 as a *new* handler. [`encrypt`] also covers those ciphers, so a
//! document opened with one can be saved under the key it was opened with.
//!
//! Core has no ambient randomness. Every random value (the salts, the file key and the IVs)
//! comes from an [`Entropy`] the caller passes; [`SeededEntropy`] expands one 32-byte seed from
//! `crypto.getRandomValues` or `getrandom`, and gives the same bytes for the same seed, which is
//! what makes a save reproducible in tests.

use std::fmt;

use sha2::{Digest, Sha256};

use super::{Cipher, FileKey, Permissions, aes, candidates, key, rc4};

/// A source of random bytes, injected by the host (core reads no clock and no OS source).
///
/// ```
/// use laminapdf_syntax::crypt::{Entropy, SeededEntropy};
///
/// let mut a = SeededEntropy::new([7; 32]);
/// let mut b = SeededEntropy::new([7; 32]);
/// let (mut x, mut y) = ([0u8; 40], [0u8; 40]);
/// a.fill(&mut x);
/// b.fill(&mut y);
/// assert_eq!(x, y);
/// ```
///
/// Added in 0.7.0.
pub trait Entropy {
    /// Fills `dest` with random bytes.
    fn fill(&mut self, dest: &mut [u8]);
}

/// An [`Entropy`] that expands a 32-byte seed with SHA-256 in counter mode (the construction of
/// NIST SP 800-90A's `Hash_df`): the same seed gives the same stream, and the stream reveals
/// nothing about the seed.
///
/// The host must draw the seed from a cryptographic source (`crypto.getRandomValues`,
/// `getrandom`). A fixed seed is for tests: a file written with one has predictable keys.
///
/// Added in 0.7.0.
#[derive(Clone, PartialEq, Eq)]
pub struct SeededEntropy {
    seed: [u8; 32],
    counter: u64,
}

impl SeededEntropy {
    /// A stream from `seed`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn new(seed: [u8; 32]) -> Self {
        Self { seed, counter: 0 }
    }
}

impl fmt::Debug for SeededEntropy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SeededEntropy(<redacted>)")
    }
}

impl Entropy for SeededEntropy {
    fn fill(&mut self, dest: &mut [u8]) {
        for chunk in dest.chunks_mut(32) {
            let mut h = Sha256::new();
            h.update(self.seed);
            h.update(self.counter.to_be_bytes());
            self.counter = self.counter.wrapping_add(1);
            let block = h.finalize();
            for (d, s) in chunk.iter_mut().zip(block.iter()) {
                *d = *s;
            }
        }
    }
}

fn random<const N: usize>(entropy: &mut dyn Entropy) -> [u8; N] {
    let mut out = [0u8; N];
    entropy.fill(&mut out);
    out
}

/// How much of printing a new document allows (Table 22 bits 3 and 12).
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum PrintRight {
    /// No printing (neither bit).
    None,
    /// Low-resolution printing only (bit 3).
    LowResolution,
    /// Faithful printing (bits 3 and 12).
    #[default]
    HighResolution,
}

/// What a new handler lets a user without the owner password do: the controls of the Protect
/// dialog, mapped to `/P` by [`Grant::p`]. Bit 10 (accessibility) is not a field: it is always
/// set, so a protected file stays readable by assistive technology.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[expect(clippy::struct_excessive_bools, reason = "one bool per bit of Table 22")]
#[non_exhaustive]
pub struct Grant {
    /// Printing.
    pub print: PrintRight,
    /// Copy text and images (bit 5).
    pub extract: bool,
    /// Fill in forms and sign (bit 9).
    pub fill_forms: bool,
    /// Add comments (bit 6).
    pub annotate: bool,
    /// Insert, rotate or delete pages (bit 11).
    pub assemble: bool,
    /// Edit the contents (bit 4).
    pub modify: bool,
}

impl Grant {
    /// Everything allowed.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn all() -> Self {
        Self {
            print: PrintRight::HighResolution,
            extract: true,
            fill_forms: true,
            annotate: true,
            assemble: true,
            modify: true,
        }
    }

    /// The `/P` value: the reserved bits ISO 32000-2 Table 22 sets (7, 8 and 13–32), bit 10, and
    /// the bits this grant allows.
    ///
    /// ```
    /// use laminapdf_syntax::crypt::Grant;
    ///
    /// assert_eq!(Grant::all().p(), 0xFFFF_FFFC);
    /// ```
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn p(&self) -> u32 {
        let mut p = 0xFFFF_F0C0 | Permissions::ACCESSIBILITY;
        if !matches!(self.print, PrintRight::None) {
            p |= Permissions::PRINT;
        }
        if matches!(self.print, PrintRight::HighResolution) {
            p |= Permissions::PRINT_HIGH;
        }
        if self.modify {
            p |= Permissions::MODIFY;
        }
        if self.extract {
            p |= Permissions::EXTRACT;
        }
        if self.annotate {
            p |= Permissions::ANNOTATE;
        }
        if self.fill_forms {
            p |= Permissions::FILL_FORMS;
        }
        if self.assemble {
            p |= Permissions::ASSEMBLE;
        }
        p
    }
}

/// What a new R6 handler is made from.
///
/// Added in 0.7.0.
#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct NewHandler<'a> {
    /// The user ("open") password, as typed; empty for none.
    pub user: &'a [u8],
    /// The owner ("permissions") password, as typed. Empty is replaced by random bytes, since
    /// R6 needs a distinct owner entry.
    pub owner: &'a [u8],
    /// What a user without the owner password may do.
    pub grant: Grant,
    /// Whether the XMP metadata stream is encrypted (`/EncryptMetadata`).
    pub encrypt_metadata: bool,
}

impl<'a> NewHandler<'a> {
    /// A handler with `user` and `owner`, everything granted and metadata encrypted.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn new(user: &'a [u8], owner: &'a [u8]) -> Self {
        Self {
            user,
            owner,
            grant: Grant::all(),
            encrypt_metadata: true,
        }
    }
}

impl fmt::Debug for NewHandler<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NewHandler")
            .field("grant", &self.grant)
            .field("encrypt_metadata", &self.encrypt_metadata)
            .finish_non_exhaustive()
    }
}

/// A written R6 handler: the `/Encrypt` entries and the file key.
///
/// Added in 0.7.0.
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Handler {
    /// `/U`: 32-byte hash ‖ validation salt ‖ key salt.
    pub user: [u8; 48],
    /// `/UE`.
    pub user_encryption: [u8; 32],
    /// `/O`.
    pub owner: [u8; 48],
    /// `/OE`.
    pub owner_encryption: [u8; 32],
    /// `/Perms`.
    pub perms: [u8; 16],
    /// `/P`.
    pub permissions: u32,
    /// `/EncryptMetadata`.
    pub encrypt_metadata: bool,
    /// The file key (never printed).
    pub key: FileKey,
    /// True when the owner password was empty and a random one was made: the caller tells the
    /// user to keep [`Handler::generated_owner`], since nothing else can unlock the file.
    pub owner_generated: bool,
    /// The random owner password, when [`Handler::owner_generated`].
    generated_owner: Option<Vec<u8>>,
}

impl Handler {
    /// The owner password made for an empty one, as hex text; `None` when the caller gave one.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn generated_owner(&self) -> Option<String> {
        self.generated_owner.as_ref().map(|bytes| {
            bytes
                .iter()
                .flat_map(|b| [b >> 4, b & 15])
                .filter_map(|n| char::from_digit(u32::from(n), 16))
                .collect()
        })
    }
}

impl fmt::Debug for Handler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handler")
            .field("permissions", &self.permissions)
            .field("encrypt_metadata", &self.encrypt_metadata)
            .field("owner_generated", &self.owner_generated)
            .finish_non_exhaustive()
    }
}

/// The password as the hash takes it (SASLprep, at most 127 bytes), the form
/// [`authenticate`](super::authenticate) tries first.
fn prepare(password: &[u8]) -> Vec<u8> {
    candidates(password, 6).into_iter().next().unwrap_or_default()
}

/// Algorithm 8 (`/U`, `/UE`), Algorithm 9 (`/O`, `/OE`) and Algorithm 10 (`/Perms`).
///
/// With both passwords empty the file would open for anyone and could not be changed: the
/// caller refuses that case (`encryption-no-password`) before it gets here.
///
/// Added in 0.7.0.
#[must_use]
pub fn new_handler(params: &NewHandler<'_>, entropy: &mut dyn Entropy) -> Handler {
    let file_key = random::<32>(entropy);
    let (generated_owner, owner_password) = if params.owner.is_empty() {
        let bytes = random::<16>(entropy).to_vec();
        let text = bytes
            .iter()
            .flat_map(|b| [b >> 4, b & 15])
            .filter_map(|n| char::from_digit(u32::from(n), 16))
            .collect::<String>()
            .into_bytes();
        (Some(bytes), text)
    } else {
        (None, prepare(params.owner))
    };
    let user_password = prepare(params.user);
    let wrap = |password: &[u8], key_salt: &[u8], udata: &[u8]| -> [u8; 32] {
        let intermediate = key::hash_r6(password, key_salt, udata);
        let mut wrapped = file_key;
        // 32 bytes is a whole number of blocks, and the key is 32 bytes: the call cannot fail.
        let _ = aes::cbc_encrypt_in_place(&intermediate, &[0; 16], &mut wrapped);
        wrapped
    };
    let entry = |password: &[u8], udata: &[u8], entropy: &mut dyn Entropy| -> ([u8; 48], [u8; 32]) {
        let validation = random::<8>(entropy);
        let key_salt = random::<8>(entropy);
        let mut out = [0u8; 48];
        out[..32].copy_from_slice(&key::hash_r6(password, &validation, udata));
        out[32..40].copy_from_slice(&validation);
        out[40..].copy_from_slice(&key_salt);
        (out, wrap(password, &key_salt, udata))
    };
    let (user, user_encryption) = entry(&user_password, &[], entropy);
    let (owner, owner_encryption) = entry(&owner_password, &user, entropy);
    let permissions = params.grant.p();
    let mut perms = [0u8; 16];
    perms[..4].copy_from_slice(&permissions.to_le_bytes());
    perms[4..8].fill(0xFF);
    perms[8] = if params.encrypt_metadata { b'T' } else { b'F' };
    perms[9..12].copy_from_slice(b"adb");
    perms[12..].copy_from_slice(&random::<4>(entropy));
    let perms = aes::ecb_encrypt_block(&file_key, &perms).unwrap_or_default();
    Handler {
        user,
        user_encryption,
        owner,
        owner_encryption,
        perms,
        permissions,
        encrypt_metadata: params.encrypt_metadata,
        key: FileKey::from_bytes(file_key.to_vec()),
        owner_generated: generated_owner.is_some(),
        generated_owner,
    }
}

/// Encrypts the string or stream data of object `num gen` (ISO 32000-2 §7.6.3, the inverse of
/// [`decrypt`](super::decrypt)): AES gets a fresh 16-byte IV from `entropy` in front of the
/// PKCS#5-padded ciphertext, RC4 is the same keystream, [`Cipher::Identity`] copies. `None` when
/// the key has the wrong length for the cipher: the data is never returned as it was, so a
/// caller that can't encrypt has to say so.
///
/// Added in 0.7.0.
#[must_use]
pub fn encrypt(
    cipher: Cipher,
    key: &FileKey,
    num: u32,
    gen_nr: u16,
    entropy: &mut dyn Entropy,
    data: &[u8],
) -> Option<Vec<u8>> {
    match cipher {
        Cipher::Rc4 => Some(rc4::apply(&key::object_key(&key.0, num, gen_nr, false), data)),
        Cipher::Aes128 => aes::encrypt_padded(&key::object_key(&key.0, num, gen_nr, true), &random(entropy), data),
        Cipher::Aes256 if key.0.len() == 32 => aes::encrypt_padded(&key.0, &random(entropy), data),
        Cipher::Aes256 => None,
        Cipher::Identity => Some(data.to_vec()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypt::{PasswordKind, StandardSecurity, authenticate, decrypt};

    fn entropy() -> SeededEntropy {
        SeededEntropy::new([9; 32])
    }

    fn open(h: &Handler, password: &[u8]) -> Option<crate::crypt::Authenticated> {
        let sec = StandardSecurity {
            revision: 6,
            key_length: 32,
            owner: &h.owner,
            user: &h.user,
            owner_encryption: &h.owner_encryption,
            user_encryption: &h.user_encryption,
            perms: Some(&h.perms),
            permissions: h.permissions,
            id0: b"",
            encrypt_metadata: h.encrypt_metadata,
        };
        authenticate(&sec, password)
    }

    #[test]
    fn the_user_password_opens_the_handler_with_the_file_key() {
        let h = new_handler(&NewHandler::new(b"open", b"owner"), &mut entropy());
        let a = open(&h, b"open").unwrap();
        assert_eq!(
            (a.matched, a.key == h.key, a.perms_valid),
            (PasswordKind::User, true, Some(true))
        );
    }

    #[test]
    fn the_owner_password_opens_the_handler_with_the_file_key() {
        let h = new_handler(&NewHandler::new(b"open", b"owner"), &mut entropy());
        let a = open(&h, b"owner").unwrap();
        assert_eq!((a.matched, a.key == h.key), (PasswordKind::Owner, true));
    }

    #[test]
    fn another_password_opens_nothing() {
        let h = new_handler(&NewHandler::new(b"open", b"owner"), &mut entropy());
        assert!(open(&h, b"guess").is_none() && open(&h, b"").is_none());
    }

    #[test]
    fn an_empty_user_password_opens_without_a_prompt() {
        let h = new_handler(&NewHandler::new(b"", b"owner"), &mut entropy());
        assert_eq!(open(&h, b"").unwrap().matched, PasswordKind::User);
    }

    #[test]
    fn an_empty_owner_password_is_replaced_by_one_that_opens_the_file() {
        let h = new_handler(&NewHandler::new(b"open", b""), &mut entropy());
        let made = h.generated_owner().unwrap();
        assert_eq!((h.owner_generated, made.len()), (true, 32));
        assert_eq!(open(&h, made.as_bytes()).unwrap().matched, PasswordKind::Owner);
    }

    #[test]
    fn a_given_owner_password_is_not_reported_as_generated() {
        let h = new_handler(&NewHandler::new(b"open", b"owner"), &mut entropy());
        assert_eq!((h.owner_generated, h.generated_owner()), (false, None));
    }

    #[test]
    fn the_same_seed_writes_the_same_handler() {
        let params = NewHandler::new(b"open", b"owner");
        let (a, b) = (
            new_handler(&params, &mut entropy()),
            new_handler(&params, &mut entropy()),
        );
        assert_eq!(
            (a.user, a.owner, a.perms, a.key == b.key),
            (b.user, b.owner, b.perms, true)
        );
    }

    #[test]
    fn another_seed_writes_another_file_key() {
        let params = NewHandler::new(b"open", b"owner");
        let other = new_handler(&params, &mut SeededEntropy::new([1; 32]));
        assert_ne!(new_handler(&params, &mut entropy()).key, other.key);
    }

    #[test]
    fn perms_records_p_and_the_metadata_flag() {
        let mut params = NewHandler::new(b"open", b"owner");
        params.encrypt_metadata = false;
        let h = new_handler(&params, &mut entropy());
        let plain = aes::ecb_decrypt_block(&h.key.0, &h.perms).unwrap();
        assert_eq!(
            (&plain[..4], &plain[4..8], plain[8], &plain[9..12]),
            (&h.permissions.to_le_bytes()[..], &[0xFF; 4][..], b'F', &b"adb"[..])
        );
    }

    #[test]
    fn full_rights_make_p_all_ones_but_the_two_low_bits() {
        assert_eq!(Grant::all().p(), 0xFFFF_FFFC);
    }

    #[test]
    fn bit_10_stays_set_when_everything_else_is_refused() {
        let none = Grant {
            print: PrintRight::None,
            extract: false,
            fill_forms: false,
            annotate: false,
            assemble: false,
            modify: false,
        };
        let p = Permissions::new(i64::from(none.p()), 6);
        assert_eq!(p.bits(), 1 << 9);
        assert!(p.accessibility() && !p.print_low_res() && !p.extract() && !p.annotate() && !p.modify());
    }

    #[test]
    fn each_control_sets_only_its_bits() {
        let base = Grant {
            print: PrintRight::None,
            extract: false,
            fill_forms: false,
            annotate: false,
            assemble: false,
            modify: false,
        };
        let bits = |g: Grant| g.p() ^ base.p();
        assert_eq!(
            bits(Grant {
                print: PrintRight::LowResolution,
                ..base
            }),
            1 << 2
        );
        assert_eq!(
            bits(Grant {
                print: PrintRight::HighResolution,
                ..base
            }),
            (1 << 2) | (1 << 11)
        );
        assert_eq!(bits(Grant { extract: true, ..base }), 1 << 4);
        assert_eq!(
            bits(Grant {
                fill_forms: true,
                ..base
            }),
            1 << 8
        );
        assert_eq!(bits(Grant { annotate: true, ..base }), 1 << 5);
        assert_eq!(bits(Grant { assemble: true, ..base }), 1 << 10);
        assert_eq!(bits(Grant { modify: true, ..base }), 1 << 3);
    }

    #[test]
    fn aes_256_data_decrypts_under_the_file_key() {
        let key = FileKey::from_bytes(vec![5; 32]);
        let sealed = encrypt(Cipher::Aes256, &key, 7, 0, &mut entropy(), b"LAMINA-SENTINEL").unwrap();
        assert_eq!(sealed.len(), 16 + 16);
        assert!(!sealed.windows(8).any(|w| w == b"SENTINEL"));
        assert_eq!(decrypt(Cipher::Aes256, &key, 7, 0, &sealed).data, b"LAMINA-SENTINEL");
    }

    #[test]
    fn a_whole_block_gets_a_whole_block_of_padding() {
        let key = FileKey::from_bytes(vec![5; 32]);
        let sealed = encrypt(Cipher::Aes256, &key, 1, 0, &mut entropy(), &[1; 16]).unwrap();
        assert_eq!(sealed.len(), 16 + 32);
    }

    #[test]
    fn every_encryption_draws_a_new_iv() {
        let key = FileKey::from_bytes(vec![5; 32]);
        let mut e = entropy();
        let a = encrypt(Cipher::Aes256, &key, 1, 0, &mut e, b"same").unwrap();
        let b = encrypt(Cipher::Aes256, &key, 1, 0, &mut e, b"same").unwrap();
        assert!(a[..16] != b[..16] && a[16..] != b[16..]);
    }

    #[test]
    fn the_older_ciphers_round_trip_for_a_resave_under_the_key_they_opened_with() {
        let key = FileKey::from_bytes(vec![3; 16]);
        for cipher in [Cipher::Rc4, Cipher::Aes128] {
            let sealed = encrypt(cipher, &key, 4, 1, &mut entropy(), b"older").unwrap();
            assert_eq!(decrypt(cipher, &key, 4, 1, &sealed).data, b"older");
        }
    }

    #[test]
    fn a_key_of_the_wrong_length_refuses_instead_of_returning_plaintext() {
        let key = FileKey::from_bytes(vec![5; 16]);
        assert_eq!(encrypt(Cipher::Aes256, &key, 1, 0, &mut entropy(), b"x"), None);
    }

    #[test]
    fn entropy_fills_more_than_one_block() {
        let mut out = [0u8; 100];
        entropy().fill(&mut out);
        assert_ne!(out[..32], out[32..64]);
    }

    #[test]
    fn debug_output_names_no_secret() {
        let h = new_handler(&NewHandler::new(b"open", b"owner"), &mut entropy());
        let text = format!("{h:?} {:?} {:?}", NewHandler::new(b"open", b"owner"), entropy());
        assert!(!text.contains("open") && !text.contains("owner:") && text.contains("redacted"));
    }
}
