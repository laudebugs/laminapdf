//! Stamps, files attached to a spot on a page, and sound comments (ISO 32000-2 §12.5.6.12,
//! §12.5.6.15, §12.5.6.16), from boxes and points in page space (ADR 0027):
//! [`Document::stamp_at`] and [`Document::attach_file_at`] give the edit, upright as the page is
//! shown, and [`Document::sound_as_wav`] gives a sound comment's recording as a WAV file.
//!
//! Standard and image stamps are drawn by every build. A dynamic stamp's lines are set in
//! Helvetica, which needs the `extract` or `text` feature (in WebAssembly, the extract or text
//! module): elsewhere the edit fails with `appearance-needs-text`.

use laminapdf_model::{AnnotId, PointPt, RectPt};
use laminapdf_ops::{AddAnnot, AnnotSpec, AttachmentFile, AttachmentIcon, FileAttachment, Stamp, StampKind};

use crate::document::Document;
use crate::error::{Error, Result};
use crate::links::user_rect;

impl Document {
    /// The edit that stamps `area`, a box in page space on page `page` (0-based), with `stamp`,
    /// upright as the page is shown. Its `/Contents`, which a screen reader reads, is the
    /// stamp's name or lines unless the edit's common entries give one. An image is added with
    /// [`Document::add_blob`] first.
    ///
    /// ```
    /// # use laminapdf::{Document, Op, OpenOptions, Pt, RectPt, StampKind, StampName};
    /// # let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/corpus/hello.pdf"))?;
    /// let mut doc = Document::open_bytes(bytes, OpenOptions::default())?;
    /// let area = RectPt::from_xywh(Pt::new(300.0), Pt::new(60.0), Pt::new(180.0), Pt::new(50.0));
    /// let stamp = doc.stamp_at(0, area, StampKind::Standard(StampName::Approved))?;
    /// let edited = doc.apply(vec![Op::from(stamp)])?;
    /// let annots = doc.page_annotations(0)?;
    /// assert_eq!(annots[0].contents.as_deref(), Some("Approved"));
    /// # assert_eq!(edited.created_annotations.len(), 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors (the page tree).
    ///
    /// Added in 0.6.0.
    pub fn stamp_at(&self, page: u32, area: RectPt, stamp: StampKind) -> Result<AddAnnot> {
        let (id, frame) = self.page_frame(page)?;
        let spec = Stamp::new(user_rect(&frame, &[area]), stamp).with_rotation(frame.rotate().degrees());
        Ok(AddAnnot::new(id, AnnotSpec::Stamp(spec)))
    }

    /// The edit that attaches `bytes` as the file `name` of type `mime` (empty for none) at
    /// `at` on page `page` (0-based): an icon whose upper-left corner is `at` in page space,
    /// holding the file. The bytes are kept for the session as [`Document::add_blob`] keeps them;
    /// set the file's description or dates on the spec before applying it.
    ///
    /// ```
    /// # use laminapdf::{AnnotDetails, Document, Op, OpenOptions, PointPt, Pt};
    /// # let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/corpus/hello.pdf"))?;
    /// let mut doc = Document::open_bytes(bytes, OpenOptions::default())?;
    /// let at = PointPt::new(Pt::new(40.0), Pt::new(40.0));
    /// let attach = doc.attach_file_at(0, at, "readings.csv", b"day,reading\n1,12.5\n".to_vec(), "text/csv")?;
    /// doc.apply(vec![Op::from(attach)])?;
    /// let annots = doc.page_annotations(0)?;
    /// assert!(matches!(&annots[0].details, AnnotDetails::FileAttachment { name: Some(n), .. } if n == "readings.csv"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors (the page tree). An empty name fails when the
    /// edit is applied (`invalid-op`).
    ///
    /// Added in 0.6.0.
    pub fn attach_file_at(
        &mut self,
        page: u32,
        at: PointPt,
        name: impl Into<String>,
        bytes: Vec<u8>,
        mime: impl Into<String>,
    ) -> Result<AddAnnot> {
        let (id, frame) = self.page_frame(page)?;
        let file = AttachmentFile::new(name, self.add_blob(bytes), mime);
        let spec = FileAttachment::new(frame.to_user_space(at), AttachmentIcon::default(), file);
        Ok(AddAnnot::new(id, AnnotSpec::FileAttachment(spec)))
    }

    /// The recording of the sound comment `id` as a WAV file, which every browser plays: raw,
    /// signed, μ-law and A-law samples at their rate and channels, as 8-bit or 16-bit PCM.
    ///
    /// # Errors
    /// `sound-unavailable` when no page has a `Sound` annotation with this ID, it has no sound
    /// stream, or its samples are compressed or in a format PDF doesn't allow; `limit-exceeded`
    /// when its samples or the WAV file are larger than
    /// [`Limits::max_stream_decoded`](crate::Limits::max_stream_decoded); decoding errors.
    ///
    /// Added in 0.6.0.
    pub fn sound_as_wav(&self, id: &AnnotId) -> Result<Vec<u8>> {
        let sound = self.ops().sound(id)?.ok_or(Error::SoundUnavailable)?;
        sound
            .to_wav(self.ops().limits().max_stream_decoded())?
            .ok_or(Error::SoundUnavailable)
    }
}
