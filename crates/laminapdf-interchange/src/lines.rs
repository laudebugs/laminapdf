//! Where an offset is in a file, as a line number for messages: the line starts found once, then
//! each offset's line by binary search, however many are asked for.

use crate::InterchangeError;

/// A file's line starts.
pub(crate) struct Lines {
    /// The format's name, for errors.
    format: &'static str,
    /// The offset after each `\n`.
    starts: Vec<usize>,
}

impl Lines {
    pub(crate) fn new(bytes: &[u8], format: &'static str) -> Self {
        let starts = bytes
            .iter()
            .enumerate()
            .filter(|&(_, b)| *b == b'\n')
            .map(|(i, _)| i + 1)
            .collect();
        Self { format, starts }
    }

    /// The line of byte `offset`, counted from 1.
    pub(crate) fn at(&self, offset: usize) -> u32 {
        u32::try_from(self.starts.partition_point(|&s| s <= offset) + 1).unwrap_or(u32::MAX)
    }

    /// The malformed-file error at byte `offset`.
    pub(crate) fn malformed(&self, offset: usize, reason: &'static str) -> InterchangeError {
        InterchangeError::Malformed {
            format: self.format,
            line: self.at(offset),
            reason,
        }
    }
}
