//! What a panic leaves for a crash report (D7a): its message, where it happened and the
//! panicking thread's frames, captured in the process that panicked. The engine sends this to the
//! main process in [`crate::FromEngine::Panic`]; the main process captures its own. Neither side
//! sends it anywhere else: the main process builds the report from it, scrubbed, and only with the
//! user's consent.
//!
//! The frames hold function names and offsets, never a source path or a variable's value, so
//! they can't carry document content. The message can (a `format!` with a path), which is why the
//! main process scrubs it before it shows or sends a report.

use std::panic::PanicHookInfo;

use serde::{Deserialize, Serialize};

/// The most frames a summary keeps.
pub const MAX_FRAMES: usize = 64;

/// The most bytes of a panic message a summary keeps.
pub const MAX_MESSAGE: usize = 1024;

/// A panic, as captured where it happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanicSummary {
    /// The panic's message, cut to [`MAX_MESSAGE`] bytes. Unscrubbed.
    pub message: String,
    /// Where the panic was raised, as the compiler recorded it.
    pub location: Option<SourceLocation>,
    /// The panicking thread's frames, innermost first, without the panic machinery's own.
    pub frames: Vec<NativeFrame>,
}

/// A source position: a file as the compiler named it, a line and a column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    /// The file, relative to the workspace for lamina's own crates, absolute for others.
    pub file: String,
    /// The line, from 1.
    pub line: u32,
    /// The column, from 1.
    pub column: u32,
}

/// One frame of a native stack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeFrame {
    /// The binary the function is in.
    pub module: String,
    /// The instruction's offset within `function`.
    pub offset: u64,
    /// The function, demangled, without its hash.
    pub function: String,
}

/// The summary of the panic `info` describes, raised in the binary `module`. Call it from a panic
/// hook: it walks the current thread's stack.
#[must_use]
pub fn summarize(info: &PanicHookInfo<'_>, module: &str) -> PanicSummary {
    let message = info
        .payload()
        .downcast_ref::<&str>()
        .map(|text| (*text).to_owned())
        .or_else(|| info.payload().downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "a panic with no message".to_owned());
    PanicSummary {
        message: cut(message, MAX_MESSAGE),
        location: info.location().map(|at| SourceLocation {
            file: at.file().to_owned(),
            line: at.line(),
            column: at.column(),
        }),
        frames: frames(module),
    }
}

/// The current thread's named frames, from the first that isn't the panic machinery's.
fn frames(module: &str) -> Vec<NativeFrame> {
    let trace = backtrace::Backtrace::new();
    let named = trace.frames().iter().filter_map(|frame| {
        // The innermost symbol: with inlining, a frame can hold several.
        let symbol = frame.symbols().first()?;
        let function = format!("{:#}", symbol.name()?);
        let start = symbol.addr()? as usize;
        let offset = (frame.ip() as usize).checked_sub(start)?;
        Some(NativeFrame {
            module: module.to_owned(),
            offset: offset as u64,
            function,
        })
    });
    named
        .skip_while(|frame| is_machinery(&frame.function))
        .take(MAX_FRAMES)
        .collect()
}

/// Whether `function` is part of raising or reporting a panic rather than the code that panicked.
fn is_machinery(function: &str) -> bool {
    const PREFIXES: [&str; 7] = [
        "backtrace::",
        "std::",
        "core::",
        "alloc::",
        "rust_begin_unwind",
        "__rust",
        "laminapdf_desktop_protocol::crash::",
    ];
    PREFIXES.iter().any(|prefix| function.starts_with(prefix)) || function.contains("panic")
}

/// `text` cut to at most `max` bytes, on a character boundary.
#[must_use]
pub fn cut(mut text: String, max: usize) -> String {
    if text.len() > max {
        let end = (0..=max).rev().find(|&i| text.is_char_boundary(i)).unwrap_or(0);
        text.truncate(end);
    }
    text
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex, PoisonError};

    use super::*;

    #[test]
    fn a_long_message_should_be_cut_on_a_character_boundary() {
        assert_eq!(cut("aé".repeat(3), 4), "aéa");
    }

    #[test]
    fn machinery_frames_should_be_told_from_the_codes_own() {
        let checks = [
            is_machinery("std::panicking::begin_panic_handler"),
            is_machinery("core::panicking::panic_fmt"),
            is_machinery("laminapdf_desktop_engine::worker::render"),
        ];
        assert_eq!(checks, [true, true, false]);
    }

    #[test]
    fn a_panic_should_be_summarised_with_its_message_location_and_frames() {
        let seen: Arc<Mutex<Option<PanicSummary>>> = Arc::default();
        let into = Arc::clone(&seen);
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            *into.lock().unwrap_or_else(PoisonError::into_inner) = Some(summarize(info, "tests"));
        }));
        let _ = std::panic::catch_unwind(|| panic!("index {} out of range", 7));
        std::panic::set_hook(previous);
        let summary = seen.lock().unwrap_or_else(PoisonError::into_inner).take().unwrap();
        assert_eq!(
            (
                summary.message.as_str(),
                summary.location.map(|at| at.file),
                summary.frames.first().map(|frame| frame.module.clone()),
            ),
            (
                "index 7 out of range",
                Some("crates/laminapdf-desktop-protocol/src/crash.rs".to_owned()),
                Some("tests".to_owned()),
            )
        );
    }
}
