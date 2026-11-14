//! `MergeDocuments`, `Document::merge` and `Document::split` (E2 plan §2.1, §2.2) over
//! `fixtures/pages/`: bookmarks per file, kept or left out; named destinations renamed on a
//! clash; form fields under the clash rule; layers; page labels; tags dropped and the PDF/A
//! warning; undo; and parts split by count, ranges and bookmarks.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use std::num::NonZeroUsize;

use common::{fixture, open, page_ids, snapshot};
use laminapdf_cos::Object;
use laminapdf_model::{OutlineItem, PagePosition};
use laminapdf_ops::{
    ConformanceWarning, Document, FieldOutcome, MergeDocuments, MergeInput, MergeOptions, MergeSource, Origin,
    OutlineMerge, SourceId, SplitBy, pending_conformance_warnings,
};

fn pages(name: &str) -> Document {
    open(&fixture(&format!("pages/{name}")))
}

/// `name` from `fixtures/pages/` added to `doc` as a source.
fn source(doc: &mut Document, name: &str) -> SourceId {
    doc.add_source(fixture(&format!("pages/{name}")), None).unwrap()
}

fn merge(doc: &mut Document, sources: Vec<MergeSource>, options: MergeOptions) -> laminapdf_ops::Applied {
    doc.apply_all(
        Origin::Local,
        [MergeDocuments::new(sources, PagePosition::End)
            .with_options(options)
            .into()],
    )
    .unwrap()
}

/// The outline as (depth, title) per item, in order.
fn outline(doc: &laminapdf_model::Document) -> Vec<(usize, String)> {
    fn walk(item: OutlineItem<'_, '_>, out: &mut Vec<(usize, String)>) {
        out.push((item.depth(), item.title().unwrap()));
        for child in item.children() {
            walk(child, out);
        }
    }
    let outline = doc.outline().unwrap();
    let mut out = Vec::new();
    for item in outline.roots() {
        walk(item, &mut out);
    }
    out
}

/// The page index each top-level bookmark goes to.
fn targets(doc: &laminapdf_model::Document) -> Vec<Option<usize>> {
    let outline = doc.outline().unwrap();
    outline
        .roots()
        .map(|item| item.target().unwrap().and_then(|t| t.page_index))
        .collect()
}

fn labels(doc: &laminapdf_model::Document) -> Vec<String> {
    let labels = doc.page_labels().unwrap();
    (0..doc.page_count().unwrap())
        .map(|i| labels.label(i).unwrap_or_default())
        .collect()
}

fn titled_id(id: SourceId, title: &str) -> MergeSource {
    MergeSource::new(id).with_title(title)
}

fn titled(doc: &mut Document, name: &str, title: &str) -> MergeSource {
    MergeSource::new(source(doc, name)).with_title(title)
}

#[test]
fn a_merge_appends_each_files_pages_in_order() {
    let mut doc = pages("references.pdf");
    let before = doc.page_count().unwrap();
    let sources = vec![
        titled(&mut doc, "chapters.pdf", "Chapters"),
        titled(&mut doc, "labels.pdf", "Labels"),
    ];
    let applied = merge(&mut doc, sources, MergeOptions::default());
    assert_eq!(doc.page_count().unwrap(), before + 12 + 10);
    assert_eq!(applied.page_order.unwrap().len(), before + 22);
}

#[test]
fn per_file_bookmarks_go_under_one_named_after_each_file() {
    let mut doc = pages("references.pdf");
    let own = outline(&doc).len();
    let sources = vec![titled(&mut doc, "chapters.pdf", "Chapters")];
    let applied = merge(&mut doc, sources, MergeOptions::default());
    let tree = outline(&doc);
    assert_eq!(tree[own], (1, "Chapters".to_owned()));
    let chapter_titles: Vec<&str> = tree[own + 1..]
        .iter()
        .filter(|(depth, _)| *depth == 2)
        .map(|(_, t)| t.as_str())
        .collect();
    assert_eq!(chapter_titles, ["Chapter 1", "Chapter 2", "Chapter 3"]);
    assert_eq!(tree.len(), own + 1 + 9);
    assert_eq!(applied.created_outline_items.len(), 1);
}

#[test]
fn the_per_file_bookmark_opens_the_files_first_page() {
    let mut doc = pages("references.pdf");
    let before = doc.page_count().unwrap();
    let sources = vec![titled(&mut doc, "chapters.pdf", "Chapters")];
    merge(&mut doc, sources, MergeOptions::default());
    assert_eq!(targets(&doc).last().copied().flatten(), Some(before));
}

#[test]
fn a_file_that_brings_no_pages_gets_no_bookmark() {
    let mut doc = pages("references.pdf");
    let before = doc.page_count().unwrap();
    let mut empty = titled(&mut doc, "labels.pdf", "Labels");
    empty.pages = Some(Vec::new());
    let sources = vec![empty, titled(&mut doc, "chapters.pdf", "Chapters")];
    let applied = merge(&mut doc, sources, MergeOptions::default());
    let titles: Vec<String> = outline(&doc)
        .into_iter()
        .filter(|(d, _)| *d == 1)
        .map(|(_, t)| t)
        .collect();
    assert!(!titles.contains(&"Labels".to_owned()), "{titles:?}");
    assert_eq!(targets(&doc).last().copied().flatten(), Some(before));
    assert_eq!(applied.created_outline_items.len(), 1);
}

#[test]
fn kept_bookmarks_join_the_top_level() {
    let mut doc = pages("labels.pdf");
    let id = source(&mut doc, "chapters.pdf");
    merge(
        &mut doc,
        vec![MergeSource::new(id)],
        MergeOptions::default().with_outline(OutlineMerge::Keep),
    );
    let top: Vec<String> = outline(&doc)
        .into_iter()
        .filter(|(d, _)| *d == 1)
        .map(|(_, t)| t)
        .collect();
    assert_eq!(top, ["Chapter 1", "Chapter 2", "Chapter 3"]);
}

#[test]
fn a_file_without_a_title_adds_its_bookmarks_at_the_top_level() {
    let mut doc = pages("labels.pdf");
    let id = source(&mut doc, "chapters.pdf");
    let applied = merge(&mut doc, vec![MergeSource::new(id)], MergeOptions::default());
    assert_eq!(applied.created_outline_items, []);
    assert_eq!(
        outline(&doc).first().map(|(d, t)| (*d, t.as_str())),
        Some((1, "Chapter 1"))
    );
}

#[test]
fn bookmarks_left_out_leave_the_outline_alone() {
    let mut doc = pages("references.pdf");
    let own = outline(&doc);
    let id = source(&mut doc, "chapters.pdf");
    merge(
        &mut doc,
        vec![MergeSource::new(id)],
        MergeOptions::default().with_outline(OutlineMerge::None),
    );
    assert_eq!(outline(&doc), own);
}

#[test]
fn copied_bookmarks_go_to_the_copies_of_their_pages() {
    let mut doc = pages("labels.pdf");
    let before = doc.page_count().unwrap();
    let id = source(&mut doc, "chapters.pdf");
    merge(
        &mut doc,
        vec![MergeSource::new(id)],
        MergeOptions::default().with_outline(OutlineMerge::Keep),
    );
    // Chapters start on the source's pages 1, 5 and 9; chapter 2 goes there by name.
    assert_eq!(targets(&doc), [Some(before), Some(before + 4), Some(before + 8)]);
}

#[test]
fn merging_a_file_twice_renames_its_named_destination() {
    let mut doc = pages("labels.pdf");
    let id = source(&mut doc, "chapters.pdf");
    let applied = merge(
        &mut doc,
        vec![MergeSource::new(id), MergeSource::new(id)],
        MergeOptions::default().with_outline(OutlineMerge::Keep),
    );
    let renamed: Vec<(&str, &str)> = applied
        .renamed_destinations
        .iter()
        .map(|r| (r.from.as_str(), r.to.as_str()))
        .collect();
    assert_eq!(renamed, [("chapter-2", "chapter-2#2")]);
    let before = 10;
    assert_eq!(targets(&doc)[4], Some(before + 12 + 4), "the second copy's chapter 2");
}

#[test]
fn bookmarks_to_pages_left_behind_are_left_out_and_counted() {
    let mut doc = pages("labels.pdf");
    let id = source(&mut doc, "chapters.pdf");
    let src_pages = page_ids(doc.source_document(&id).unwrap());
    let applied = merge(
        &mut doc,
        vec![MergeSource::new(id).with_pages(src_pages[..4].to_vec())],
        MergeOptions::default().with_outline(OutlineMerge::Keep),
    );
    let tree = outline(&doc);
    assert_eq!(
        tree,
        [
            (1, "Chapter 1".into()),
            (2, "Section 1.1".into()),
            (2, "Section 1.2".into())
        ]
    );
    assert_eq!(applied.bookmarks_dropped, 6);
}

#[test]
fn same_named_fields_from_two_files_are_renamed() {
    let mut doc = pages("labels.pdf");
    let id = source(&mut doc, "forms.pdf");
    let applied = merge(
        &mut doc,
        vec![MergeSource::new(id), MergeSource::new(id)],
        MergeOptions::default(),
    );
    let renamed = applied
        .fields
        .iter()
        .filter(|f| matches!(f.outcome, FieldOutcome::Renamed { .. }))
        .count();
    assert!(renamed > 0, "{:?}", applied.fields);
}

#[test]
fn the_files_layers_join_the_documents_and_keep_their_visibility() {
    let mut doc = pages("labels.pdf");
    let id = source(&mut doc, "layers.pdf");
    merge(&mut doc, vec![MergeSource::new(id)], MergeOptions::default());
    let catalog = doc.catalog().unwrap();
    let props = doc.deref(catalog.get("OCProperties").unwrap()).unwrap().into_owned();
    let props = props.as_dict().unwrap();
    let ocgs = props.get("OCGs").unwrap().as_array().unwrap();
    assert_eq!(ocgs.len(), 2);
    let config = props.get("D").unwrap().as_dict().unwrap();
    let off = config.get("OFF").unwrap().as_array().unwrap();
    let off_name = doc.resolve(off[0].as_ref().unwrap()).unwrap();
    assert_eq!(
        off_name.as_dict().unwrap().get("Name"),
        Some(&Object::String(laminapdf_cos::PdfString::literal("Notes")))
    );
    assert_eq!(config.get("Order").unwrap().as_array().unwrap().len(), 2);
}

#[test]
fn copied_pages_keep_the_labels_their_file_gave_them() {
    let mut doc = pages("chapters.pdf");
    let id = source(&mut doc, "labels.pdf");
    let source_labels = labels(doc.source_document(&id).unwrap());
    merge(&mut doc, vec![MergeSource::new(id)], MergeOptions::default());
    let got = labels(&doc);
    assert_eq!(got[..3], ["1", "2", "3"], "the target numbers its own pages");
    assert_eq!(got[12..], source_labels[..]);
}

#[test]
fn merged_tagged_pages_keep_their_tags_in_a_part_named_after_the_file() {
    let mut doc = pages("tagged.pdf");
    let before = doc.structure().unwrap().unwrap().elements().len();
    let id = source(&mut doc, "tagged.pdf");
    let applied = merge(&mut doc, vec![titled_id(id, "Again")], MergeOptions::default());
    assert_eq!(applied.tags_dropped, []);
    let tree = doc.structure().unwrap().unwrap();
    let document = &tree.roots[0];
    let part = document.children().last().unwrap();
    assert_eq!(part.kind, b"Part");
    // The source's Document element isn't copied: its five children sit in the Part.
    assert_eq!(part.children().count(), 5);
    assert_eq!(tree.elements().len(), before + 1 + 5);
    let warnings = pending_conformance_warnings(&doc, doc.pending_changes()).unwrap();
    assert!(
        !warnings.contains(&ConformanceWarning::ImportedTagsDropped),
        "{warnings:?}"
    );
    // Both claim PDF/UA-1.
    assert!(
        !warnings.contains(&ConformanceWarning::ImportedPdfuaUnchecked),
        "{warnings:?}"
    );
}

#[test]
fn a_merge_with_an_untagged_file_into_a_new_document_drops_every_tag() {
    let merged = Document::merge(
        vec![
            MergeInput::new(fixture("pages/tagged.pdf")),
            MergeInput::new(fixture("pages/labels.pdf")),
        ],
        MergeOptions::default(),
    )
    .unwrap();
    assert!(merged.structure().unwrap().is_none());
    assert!(!merged.is_tagged().unwrap());
}

#[test]
fn a_merge_of_tagged_files_into_a_new_document_is_tagged() {
    let merged = Document::merge(
        vec![
            MergeInput::new(fixture("pages/tagged.pdf")).with_title("One"),
            MergeInput::new(fixture("pages/references.pdf")).with_title("Two"),
        ],
        MergeOptions::default(),
    )
    .unwrap();
    assert!(merged.is_tagged().unwrap());
    let tree = merged.structure().unwrap().unwrap();
    let parts: Vec<&[u8]> = tree.roots[0].children().map(|e| e.kind.as_slice()).collect();
    assert_eq!(parts, [&b"Part"[..], b"Part"]);
}

#[test]
fn pages_merged_into_a_pdf_a_file_from_one_without_the_claim_warn() {
    let mut doc = open(&fixture("golden/examples/pdf_a_archive-2b.pdf"));
    let id = source(&mut doc, "labels.pdf");
    merge(&mut doc, vec![MergeSource::new(id)], MergeOptions::default());
    let warnings = pending_conformance_warnings(&doc, doc.pending_changes()).unwrap();
    assert!(
        warnings.contains(&ConformanceWarning::ImportedPdfaUnchecked),
        "{warnings:?}"
    );
}

#[test]
fn pages_from_a_file_with_the_same_pdf_a_claim_dont_warn() {
    let bytes = fixture("golden/examples/pdf_a_archive-2b.pdf");
    let mut doc = open(&bytes);
    let id = doc.add_source(bytes, None).unwrap();
    merge(&mut doc, vec![MergeSource::new(id)], MergeOptions::default());
    let warnings = pending_conformance_warnings(&doc, doc.pending_changes()).unwrap();
    assert!(
        !warnings.contains(&ConformanceWarning::ImportedPdfaUnchecked),
        "{warnings:?}"
    );
}

#[test]
fn undo_takes_a_whole_merge_back() {
    let mut doc = pages("references.pdf");
    let before = snapshot(&doc);
    let sources = vec![
        titled(&mut doc, "chapters.pdf", "Chapters"),
        titled(&mut doc, "layers.pdf", "Layers"),
    ];
    merge(&mut doc, sources, MergeOptions::default());
    doc.undo().unwrap();
    assert_eq!(snapshot(&doc), before);
}

#[test]
fn a_merge_of_no_sources_changes_nothing() {
    let mut doc = pages("labels.pdf");
    let before = snapshot(&doc);
    merge(&mut doc, Vec::new(), MergeOptions::default());
    assert_eq!(snapshot(&doc), before);
}

#[test]
fn merge_makes_a_new_document_of_the_files() {
    let merged = Document::merge(
        vec![
            MergeInput::new(fixture("pages/chapters.pdf")).with_title("Chapters"),
            MergeInput::new(fixture("pages/labels.pdf")),
        ],
        MergeOptions::default(),
    )
    .unwrap();
    assert_eq!(merged.page_count().unwrap(), 22);
    assert_eq!(outline(&merged)[0], (1, "Chapters".to_owned()));
}

#[test]
fn merging_nothing_is_refused() {
    assert_eq!(
        Document::merge(Vec::new(), MergeOptions::default()).err(),
        Some(laminapdf_ops::Error::NoPagesLeft)
    );
}

#[test]
fn split_by_count_cuts_every_n_pages() {
    let doc = pages("chapters.pdf");
    let parts = doc.split(&SplitBy::Count(NonZeroUsize::new(5).unwrap())).unwrap();
    let sizes: Vec<usize> = parts.iter().map(|p| p.page_count().unwrap()).collect();
    assert_eq!(sizes, [5, 5, 2]);
}

#[test]
fn split_by_ranges_makes_one_part_per_list() {
    let doc = pages("chapters.pdf");
    let ids = page_ids(&doc);
    let ranges = vec![vec![ids[0].clone(), ids[1].clone()], vec![ids[11].clone()]];
    let parts = doc.split(&SplitBy::Ranges(ranges)).unwrap();
    let sizes: Vec<usize> = parts.iter().map(|p| p.page_count().unwrap()).collect();
    assert_eq!(sizes, [2, 1]);
}

#[test]
fn split_by_bookmarks_makes_a_part_per_chapter_with_its_bookmarks() {
    let doc = pages("chapters.pdf");
    let parts = doc
        .split(&SplitBy::Bookmarks {
            level: NonZeroUsize::new(1).unwrap(),
        })
        .unwrap();
    assert_eq!(parts.len(), 3);
    for (i, part) in parts.iter().enumerate() {
        let c = i + 1;
        assert_eq!(part.page_count().unwrap(), 4);
        assert_eq!(
            outline(part),
            [
                (1, format!("Chapter {c}")),
                (2, format!("Section {c}.1")),
                (2, format!("Section {c}.2")),
            ]
        );
        assert_eq!(targets(part), [Some(0)], "chapter {c} opens its part's first page");
    }
}

#[test]
fn a_chapter_reached_by_name_brings_its_named_destination() {
    let doc = pages("chapters.pdf");
    let parts = doc
        .split(&SplitBy::Bookmarks {
            level: NonZeroUsize::new(1).unwrap(),
        })
        .unwrap();
    let tree = parts[1].name_tree("Dests").unwrap().unwrap();
    assert!(tree.get(b"chapter-2").unwrap().is_some());
}

#[test]
fn split_by_section_bookmarks_keeps_the_pages_before_the_first() {
    let doc = pages("chapters.pdf");
    let ranges = doc
        .split_ranges(&SplitBy::Bookmarks {
            level: NonZeroUsize::new(2).unwrap(),
        })
        .unwrap();
    // Sections start on pages 2, 3, 6, 7, 10 and 11; page 1 is a part of its own.
    let sizes: Vec<usize> = ranges.iter().map(Vec::len).collect();
    assert_eq!(sizes, [1, 1, 3, 1, 3, 1, 2]);
}

#[test]
fn an_extracted_part_keeps_its_layers() {
    let doc = pages("layers.pdf");
    let ids = page_ids(&doc);
    let part = doc.extract_pages(&ids[..1]).unwrap();
    let catalog = part.catalog().unwrap();
    assert!(catalog.contains_key("OCProperties"));
}
