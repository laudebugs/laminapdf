//! The `FileAttachment` spec (ISO 32000-2 §12.5.6.15): an icon on the page that holds a file.
//! The file specification is the one attachments use ([`laminapdf_model::write_embedded_file`]), so the
//! Attachments tab and [`DeleteAttachment`](crate::DeleteAttachment) see the annotation as one
//! more holder of the file.

use laminapdf_cos::Dict;
use laminapdf_model::{AfRelationship, EmbeddedFileSpec, PdfDate, UserSpacePoint, UserSpaceRect, write_embedded_file};
use serde::{Deserialize, Serialize};

use super::geom;
use crate::blob::{BlobId, BlobStore};
use crate::error::{Error, Result};
use crate::wire;

/// A file attached at a point on the page: an icon whose upper-left corner is `at`, 20 points a
/// side, holding the file's bytes. The icon keeps its size and stays upright however the page is
/// zoomed or turned (`NoZoom`, `NoRotate`).
///
/// On a file that claims PDF/A-3, the file specification also goes in the annotation's `/AF`,
/// as ISO 19005-3 §6.8 asks; the save plan warns `pdfa-attachment-incomplete` without a
/// `modified` date or a MIME type, and `pdfa-attachment-forbidden` on PDF/A-1 and PDF/A-2.
///
/// On the wire: `{ "type": "fileAttachment", "at": { "x": 72, "y": 720 }, "icon": "paperclip",
/// "file": { "file": "<64 hex digits>", "name": "data.csv", "mime": "text/csv" } }`.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct FileAttachment {
    /// The icon's upper-left corner, in user space.
    #[serde(with = "wire::user_space_point")]
    #[cfg_attr(feature = "tsify", tsify(type = "{ x: number; y: number }"))]
    pub at: UserSpacePoint,
    /// The icon.
    #[serde(default)]
    pub icon: AttachmentIcon,
    /// The file.
    pub file: FileToAttach,
}

impl FileAttachment {
    /// The side of the icon, in points.
    ///
    /// Added in 0.6.0.
    pub const ICON: f64 = 20.0;

    /// The file `file` attached with `icon` at `at`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn new(at: UserSpacePoint, icon: AttachmentIcon, file: FileToAttach) -> Self {
        Self { at, icon, file }
    }

    pub(crate) fn is_valid(&self) -> bool {
        geom::finite([self.at]) && !self.file.name.is_empty()
    }

    /// The icon of an annotation `dict` describes, at its `/Rect`'s upper-left corner, with a
    /// file that stands for the one it holds: only the drawing is read back, and redrawing
    /// leaves `/FS` alone.
    pub(crate) fn read(dict: &Dict, rect: Option<UserSpaceRect>) -> Option<Self> {
        let rect = rect?;
        let icon = dict
            .get_name("Name")
            .map_or(AttachmentIcon::PushPin, AttachmentIcon::from_name);
        Some(Self::new(
            UserSpacePoint::new(rect.x0, rect.y1),
            icon,
            FileToAttach::new("file", BlobId::from_bytes([0; 32]), ""),
        ))
    }

    /// Writes the embedded file and its specification into the overlay; returns the
    /// specification's object, for `/FS` (and `/AF`).
    pub(crate) fn write_file(
        &self,
        doc: &mut laminapdf_model::Document,
        blobs: &BlobStore,
    ) -> Result<laminapdf_cos::ObjRef> {
        let f = &self.file;
        let bytes = blobs.get(&f.file).ok_or(Error::UnknownBlob(f.file))?;
        let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        let limit = u64::try_from(doc.limits().max_stream_decoded()).unwrap_or(u64::MAX);
        if size > limit {
            return Err(Error::AttachmentTooLarge { size, limit });
        }
        let mut spec = EmbeddedFileSpec::new(&f.name, bytes, &f.mime);
        spec.description = f.description.as_deref();
        spec.created = f.created.as_ref();
        spec.modified = f.modified.as_ref();
        spec.relationship = AfRelationship::Unspecified;
        Ok(write_embedded_file(&mut doc.cos_mut().edit(), &spec)?)
    }
}

/// The file a [`FileAttachment`] holds: its bytes by their ID in the blob store
/// ([`Document::add_blob`](crate::Document::add_blob)) and the facts its file specification
/// records. The dates come in the op, never from a clock.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct FileToAttach {
    /// The bytes.
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub file: BlobId,
    /// The file's name, not empty.
    pub name: String,
    /// Its MIME type, such as `text/csv`; empty for none.
    #[serde(default)]
    pub mime: String,
    /// `/Desc`, which viewers show beside the file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub description: Option<String>,
    /// When the file was made.
    #[serde(default, with = "wire::opt_date", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string"))]
    pub created: Option<PdfDate>,
    /// When it last changed, which PDF/A-3 needs.
    #[serde(default, with = "wire::opt_date", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string"))]
    pub modified: Option<PdfDate>,
}

impl FileToAttach {
    /// The bytes `file` named `name`, of type `mime`, with no description or dates.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn new(name: impl Into<String>, file: BlobId, mime: impl Into<String>) -> Self {
        Self {
            file,
            name: name.into(),
            mime: mime.into(),
            description: None,
            created: None,
            modified: None,
        }
    }

    /// The same file with its last change at `modified`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn with_modified(mut self, modified: PdfDate) -> Self {
        self.modified = Some(modified);
        self
    }
}

/// A file attachment's icon: the four names of ISO 32000-2 Table 188.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub enum AttachmentIcon {
    /// A paper clip, the default.
    #[default]
    Paperclip,
    /// A push pin, which readers draw for a name they don't know.
    PushPin,
    /// A bar chart.
    Graph,
    /// A tag.
    Tag,
}

impl AttachmentIcon {
    /// Its `/Name`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Paperclip => "Paperclip",
            Self::PushPin => "PushPin",
            Self::Graph => "Graph",
            Self::Tag => "Tag",
        }
    }

    /// The icon a `/Name` names; any other is a push pin, as ISO 32000-2 makes it the default.
    pub(crate) fn from_name(name: &[u8]) -> Self {
        [Self::Paperclip, Self::Graph, Self::Tag]
            .into_iter()
            .find(|i| i.name().as_bytes() == name)
            .unwrap_or(Self::PushPin)
    }
}

/// A change to a file attachment's icon. The file stays as it is.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct FileAttachmentPatch {
    /// The icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub icon: Option<AttachmentIcon>,
}

impl FileAttachmentPatch {
    /// The patch that changes the icon to `icon`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn icon(icon: AttachmentIcon) -> Self {
        Self { icon: Some(icon) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use laminapdf_cos::Object;

    #[test]
    fn an_icon_reads_back_from_its_name_and_corner() {
        let dict: Dict = [("Name", Object::name("Graph"))].into_iter().collect();
        let read = FileAttachment::read(&dict, Some(UserSpaceRect::from_corners(10.0, 20.0, 30.0, 40.0))).unwrap();
        assert_eq!(
            (read.at, read.icon),
            (UserSpacePoint::new(10.0, 40.0), AttachmentIcon::Graph)
        );
    }

    #[test]
    fn an_unknown_icon_is_a_push_pin() {
        assert_eq!(AttachmentIcon::from_name(b"Star"), AttachmentIcon::PushPin);
    }

    #[test]
    fn the_wire_takes_the_file_inline() {
        let spec: FileAttachment = serde_json::from_str(&format!(
            r#"{{ "at": {{ "x": 1, "y": 2 }}, "file": {{ "file": "{}", "name": "a.csv" }} }}"#,
            "00".repeat(32)
        ))
        .unwrap();
        assert_eq!(
            (spec.icon, spec.file.name.as_str()),
            (AttachmentIcon::Paperclip, "a.csv")
        );
    }
}
