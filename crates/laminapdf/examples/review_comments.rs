//! Answer a review: read the comments on a marked-up report, reply to a question and mark a
//! highlight done.
//!
//! The comments come back as threads, each with its replies and the status set on it, and a
//! highlight's thread carries the text it marks. A reply and a status are annotations of their
//! own, so the review keeps its history.
//!
//! ```sh
//! cargo run -p laminapdf --features extract --example review_comments -- marked.pdf <time> [out-dir]   # the time as a PDF date
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{AddReply, Document, Op, OpenOptions, PdfDate, ReviewState, SaveMode, SaveOptions, SetReviewState};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: review_comments marked.pdf <time> [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    // When the review was answered, which your app takes from its clock.
    let at: PdfDate = args.next().ok_or(usage)?.parse()?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let answered = review(std::fs::read(input)?, at)?;
    std::fs::write(out.join("review_comments.pdf"), answered)?;
    Ok(())
}

/// Who answers the review.
const ME: &str = "Grace Hopper";

/// The document with a reply to every note that asks a question, and every highlight marked
/// done, saved incrementally.
pub(crate) fn review(bytes: Vec<u8>, at: PdfDate) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let mut ops = Vec::new();
    for thread in doc.comment_threads(None)? {
        let id = thread.comment.id.parse()?;
        let asks = thread.comment.contents.as_deref().is_some_and(|c| c.ends_with('?'));
        if thread.comment.subtype == "Text" && asks {
            let mut reply = AddReply::new(id, "June's figures are in the appendix.");
            reply.author = Some(ME.into());
            reply.created = Some(at);
            ops.push(Op::from(reply));
        } else if thread.comment.subtype == "Highlight" && thread.review.as_deref() != Some("Completed") {
            // `quoted` is the text the highlight marks, for a list of what was checked.
            let mut done = SetReviewState::new(id, ReviewState::Completed);
            done.author = Some(ME.into());
            done.created = Some(at);
            ops.push(Op::from(done));
        }
    }
    // Every answer is one undo step.
    doc.apply(ops)?;
    let options = SaveOptions::new().with_mode(SaveMode::Incremental);
    Ok(doc.save(&options)?.bytes)
}
