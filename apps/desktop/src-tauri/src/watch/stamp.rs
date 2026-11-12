//! What a watched file looks like on disk ([`FileStamp`]), and when a change to it has settled
//! ([`Settle`]): the watcher's decisions, apart from the file system so they can be tested.

use std::fs::File;
use std::hash::{DefaultHasher, Hasher as _};
use std::io::{self, Read as _, Seek as _, SeekFrom};
use std::path::Path;
use std::time::SystemTime;

/// How much of each end of the file [`FileStamp::sample`] covers.
const SAMPLE: u64 = 64 * 1024;

/// A file's size, modification time, identity and a hash of its first and last 64 KiB: enough to
/// tell a real change from an event that changed nothing (a touch of the folder, another file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileStamp {
    pub len: u64,
    pub modified: Option<SystemTime>,
    /// The inode, or Windows' file index: a file replaced by a rename has a new one.
    pub id: Option<file_id::FileId>,
    pub sample: u64,
}

impl FileStamp {
    /// `path`'s stamp now.
    ///
    /// # Errors
    /// The OS's error when the file can't be opened or read: missing, or not allowed.
    pub(crate) fn read(path: &Path) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let meta = file.metadata()?;
        let len = meta.len();
        let mut hasher = DefaultHasher::new();
        hasher.write_u64(len);
        let mut head = Vec::new();
        (&mut file).take(SAMPLE).read_to_end(&mut head)?;
        hasher.write(&head);
        if len > SAMPLE {
            file.seek(SeekFrom::Start(len.saturating_sub(SAMPLE).max(SAMPLE)))?;
            let mut tail = Vec::new();
            file.take(SAMPLE).read_to_end(&mut tail)?;
            hasher.write(&tail);
        }
        Ok(Self {
            len,
            modified: meta.modified().ok(),
            id: file_id::get_file_id(path).ok(),
            sample: hasher.finish(),
        })
    }
}

/// What became of a watched file since its document opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Change {
    /// Nothing that matters.
    Unchanged,
    /// Different contents at the same path: written in place, or replaced by a rename.
    Modified(FileStamp),
    /// Nothing at the path: deleted or moved away.
    Missing,
}

/// `now` against `opened`: `Missing` when nothing is at the path, `Unchanged` when every part of
/// the stamp is the same, else `Modified`.
pub(crate) fn classify(opened: &FileStamp, now: Option<&FileStamp>) -> Change {
    match now {
        None => Change::Missing,
        Some(now) if now == opened => Change::Unchanged,
        Some(now) => Change::Modified(now.clone()),
    }
}

/// What to do after a look at the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// Look again one debounce period later: the file may still be being written.
    Wait,
    /// Settled: report it.
    Settled(Change),
}

/// A change waiting to settle. The first look comes a debounce period after the last event, and a
/// change counts only once a second look, one period later, finds the same: the same new stamp,
/// so a build writing in chunks is never read half-written, or nothing again, so a file deleted
/// and written back a little late (a slow editor's save) reloads instead of showing as missing.
/// An event between the looks puts the second one off by a period.
#[derive(Debug, Default)]
pub(crate) struct Settle {
    /// The last look's change, when it was one that must be seen twice.
    last: Option<Change>,
}

impl Settle {
    /// One look at the file (`now`, `None` when nothing is at the path) against `opened`.
    pub(crate) fn look(&mut self, opened: &FileStamp, now: Option<&FileStamp>) -> Verdict {
        let change = classify(opened, now);
        if change == Change::Unchanged {
            self.last = None;
            return Verdict::Settled(change);
        }
        if self.last.as_ref() == Some(&change) {
            self.last = None;
            Verdict::Settled(change)
        } else {
            self.last = Some(change);
            Verdict::Wait
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn stamp(len: u64, sample: u64) -> FileStamp {
        FileStamp {
            len,
            modified: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(len)),
            id: None,
            sample,
        }
    }

    #[test]
    fn an_identical_stamp_should_be_unchanged() {
        assert_eq!(classify(&stamp(10, 1), Some(&stamp(10, 1))), Change::Unchanged);
    }

    #[test]
    fn nothing_at_the_path_should_be_missing() {
        assert_eq!(classify(&stamp(10, 1), None), Change::Missing);
    }

    #[test]
    fn same_size_but_different_bytes_should_be_modified() {
        assert_eq!(
            classify(&stamp(10, 1), Some(&stamp(10, 2))),
            Change::Modified(stamp(10, 2))
        );
    }

    #[test]
    fn a_new_identity_at_the_same_path_should_be_modified() {
        let mut replaced = stamp(10, 1);
        replaced.id = file_id::get_file_id(Path::new(".")).ok();
        assert!(matches!(classify(&stamp(10, 1), Some(&replaced)), Change::Modified(_)));
    }

    #[test]
    fn a_change_should_settle_once_two_looks_agree() {
        let mut settle = Settle::default();
        let opened = stamp(10, 1);
        let looks = [
            settle.look(&opened, Some(&stamp(20, 2))),
            settle.look(&opened, Some(&stamp(30, 3))),
            settle.look(&opened, Some(&stamp(30, 3))),
        ];
        assert_eq!(
            looks,
            [
                Verdict::Wait,
                Verdict::Wait,
                Verdict::Settled(Change::Modified(stamp(30, 3)))
            ]
        );
    }

    #[test]
    fn a_deletion_should_settle_once_two_looks_find_nothing() {
        let mut settle = Settle::default();
        let opened = stamp(10, 1);
        let looks = [settle.look(&opened, None), settle.look(&opened, None)];
        assert_eq!(looks, [Verdict::Wait, Verdict::Settled(Change::Missing)]);
    }

    #[test]
    fn a_file_written_back_after_a_deletion_should_settle_modified() {
        let mut settle = Settle::default();
        let opened = stamp(10, 1);
        let looks = [
            settle.look(&opened, None),
            settle.look(&opened, Some(&stamp(20, 2))),
            settle.look(&opened, Some(&stamp(20, 2))),
        ];
        assert_eq!(
            looks,
            [
                Verdict::Wait,
                Verdict::Wait,
                Verdict::Settled(Change::Modified(stamp(20, 2)))
            ]
        );
    }

    #[test]
    fn a_change_undone_should_settle_unchanged() {
        let mut settle = Settle::default();
        let opened = stamp(10, 1);
        settle.look(&opened, Some(&stamp(20, 2)));
        assert_eq!(
            settle.look(&opened, Some(&stamp(10, 1))),
            Verdict::Settled(Change::Unchanged)
        );
    }

    #[test]
    fn a_file_rewritten_in_place_should_stamp_differently() {
        let path = std::env::temp_dir().join(format!("laminapdf-stamp-{}", std::process::id()));
        std::fs::write(&path, vec![1_u8; 200_000]).unwrap();
        let before = FileStamp::read(&path).unwrap();
        let mut bytes = vec![1_u8; 200_000];
        bytes[199_999] = 2;
        std::fs::write(&path, bytes).unwrap();
        let after = FileStamp::read(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_ne!(before.sample, after.sample);
    }
}
