//! What a stamp, a file attachment or a sound annotation holds besides the entries every
//! annotation has ([`AnnotDetails`]), read for [`Annot::details`](crate::Annot::details).

use laminapdf_cos::{Dict, Object, Result};

use crate::AnnotSubtype;
use crate::attachments::read_spec;
use crate::document::Document;
use crate::id::AttachmentId;
use crate::sound::SoundFormat;
use crate::values::read_name;

/// A stamp's, file attachment's or sound annotation's own facts.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum AnnotDetails {
    /// None: any other subtype.
    #[default]
    None,
    /// A `Stamp` (ISO 32000-2 §12.5.6.12).
    Stamp {
        /// `/Name`: a standard name such as `Approved`, another app's own, or
        /// `LMNA_Image`/`LMNA_Dynamic` for Lamina PDF's image and dynamic stamps; `None` when absent,
        /// which readers show as `Draft`.
        name: Option<String>,
    },
    /// A `FileAttachment` (§12.5.6.15).
    FileAttachment {
        /// `/Name`, the icon: `PushPin`, `Paperclip`, `Graph`, `Tag` or another app's.
        icon: Option<String>,
        /// The attachment's ID, for [`Document::attachment_bytes`]; `None` for a direct file
        /// specification or none.
        file: Option<AttachmentId>,
        /// The file's name: `/UF`, else `/F`.
        name: Option<String>,
        /// The embedded file's MIME type.
        mime: Option<String>,
        /// The file's size, as `/Params /Size` states it.
        size: Option<u64>,
        /// `/Desc`.
        description: Option<String>,
    },
    /// A `Sound` (§12.5.6.16).
    Sound {
        /// The samples' format; `None` without a sound stream.
        format: Option<SoundFormat>,
        /// How long it plays, in seconds, from the stored length of an unfiltered stream or the
        /// decoded length; `None` when unknown.
        seconds: Option<f64>,
    },
}

impl Document {
    pub(crate) fn annot_details(&self, subtype: &AnnotSubtype, dict: &Dict) -> Result<AnnotDetails> {
        let name = |key: &str| -> Result<Option<String>> {
            Ok(read_name(self, dict.get(key))?.map(|n| String::from_utf8_lossy(&n).into_owned()))
        };
        Ok(match subtype {
            AnnotSubtype::Stamp => AnnotDetails::Stamp { name: name("Name")? },
            AnnotSubtype::FileAttachment => {
                let fs = dict.get("FS");
                let file = fs.and_then(Object::as_ref).map(AttachmentId::object);
                let spec = match fs {
                    Some(fs) => self.read(fs)?.and_then(|o| o.as_dict().cloned()),
                    None => None,
                };
                // The specification alone, not the document-wide scan `attachment` makes.
                let found = match (&file, &spec) {
                    (Some(id), Some(spec)) => Some(read_spec(self, id.clone(), spec)?),
                    _ => None,
                };
                AnnotDetails::FileAttachment {
                    icon: name("Name")?,
                    file,
                    name: found.as_ref().and_then(|f| f.name.clone()),
                    mime: found.as_ref().and_then(|f| f.mime.clone()),
                    size: found.as_ref().and_then(|f| f.size),
                    description: found.and_then(|f| f.description),
                }
            }
            AnnotSubtype::Sound => {
                let stream = match dict.get("Sound") {
                    Some(s) => self.read(s)?,
                    None => None,
                };
                match stream.as_deref() {
                    Some(Object::Stream(stream)) => {
                        let format = self.sound_format(&stream.dict)?;
                        // An unfiltered stream's stored length is its samples' length, read
                        // without decoding; a filtered one's isn't known until it is played.
                        let seconds = stream
                            .dict
                            .get("Filter")
                            .is_none()
                            .then(|| format.seconds(stream.data.len()));
                        AnnotDetails::Sound {
                            format: Some(format),
                            seconds,
                        }
                    }
                    _ => AnnotDetails::Sound {
                        format: None,
                        seconds: None,
                    },
                }
            }
            _ => AnnotDetails::None,
        })
    }
}
