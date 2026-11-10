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
use zeroize::{Zeroize as _, Zeroizing};

/// Set once a lock has failed, so the warning shows once per run.
static LOCK_WARNED: AtomicBool = AtomicBool::new(false);

/// The bytes one locked buffer holds: 16 KiB, a whole page on Apple silicon and four on x86 and
/// most arm64 Linux. On a kernel with 64 KiB pages two passwords can share a page, so closing one
/// document can unlock another's password; a smaller buffer keeps more passwords inside Linux's
/// default lock limit (64 KiB before 5.16). A PDF password is at most 127 bytes after SASLprep;
/// a longer one is kept on the ordinary heap, unlocked.
const PAGE: usize = 16 * 1024;

/// A buffer aligned to, and as long as, a 16 KiB page, so no other allocation shares its pages
/// (on 4 KiB and 16 KiB pages): unlocking one password then never unlocks another's bytes.
#[repr(C, align(16384))]
struct Page([u8; PAGE]);

/// Where the bytes live.
enum Store {
    Locked(Box<Page>),
    Heap(Zeroizing<Box<[u8]>>),
}

/// A document's password, wiped when dropped.
pub(crate) struct SessionPassword {
    // Fields drop in order, after `Drop::drop` has wiped the bytes: the pages are unlocked
    // before the buffer is freed.
    _lock: Option<region::LockGuard>,
    store: Store,
    len: usize,
}

impl SessionPassword {
    /// Copies `password` into a buffer of its own, locked into RAM where the OS allows.
    pub(crate) fn new(password: &str) -> Self {
        let bytes = password.as_bytes();
        let len = bytes.len();
        let Some(dst) = (len <= PAGE).then(|| Box::new(Page([0; PAGE]))) else {
            return Self {
                _lock: None,
                store: Store::Heap(Zeroizing::new(bytes.into())),
                len,
            };
        };
        let mut page = dst;
        if let Some(head) = page.0.get_mut(..len) {
            head.copy_from_slice(bytes);
        }
        let lock = match region::lock(page.0.as_ptr(), PAGE) {
            Ok(guard) => Some(guard),
            Err(error) => {
                if !LOCK_WARNED.swap(true, Ordering::Relaxed) {
                    tracing::warn!("could not lock a password into memory ({error}); it may reach swap");
                }
                None
            }
        };
        Self {
            _lock: lock,
            store: Store::Locked(page),
            len,
        }
    }

    fn bytes(&self) -> &[u8] {
        match &self.store {
            Store::Locked(page) => page.0.get(..self.len).unwrap_or_default(),
            Store::Heap(bytes) => bytes,
        }
    }

    /// The password for one `UnlockDocument` message (wiped when that message is dropped).
    pub(crate) fn to_password(&self) -> Password {
        Password::new(String::from_utf8_lossy(self.bytes()).into_owned())
    }

    /// The whole buffer, for tests to see that a wiped password holds only zeros.
    #[cfg(test)]
    fn peek(&self) -> &[u8] {
        match &self.store {
            Store::Locked(page) => &page.0,
            Store::Heap(bytes) => bytes,
        }
    }

    /// Wipes the bytes (dropping does the same).
    fn wipe(&mut self) {
        match &mut self.store {
            Store::Locked(page) => page.0.zeroize(),
            Store::Heap(bytes) => bytes.zeroize(),
        }
    }
}

impl Drop for SessionPassword {
    fn drop(&mut self) {
        self.wipe();
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
    fn a_wiped_password_should_hold_only_zeros() {
        let mut password = SessionPassword::new("hunter2");
        password.wipe();
        assert!(password.peek().iter().all(|&b| b == 0));
    }

    #[test]
    fn each_password_should_have_its_own_pages() {
        let (a, b) = (SessionPassword::new("a"), SessionPassword::new("b"));
        let (a, b) = (a.peek().as_ptr() as usize, b.peek().as_ptr() as usize);
        assert!(a % PAGE == 0 && b % PAGE == 0 && a.abs_diff(b) >= PAGE);
    }

    #[test]
    fn a_password_longer_than_a_page_should_still_round_trip() {
        let long = "x".repeat(PAGE + 1);
        let password = SessionPassword::new(&long);
        assert_eq!(password.to_password().expose(), long);
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
