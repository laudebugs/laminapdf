//! The save fuzz targets' entry points (ADR 0018 tier 9; todo B3c), one function per target.
//! `fuzz/fuzz_targets/{ops_json,save_roundtrip}.rs` call these through `fuzz/src/lib.rs`,
//! which includes this file, and `tests/regressions.rs` replays every saved crash input
//! (`fixtures/regressions/<target>/`) through the same function.

use lamina_cos::OpenOptions;
use lamina_model::Rotation;
use lamina_ops::{Document, InfoPatch, OpLog, Origin, RotatePage, SetInfo};
use lamina_save::{Renumber, Save, SaveMode, SaveOptions, XrefKind};

/// A fuzz target's entry point.
pub type Entry = fn(&[u8]);

/// The targets, by name.
pub const TARGETS: [(&str, Entry); 2] = [("ops_json", ops_json), ("save_roundtrip", save_roundtrip)];

/// The files `ops_json` applies its op log to: the pages and Info the samples in
/// `fixtures/ops/v1/` name (`p3.0` in `hello.pdf`, `p4.0` in `outline.pdf`).
const OPS_FIXTURES: [&[u8]; 2] = [
    include_bytes!("../../../../fixtures/corpus/hello.pdf"),
    include_bytes!("../../../../fixtures/corpus/outline.pdf"),
];

/// The control bytes `save_roundtrip` reads before the PDF (see [`save_roundtrip`]).
pub const SAVE_CONTROL_LEN: usize = 8;

/// The control bytes every `save_roundtrip` seed starts with: `Auto`, then a rotation, a
/// title, an undo, a redo and three more rotations. Must match `SAVE_SEED_CONTROL` in
/// `tools/xtask/src/harness.rs`.
pub const SAVE_SEED_CONTROL: [u8; SAVE_CONTROL_LEN] = [0, 0x14, 0x41, 0x02, 0x03, 0x24, 0x34, 0x04];

/// Reads `data` as an op log (`{ "v": 1, "ops": [ … ] }`) and applies it to each fixture:
/// as one transaction, then saved (`Auto`) and reopened; and op by op, all undone, after which
/// a save must write the fixture's bytes unchanged.
///
/// # Panics
/// When a saved file does not reopen, or undoing every op leaves something to save.
pub fn ops_json(data: &[u8]) {
    let Ok(json) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(log) = OpLog::from_json(json) else {
        return;
    };
    for base in OPS_FIXTURES {
        let mut doc = open(base);
        if doc.apply_all(Origin::Local, log.ops().iter().cloned()).is_ok() {
            let mut out = Vec::new();
            if doc.save(&mut out, &SaveOptions::new()).is_ok() {
                assert!(
                    lamina_cos::Document::open(out, OpenOptions::default()).is_ok(),
                    "a saved file does not reopen"
                );
            }
        }
        let mut doc = open(base);
        let applied = log
            .ops()
            .iter()
            .filter(|op| doc.apply_all(Origin::Local, [(*op).clone()]).is_ok())
            .count();
        for _ in 0..applied {
            let _ = doc.undo();
        }
        let mut out = Vec::new();
        doc.save(&mut out, &SaveOptions::new().with_mode(SaveMode::Incremental))
            .unwrap_or_else(|e| panic!("saving after undoing everything: {e}"));
        assert!(out == base, "undoing every op left something to save");
    }
}

/// Opens the PDF after the first [`SAVE_CONTROL_LEN`] bytes, applies the ops the control
/// bytes name, saves, and checks that the output reopens (and that the document rebases onto
/// it).
///
/// Control byte 0: bits 0–1 the mode (`Auto`, `Incremental`, `FullRewrite`, `Auto`), bit 2
/// `invalidate_signatures`, bit 3 `remove_encryption`, bit 4 classic xref, bit 5 compact
/// numbering. Bytes 1–7, one step each, by the low two bits: 0 rotates page `(b >> 4)` (modulo
/// the page count) by `(b >> 2) & 3` quarter turns, 1 sets the title to `b` as a number, 2
/// undoes, 3 redoes.
///
/// # Panics
/// When a saved file does not reopen, or the rebase after a full rewrite fails.
pub fn save_roundtrip(data: &[u8]) {
    let Some((control, pdf)) = data.split_first_chunk::<SAVE_CONTROL_LEN>() else {
        return;
    };
    let Ok(mut doc) = Document::open(pdf.to_vec(), OpenOptions::default()) else {
        return;
    };
    let [flags, steps @ ..] = *control;
    for step in steps {
        let _ = match step & 3 {
            0 => rotate(&mut doc, usize::from(step >> 4), step >> 2),
            1 => doc
                .apply_all(
                    Origin::Local,
                    [SetInfo::new(InfoPatch::new().title(Some(&step.to_string())), None).into()],
                )
                .map(drop),
            2 => doc.undo().map(drop),
            _ => doc.redo().map(drop),
        };
    }
    let mode = match flags & 3 {
        1 => SaveMode::Incremental,
        2 => SaveMode::FullRewrite,
        _ => SaveMode::Auto,
    };
    let options = SaveOptions::new()
        .with_mode(mode)
        .with_invalidate_signatures(flags & 4 != 0)
        .with_remove_encryption(flags & 8 != 0)
        .with_xref(if flags & 16 == 0 {
            XrefKind::Stream
        } else {
            XrefKind::Classic
        })
        .with_renumber(if flags & 32 == 0 {
            Renumber::Keep
        } else {
            Renumber::Compact
        });
    let mut out = Vec::new();
    if doc.save(&mut out, &options).is_err() {
        return;
    }
    assert!(
        lamina_cos::Document::open(out.clone(), OpenOptions::default()).is_ok(),
        "a saved file does not reopen"
    );
    if doc.save_state() == lamina_ops::SaveState::SavedNotRebased {
        doc.rebase(out)
            .unwrap_or_else(|e| panic!("rebasing onto the saved file: {e}"));
    }
}

fn open(bytes: &[u8]) -> Document {
    Document::open(bytes.to_vec(), OpenOptions::default()).unwrap_or_else(|e| panic!("fixture: {e}"))
}

fn rotate(doc: &mut Document, index: usize, quarters: u8) -> lamina_ops::Result<()> {
    let pages = doc.pages()?;
    let Some(page) = pages.get(index % pages.len().max(1)).map(|p| p.id().clone()) else {
        return Ok(());
    };
    let rotation = match quarters & 3 {
        0 => Rotation::None,
        1 => Rotation::Clockwise90,
        2 => Rotation::Half,
        _ => Rotation::Clockwise270,
    };
    doc.apply_all(Origin::Local, [RotatePage::new(page, rotation).into()])
        .map(drop)
}
