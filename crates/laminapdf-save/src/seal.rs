//! Encrypting what a save writes (E11 plan §2.1, §3): a new AES-256 handler for a pending
//! [`SetEncryption`], or the protection the document was opened with, kept.

use std::cell::RefCell;

use laminapdf_cos::{Document, Encryptor, Object};
use laminapdf_ops::{Secret, SetEncryption};
use laminapdf_syntax::crypt::{NewHandler, SeededEntropy, new_handler};

use crate::error::{Result, SaveError};
use crate::options::SaveOptions;

/// The encryptor a save writes with, the entropy it draws IVs from, and what the save reports.
pub(crate) struct Seal<'d> {
    doc: &'d Document,
    encryptor: Encryptor,
    entropy: RefCell<SeededEntropy>,
    /// A new handler (the document's passwords or limits change), not the one it was opened with.
    new: bool,
    generated_owner: Option<Secret>,
}

impl std::fmt::Debug for Seal<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Seal").field("new", &self.new).finish_non_exhaustive()
    }
}

impl<'d> Seal<'d> {
    /// How `doc` is written under `options` and the pending `new` protection: a new handler for
    /// `new`, the document's own for an encrypted document unless the options remove it, and
    /// none otherwise.
    ///
    /// # Errors
    /// [`SaveError::EncryptionNoEntropy`] when the save encrypts and the options hold no seed.
    pub(crate) fn choose(doc: &'d Document, options: &SaveOptions, new: Option<&SetEncryption>) -> Result<Option<Self>> {
        if new.is_none() && (!doc.is_encrypted() || options.remove_encryption()) {
            return Ok(None);
        }
        let mut entropy = SeededEntropy::new(options.entropy_seed().ok_or(SaveError::EncryptionNoEntropy)?);
        let (encryptor, generated_owner) = match new {
            Some(set) => {
                let mut params = NewHandler::new(set.user.expose(), set.owner.expose());
                params.grant = set.permissions.grant(!set.owner.is_empty());
                params.encrypt_metadata = set.encrypt_metadata;
                let handler = new_handler(&params, &mut entropy);
                let generated = handler.generated_owner().map(Secret::new);
                (Encryptor::new_r6(&handler), generated)
            }
            None => (doc.encryptor().ok_or(SaveError::EncryptionNoEntropy)?, None),
        };
        Ok(Some(Self {
            doc,
            encryptor,
            entropy: RefCell::new(entropy),
            new: new.is_some(),
            generated_owner,
        }))
    }

    /// Whether the handler is new, so objects read from the base are written as plaintext first
    /// (their old `/Crypt` filters mean nothing under it).
    pub(crate) const fn is_new(&self) -> bool {
        self.new
    }

    /// The `/Encrypt` dictionary to write.
    pub(crate) fn dictionary(&self) -> Object {
        Object::Dict(self.encryptor.dictionary().clone())
    }

    /// The encryptor the output is written under, for the rebase onto it.
    pub(crate) fn encryptor(&self) -> Encryptor {
        self.encryptor.clone()
    }

    pub(crate) fn generated_owner(&self) -> Option<Secret> {
        self.generated_owner.clone()
    }

    /// Encrypts object `num gen` in place.
    ///
    /// # Errors
    /// [`SaveError::EncryptionIncomplete`] when a string or stream could not be encrypted.
    pub(crate) fn object(&self, object: &mut Object, num: u32, generation: u16) -> Result<()> {
        let mut resolve = |value: &Object| self.doc.deref(value).ok().map(|v| v.into_owned());
        let sealed = self
            .encryptor
            .encrypt_object(object, num, generation, &mut *self.entropy.borrow_mut(), &mut resolve);
        if sealed { Ok(()) } else { Err(SaveError::EncryptionIncomplete) }
    }
}
