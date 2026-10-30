//! The records of the document API (ADR 0028 F5, F6): what JavaScript passes in (`…Init`,
//! every field optional, unknown keys refused) and what it gets back. Each derives `Tsify`,
//! so the generated `.d.ts` declares it; names are camelCase, string unions camelCase too
//! (as the op JSON's `"op": "setInfo"`, ADR 0016 §7).

use lamina::{IncrementalOutput, Limits, OpenOptions, Renumber, SaveMode, SaveOptions, SaveReport, XrefKind};
use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::error::{JsResult, invalid_argument};

/// Options for `WasmDocument.openBytes`.
#[derive(Debug, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenOptionsInit {
    /// The user or owner password of an encrypted document.
    #[tsify(optional)]
    pub password: Option<String>,
    /// Refuse files that need repair (`strict`).
    #[tsify(optional)]
    pub strict: Option<bool>,
    /// Resource limits (ADR 0003 §3); each one left out keeps its default.
    #[tsify(optional)]
    pub limits: Option<LimitsInit>,
}

/// Declares [`LimitsInit`] and its conversion from one list of `(field, setter, type)`.
macro_rules! limits_init {
    ($( $field:ident: $ty:ty => $with:ident; )*) => {
        /// `Limits` with every field optional (ADR 0003 §3; names as in Rust, camelCase).
        #[derive(Debug, Default, Deserialize, Tsify)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        pub struct LimitsInit {
            $(
                #[doc = concat!("`", stringify!($field), "`.")]
                #[tsify(optional)]
                pub $field: Option<u64>,
            )*
        }

        impl LimitsInit {
            fn to_limits(&self) -> JsResult<Limits> {
                let mut limits = Limits::new();
                $(
                    if let Some(value) = self.$field {
                        let value = <$ty>::try_from(value).map_err(|_| {
                            invalid_argument("options.limits", format!("{} is too large", stringify!($field)), Vec::new())
                        })?;
                        limits = limits.$with(value);
                    }
                )*
                Ok(limits)
            }
        }
    };
}

limits_init! {
    max_nesting_depth: usize => with_max_nesting_depth;
    max_stream_decoded: usize => with_max_stream_decoded;
    max_decoded_total: usize => with_max_decoded_total;
    max_filter_chain: usize => with_max_filter_chain;
    max_resolve_depth: usize => with_max_resolve_depth;
    max_page_tree_depth: usize => with_max_page_tree_depth;
    max_tree_depth: usize => with_max_tree_depth;
    max_string_length: usize => with_max_string_length;
    max_xref_sections: usize => with_max_xref_sections;
    max_object_number: u32 => with_max_object_number;
    max_warnings: usize => with_max_warnings;
    max_content_operators: u64 => with_max_content_operators;
    max_xobject_depth: usize => with_max_xobject_depth;
    max_image_pixels: u64 => with_max_image_pixels;
    max_read_window: usize => with_max_read_window;
}

impl OpenOptionsInit {
    /// The façade's options.
    pub(crate) fn to_options(&self) -> JsResult<OpenOptions> {
        let mut options = OpenOptions::default().with_strict(self.strict.unwrap_or(false));
        if let Some(limits) = &self.limits {
            options = options.with_limits(limits.to_limits()?);
        }
        if let Some(password) = &self.password {
            options = options.with_password(password.as_bytes());
        }
        Ok(options)
    }
}

/// The save mode (ADR 0007 §3).
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub enum SaveModeJs {
    /// Incremental when allowed, else a full rewrite (the default).
    Auto,
    /// Append a revision.
    Incremental,
    /// Rewrite the whole file.
    FullRewrite,
}

/// What an incremental save returns.
#[derive(Debug, Clone, Copy, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub enum OutputJs {
    /// The whole file (the default).
    Whole,
    /// Only the appended revision (F7).
    AppendedOnly,
}

/// Object numbering in a full rewrite.
#[derive(Debug, Clone, Copy, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub enum RenumberJs {
    /// Keep the numbers (the default).
    Keep,
    /// Number objects densely from 1.
    Compact,
}

/// The cross-reference form.
#[derive(Debug, Clone, Copy, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub enum XrefJs {
    /// Cross-reference and object streams (the default).
    Stream,
    /// A classic `xref` table.
    Classic,
}

/// Options for saving.
#[derive(Debug, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveOptionsInit {
    /// The mode (default `auto`).
    #[tsify(optional)]
    pub mode: Option<SaveModeJs>,
    /// Save even when it invalidates signatures (removing them from the output).
    #[tsify(optional)]
    pub invalidate_signatures: Option<bool>,
    /// Write an encrypted document as a plaintext full rewrite.
    #[tsify(optional)]
    pub remove_encryption: Option<bool>,
    /// What an incremental save returns.
    #[tsify(optional)]
    pub output: Option<OutputJs>,
    /// Numbering in a full rewrite.
    #[tsify(optional)]
    pub renumber: Option<RenumberJs>,
    /// The cross-reference form.
    #[tsify(optional)]
    pub xref: Option<XrefJs>,
}

impl SaveOptionsInit {
    /// The façade's options.
    pub(crate) fn to_options(&self) -> SaveOptions {
        let mut options = SaveOptions::new();
        if let Some(mode) = self.mode {
            options = options.with_mode(match mode {
                SaveModeJs::Auto => SaveMode::Auto,
                SaveModeJs::Incremental => SaveMode::Incremental,
                SaveModeJs::FullRewrite => SaveMode::FullRewrite,
            });
        }
        if let Some(on) = self.invalidate_signatures {
            options = options.with_invalidate_signatures(on);
        }
        if let Some(on) = self.remove_encryption {
            options = options.with_remove_encryption(on);
        }
        if let Some(output) = self.output {
            options = options.with_output(match output {
                OutputJs::Whole => IncrementalOutput::Whole,
                OutputJs::AppendedOnly => IncrementalOutput::AppendedOnly,
            });
        }
        if let Some(renumber) = self.renumber {
            options = options.with_renumber(match renumber {
                RenumberJs::Keep => Renumber::Keep,
                RenumberJs::Compact => Renumber::Compact,
            });
        }
        if let Some(xref) = self.xref {
            options = options.with_xref(match xref {
                XrefJs::Stream => XrefKind::Stream,
                XrefJs::Classic => XrefKind::Classic,
            });
        }
        options
    }
}

/// One page's facts (`lamina::PageInfo`).
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct PageInfoJs {
    /// The 0-based index.
    pub index: u32,
    /// The stable page ID (`p12.0`).
    pub id: String,
    /// Displayed width in points.
    pub width: f64,
    /// Displayed height in points.
    pub height: f64,
    /// `/Rotate` in degrees.
    pub rotation: u16,
    /// The page label, when the document defines one (document text).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub label: Option<String>,
}

impl From<lamina::PageInfo> for PageInfoJs {
    fn from(p: lamina::PageInfo) -> Self {
        Self {
            index: p.index,
            id: p.id.to_string(),
            width: p.width,
            height: p.height,
            rotation: p.rotation,
            label: p.label,
        }
    }
}

/// The Info dictionary's text entries; absent ones are left out (document text).
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct DocumentInfoJs {
    /// `/Title`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub title: Option<String>,
    /// `/Author`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub author: Option<String>,
    /// `/Subject`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub subject: Option<String>,
    /// `/Keywords`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub keywords: Option<String>,
    /// `/Creator`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub creator: Option<String>,
    /// `/Producer`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub producer: Option<String>,
}

impl From<lamina::DocumentInfo> for DocumentInfoJs {
    fn from(i: lamina::DocumentInfo) -> Self {
        Self {
            title: i.title,
            author: i.author,
            subject: i.subject,
            keywords: i.keywords,
            creator: i.creator,
            producer: i.producer,
        }
    }
}

/// The pages an edit touched.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct EditedJs {
    /// IDs of the pages whose content or attributes changed.
    pub touched_pages: Vec<String>,
}

impl From<lamina::Edited> for EditedJs {
    fn from(e: lamina::Edited) -> Self {
        Self {
            touched_pages: e.touched_pages.iter().map(ToString::to_string).collect(),
        }
    }
}

/// What a save wrote (`lamina::SaveReport`).
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct SaveReportJs {
    /// Bytes written.
    pub bytes_written: u64,
    /// The mode used (`incremental` or `fullRewrite`).
    pub mode: SaveModeJs,
    /// Old → new object numbers, when a full rewrite renumbered.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub renumbered: Option<std::collections::BTreeMap<u32, u32>>,
    /// Where the appended revision starts, for an incremental save.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub append_offset: Option<u64>,
    /// How many signatures the save invalidated (with `invalidateSignatures`).
    pub invalidated_signatures: u32,
    /// Whether encryption was removed.
    pub encryption_removed: bool,
}

impl From<&SaveReport> for SaveReportJs {
    fn from(r: &SaveReport) -> Self {
        Self {
            bytes_written: r.bytes_written(),
            mode: if r.mode() == SaveMode::Incremental {
                SaveModeJs::Incremental
            } else {
                SaveModeJs::FullRewrite
            },
            renumbered: r.renumbered().cloned(),
            append_offset: r.append_offset(),
            invalidated_signatures: u32::try_from(r.invalidated().len()).unwrap_or(u32::MAX),
            encryption_removed: r.encryption_removed(),
        }
    }
}

/// How an undo or redo went (`lamina::UndoOutcome`).
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub enum UndoOutcomeJs {
    /// Everything was restored.
    Applied,
    /// Some properties were left alone (a later change replaced them, or they are gone).
    Partial,
    /// Nothing could be restored.
    Skipped,
}

impl From<&lamina::UndoOutcome> for UndoOutcomeJs {
    fn from(o: &lamina::UndoOutcome) -> Self {
        match o {
            lamina::UndoOutcome::Applied => Self::Applied,
            lamina::UndoOutcome::Partial { .. } => Self::Partial,
            _ => Self::Skipped,
        }
    }
}

/// A job's progress (`lamina::Step::Progress`).
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct ProgressJs {
    /// Work done so far (units of the phase).
    pub done: u32,
    /// The total, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub total: Option<u32>,
    /// The phase (`parse`, `layout`, `extract`, `render`, `write`, `sign`).
    pub phase: &'static str,
}

/// `lamina::Phase` as a camelCase string.
pub(crate) fn phase_name(phase: lamina::Phase) -> &'static str {
    match phase {
        lamina::Phase::Parse => "parse",
        lamina::Phase::Layout => "layout",
        lamina::Phase::Extract => "extract",
        lamina::Phase::Render => "render",
        lamina::Phase::Write => "write",
        lamina::Phase::Sign => "sign",
        // A phase added to the non-exhaustive enum before this list learns it.
        _ => "other",
    }
}
