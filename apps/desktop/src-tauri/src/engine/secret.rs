//! [`SessionPassword`]: an unlocked document's password, kept by the main process while the
//! document is open so a restarted engine can unlock it again without asking the user (ADR 0008
//! §2). A crash isn't the user's fault, and a prompt per crash per document would punish them.
//!
//! The bytes live in one heap buffer that never moves, locked into RAM where the OS allows it
//! (`mlock` / `VirtualLock`), so they don't reach swap, and wiped when the password is dropped:
//! when its document closes, its window closes, the app exits or an unlock with it fails. They
//! are never serialised, logged, saved in a session file or sent to the webview. Locking doesn't
//! keep them out of a core dump or a debugger attached to the main process.

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};

use laminapdf_desktop_protocol::Password;
use zeroize::Zeroizing;

/// Set once a lock has failed, so the warning shows once per run.
static LOCK_WARNED: AtomicBool = AtomicBool::new(false);

/// A document's password, wiped when dropped.
pub(crate) struct SessionPassword {
    // Declared before `_lock`, so the bytes are wiped before the pages are unlocked.
    bytes: Zeroizing<Box<[u8]>>,
    /// Keeps the buffer's pages locked; `None` when locking failed or the password is empty.
    _lock: Option<region::LockGuard>,
}

impl SessionPassword {
    /// Copies `password` into a locked buffer.
    pub(crate) fn new(password: &str) -> Self {
        let bytes: Zeroizing<Box<[u8]>> = Zeroizing::new(password.as_bytes().into());
        let lock = if bytes.is_empty() {
            None
        } else {
            match region::lock(bytes.as_ptr(), bytes.len()) {
                Ok(guard) => Some(guard),
                Err(error) => {
                    if !LOCK_WARNED.swap(true, Ordering::Relaxed) {
                        tracing::warn!("could not lock a password into memory ({error}); it may reach swap");
                    }
                    None
                }
            }
        };
        Self { bytes, _lock: lock }
    }

    /// The password for one `UnlockDocument` message (wiped when that message is dropped).
    pub(crate) fn to_password(&self) -> Password {
        Password::new(String::from_utf8_lossy(&self.bytes).into_owned())
    }

    /// The bytes, for tests to see that a cleared password holds none.
    #[cfg(test)]
    fn peek(&self) -> &[u8] {
        &self.bytes
    }

    /// Wipes the bytes now (dropping does the same).
    #[cfg(test)]
    fn clear(&mut self) {
        use zeroize::Zeroize as _;
        self.bytes.zeroize();
    }
}

impl fmt::Debug for SessionPassword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SessionPassword(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_should_not_show_the_password() {
        assert!(!format!("{:?}", SessionPassword::new("hunter2")).contains("hunter2"));
    }

    #[test]
    fn a_cleared_password_should_hold_only_zeros() {
        let mut password = SessionPassword::new("hunter2");
        password.clear();
        assert!(password.peek().iter().all(|&b| b == 0));
    }

    #[test]
    fn the_message_password_should_be_the_same_text() {
        assert_eq!(SessionPassword::new("pässwörd").to_password().expose(), "pässwörd");
    }

    #[test]
    fn an_empty_password_should_need_no_lock() {
        assert_eq!(SessionPassword::new("").to_password().expose(), "");
    }
}
