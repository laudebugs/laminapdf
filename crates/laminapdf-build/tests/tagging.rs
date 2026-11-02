//! Tagged PDF (todo B8a): the structure tree `tagged-report.pdf` carries, read back by
//! laminapdf-model, matches what the builder was asked for; every MCID is owned by exactly one
//! element and listed once in the `ParentTree`; links are `Link` elements with an `OBJR`;
//! untagged content and figures without alternate text are reported; untagged documents and
//! appended pages carry no structure.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use std::collections::BTreeMap;

use common::tagged::{FIGURE_ALT, PAGE_LINK, RUN_LINK, STRUCTURE, tagged_report, tagged_report_builder};
use common::{b5b::font, fixture, reopen, save};
use laminapdf_build::{
    ArtifactKind, BuildOptions, Builder, Color, ContentKind, FillRule, Image, PageSize, Standard14, Tag, TagDiagnostic,
    TagKind,
};
use laminapdf_content::{MarkedContent, Visitor, interpret_page};
use laminapdf_cos::{ObjRef, Object};
use laminapdf_model::{PageId, PointPt, Pt, RectPt, StructElem, StructKid, StructTree};

fn rect(x: f64, y: f64, w: f64, h: f64) -> RectPt {
    RectPt::from_xywh(Pt::new(x), Pt::new(y), Pt::new(w), Pt::new(h))
}

fn tree(doc: &laminapdf_model::Document) -> StructTree {
    doc.structure().unwrap().unwrap()
}

/// The marked-content sequences of a page's content stream, in order: (tag, MCID, how many
/// sequences enclose it).
#[derive(Default)]
struct Marks {
    open: Vec<bool>,
    seen: Vec<(String, Option<i64>, usize, usize)>,
}

impl Visitor for Marks {
    fn begin_marked(&mut self, marked: &MarkedContent<'_>) {
        let structural = self.open.iter().filter(|s| **s).count();
        let tag = String::from_utf8_lossy(marked.tag.as_bytes()).into_owned();
        self.seen.push((tag, marked.mcid, self.open.len(), structural));
        self.open.push(marked.mcid.is_some());
    }

    fn end_marked(&mut self) {
        self.open.pop();
    }
}

fn marks(doc: &laminapdf_model::Document, index: usize) -> Marks {
    let pages = doc.pages().unwrap();
    let mut m = Marks::default();
    interpret_page(doc, &pages.get(index).unwrap(), &mut m).unwrap();
    m
}

/// Every element below the `Document` element with its depth, in reading order.
fn flatten<'t>(elem: &'t StructElem, depth: usize, out: &mut Vec<(usize, &'t StructElem)>) {
    for child in elem.children() {
        out.push((depth, child));
        flatten(child, depth + 1, out);
    }
}

fn below_document(tree: &StructTree) -> Vec<(usize, &StructElem)> {
    let mut out = Vec::new();
    flatten(&tree.roots[0], 0, &mut out);
    out
}

#[test]
fn the_golden_is_the_report() {
    // Not `assert_eq!`: a byte diff of two PDFs is unreadable.
    assert!(
        tagged_report() == fixture("golden/tagged-report.pdf"),
        "stale tagged-report.pdf"
    );
}

mod structure {
    use super::*;

    #[test]
    fn the_root_has_one_document_element() {
        let doc = reopen(&fixture("golden/tagged-report.pdf"));
        let kinds: Vec<Vec<u8>> = tree(&doc).roots.iter().map(|e| e.kind.clone()).collect();
        assert_eq!(kinds, [b"Document".to_vec()]);
    }

    #[test]
    fn kinds_order_and_alt_text_match_the_builder_input() {
        let doc = reopen(&fixture("golden/tagged-report.pdf"));
        let tree = tree(&doc);
        let read: Vec<(usize, String, Option<String>)> = below_document(&tree)
            .into_iter()
            .map(|(d, e)| (d, String::from_utf8_lossy(&e.kind).into_owned(), e.alt.clone()))
            .collect();
        let expected: Vec<(usize, String, Option<String>)> = STRUCTURE
            .iter()
            .map(|(d, k, a)| (*d, (*k).to_owned(), a.map(str::to_owned)))
            .collect();
        assert_eq!(read, expected);
    }

    #[test]
    fn a_custom_type_is_role_mapped_to_a_standard_one() {
        let doc = reopen(&fixture("golden/tagged-report.pdf"));
        let tree = tree(&doc);
        assert_eq!(tree.standard_kind(b"Total"), b"P");
    }

    #[test]
    fn a_span_keeps_its_language_and_a_header_its_scope() {
        let doc = reopen(&fixture("golden/tagged-report.pdf"));
        let tree = tree(&doc);
        let span = tree.elements().into_iter().find(|e| e.kind == b"Span").unwrap();
        let th = tree
            .elements()
            .into_iter()
            .find(|e| e.kind == b"TH")
            .unwrap()
            .object
            .unwrap();
        let scope = doc
            .resolve(th)
            .unwrap()
            .as_dict()
            .and_then(|d| d.get("A").cloned())
            .and_then(|a| a.as_dict().and_then(|a| a.get_name("Scope").map(<[u8]>::to_vec)));
        assert_eq!((span.lang.as_deref(), scope), (Some("fr"), Some(b"Column".to_vec())));
    }

    #[test]
    fn the_catalog_says_the_document_is_marked() {
        let doc = reopen(&fixture("golden/tagged-report.pdf"));
        let catalog = doc.catalog().unwrap();
        let marked = catalog
            .get("MarkInfo")
            .and_then(Object::as_dict)
            .and_then(|m| m.get("Marked").cloned());
        assert_eq!(marked, Some(Object::Bool(true)));
    }

    #[test]
    fn the_golden_reads_back_without_warnings() {
        let doc = reopen(&fixture("golden/tagged-report.pdf"));
        let _ = tree(&doc);
        assert!(doc.warnings().is_empty(), "{:?}", doc.warnings());
    }
}

mod mcids {
    use super::*;

    /// Each page's MCIDs as the content stream has them.
    fn content_mcids(doc: &laminapdf_model::Document) -> Vec<Vec<i64>> {
        (0..doc.pages().unwrap().len())
            .map(|i| marks(doc, i).seen.iter().filter_map(|m| m.1).collect())
            .collect()
    }

    #[test]
    fn count_from_zero_on_every_page() {
        let doc = reopen(&fixture("golden/tagged-report.pdf"));
        for mcids in content_mcids(&doc) {
            assert_eq!(mcids, (0..i64::try_from(mcids.len()).unwrap()).collect::<Vec<_>>());
        }
    }

    #[test]
    fn structural_sequences_never_nest() {
        let doc = reopen(&fixture("golden/tagged-report.pdf"));
        for i in 0..2 {
            let nested: Vec<_> = marks(&doc, i)
                .seen
                .into_iter()
                .filter(|m| m.1.is_some() && m.3 > 0)
                .collect();
            assert!(nested.is_empty(), "page {i}: {nested:?}");
        }
    }

    #[test]
    fn each_is_owned_by_exactly_one_element() {
        let doc = reopen(&fixture("golden/tagged-report.pdf"));
        let tree = tree(&doc);
        let pages = doc.pages().unwrap();
        let mut owned: BTreeMap<(usize, i64), usize> = BTreeMap::new();
        for e in tree.elements() {
            for kid in &e.kids {
                if let StructKid::Mcid { page, mcid } = kid {
                    let index = pages.index_of(page.as_ref().unwrap()).unwrap();
                    *owned.entry((index, *mcid)).or_default() += 1;
                }
            }
        }
        let in_content: Vec<(usize, i64)> = content_mcids(&doc)
            .into_iter()
            .enumerate()
            .flat_map(|(p, m)| m.into_iter().map(move |m| (p, m)))
            .collect();
        assert_eq!(owned.keys().copied().collect::<Vec<_>>(), in_content);
        assert!(owned.values().all(|n| *n == 1));
    }

    #[test]
    fn the_parent_tree_lists_each_once_with_its_owner() {
        let doc = reopen(&fixture("golden/tagged-report.pdf"));
        let tree = tree(&doc);
        let parent_tree = doc.number_tree(tree.parent_tree.clone().unwrap());
        let pages = doc.pages().unwrap();
        let mcids = content_mcids(&doc);
        let owners: BTreeMap<(PageId, i64), ObjRef> = tree
            .elements()
            .into_iter()
            .flat_map(|e| {
                e.kids.iter().filter_map(move |k| match k {
                    StructKid::Mcid { page, mcid } => Some(((page.clone().unwrap(), *mcid), e.object.unwrap())),
                    _ => None,
                })
            })
            .collect();
        for (i, page) in pages.iter().enumerate() {
            let key = page.dict().get("StructParents").and_then(Object::as_int).unwrap();
            let Some(Object::Array(entry)) = parent_tree.get(key).unwrap() else {
                panic!("page {i}: no array under its /StructParents key")
            };
            assert_eq!(entry.len(), mcids[i].len(), "page {i}");
            for (mcid, owner) in entry.iter().enumerate() {
                assert_eq!(
                    owner.as_ref(),
                    owners.get(&(page.id().clone(), i64::try_from(mcid).unwrap())).copied(),
                    "page {i}"
                );
            }
        }
    }

    #[test]
    fn the_next_key_is_past_every_key() {
        let doc = reopen(&fixture("golden/tagged-report.pdf"));
        let tree = tree(&doc);
        let keys: Vec<i64> = doc
            .number_tree(tree.parent_tree.clone().unwrap())
            .entries()
            .unwrap()
            .iter()
            .map(|(k, _)| *k)
            .collect();
        assert_eq!((keys, tree.parent_tree_next_key), (vec![0, 1, 2, 3], Some(4)));
    }

    #[test]
    fn artifacts_have_no_mcid() {
        let doc = reopen(&fixture("golden/tagged-report.pdf"));
        let artifacts: Vec<_> = (0..2)
            .flat_map(|i| marks(&doc, i).seen)
            .filter(|m| m.0 == "Artifact")
            .collect();
        assert!(artifacts.len() >= 4 && artifacts.iter().all(|m| m.1.is_none()));
    }
}

mod links {
    use super::*;

    /// Every link annotation's object, `/Contents` and `/StructParent`.
    fn annotations(doc: &laminapdf_model::Document) -> Vec<(ObjRef, Option<Vec<u8>>, Option<i64>)> {
        let mut out = Vec::new();
        for page in doc.pages().unwrap().iter() {
            for r in page.annots().unwrap() {
                let annot = doc.resolve(r).unwrap();
                let d = annot.as_dict().unwrap();
                let contents = match d.get("Contents") {
                    Some(Object::String(s)) => Some(s.bytes.clone()),
                    _ => None,
                };
                out.push((r, contents, d.get("StructParent").and_then(Object::as_int)));
            }
        }
        out
    }

    #[test]
    fn are_link_elements_holding_their_annotation_and_have_contents() {
        let doc = reopen(&fixture("golden/tagged-report.pdf"));
        let tree = tree(&doc);
        let objrs: Vec<(Vec<u8>, ObjRef)> = tree
            .elements()
            .into_iter()
            .flat_map(|e| {
                e.kids.iter().filter_map(|k| match k {
                    StructKid::Object { object, .. } => Some((e.kind.clone(), *object)),
                    _ => None,
                })
            })
            .collect();
        let annots = annotations(&doc);
        let contents: Vec<Option<Vec<u8>>> = annots.iter().map(|a| a.1.clone()).collect();
        assert_eq!(
            objrs,
            annots.iter().map(|a| (b"Link".to_vec(), a.0)).collect::<Vec<_>>()
        );
        assert_eq!(
            contents,
            [Some(RUN_LINK.as_bytes().to_vec()), Some(PAGE_LINK.as_bytes().to_vec())]
        );
    }

    #[test]
    fn struct_parent_keys_resolve_to_their_link_element() {
        let doc = reopen(&fixture("golden/tagged-report.pdf"));
        let tree = tree(&doc);
        let parent_tree = doc.number_tree(tree.parent_tree.clone().unwrap());
        for (annot, _, key) in annotations(&doc) {
            let owner = parent_tree.get(key.unwrap()).unwrap().and_then(|o| o.as_ref()).unwrap();
            let elem = tree.elements().into_iter().find(|e| e.object == Some(owner)).unwrap();
            assert!(elem.kids.contains(&StructKid::Object {
                page: elem.page.clone(),
                object: annot
            }));
        }
    }

    #[test]
    fn put_annotations_in_structure_order() {
        let doc = reopen(&fixture("golden/tagged-report.pdf"));
        for page in doc.pages().unwrap().iter() {
            assert_eq!(page.dict().get_name("Tabs"), Some(&b"S"[..]));
        }
    }
}

mod diagnostics {
    use super::*;

    #[test]
    fn are_empty_for_the_tagged_report() {
        assert_eq!(tagged_report_builder().tagging_diagnostics(), []);
    }

    #[test]
    fn report_a_path_drawn_outside_every_element() {
        let mut b = Builder::new(BuildOptions::default().with_tagged(true));
        let mut page = b.page(PageSize::A4);
        page.surface()
            .path(|p| {
                p.rect(rect(10.0, 10.0, 10.0, 10.0));
            })
            .fill(FillRule::NonZero);
        page.finish().unwrap();
        assert_eq!(
            b.tagging_diagnostics(),
            [TagDiagnostic::Untagged {
                page: 0,
                content: ContentKind::Path,
                count: 1
            }]
        );
    }

    #[test]
    fn report_a_figure_without_alt_text() {
        let mut b = Builder::new(BuildOptions::default());
        let image = Image::rgba(1, 1, &[0, 0, 0, 255]).unwrap();
        let mut page = b.page(PageSize::A4);
        page.surface().figure(&image, rect(10.0, 10.0, 10.0, 10.0), None);
        page.finish().unwrap();
        assert_eq!(b.tagging_diagnostics(), [TagDiagnostic::MissingAlt { page: 0 }]);
    }

    #[test]
    fn are_empty_while_the_document_is_untagged() {
        let mut b = Builder::new(BuildOptions::default());
        let mut page = b.page(PageSize::A4);
        page.surface()
            .path(|p| {
                p.rect(rect(10.0, 10.0, 10.0, 10.0));
            })
            .fill(FillRule::NonZero);
        page.finish().unwrap();
        assert_eq!(b.tagging_diagnostics(), []);
    }

    #[test]
    fn report_a_link_inside_an_artifact() {
        let mut b = Builder::new(BuildOptions::default().with_tagged(true));
        let mut page = b.page(PageSize::A4);
        page.surface()
            .artifact(ArtifactKind::Pagination, |s| {
                s.link(
                    rect(10.0, 10.0, 10.0, 10.0),
                    laminapdf_build::LinkTarget::Uri("https://example.com".into()),
                    None,
                );
                Ok(())
            })
            .unwrap();
        page.finish().unwrap();
        assert_eq!(
            b.tagging_diagnostics(),
            [TagDiagnostic::Untagged {
                page: 0,
                content: ContentKind::Link,
                count: 1
            }]
        );
    }

    #[test]
    fn report_what_pages_finished_before_tagging_began_drew_and_linked() {
        let mut b = Builder::new(BuildOptions::default());
        let mut first = b.page(PageSize::A4);
        first
            .surface()
            .path(|p| {
                p.rect(rect(10.0, 10.0, 10.0, 10.0));
            })
            .fill(FillRule::NonZero)
            .link(
                rect(10.0, 10.0, 10.0, 10.0),
                laminapdf_build::LinkTarget::Uri("https://example.com".into()),
                None,
            );
        first.finish().unwrap();
        let mut second = b.page(PageSize::A4);
        second
            .surface()
            .tag(TagKind::Figure, |s| {
                s.path(|p| {
                    p.rect(rect(10.0, 10.0, 10.0, 10.0));
                })
                .fill(FillRule::NonZero);
                Ok(())
            })
            .unwrap();
        second.finish().unwrap();
        let untagged = |content| TagDiagnostic::Untagged {
            page: 0,
            content,
            count: 1,
        };
        assert_eq!(
            b.tagging_diagnostics(),
            [
                untagged(ContentKind::Path),
                untagged(ContentKind::Link),
                TagDiagnostic::MissingAlt { page: 1 }
            ]
        );
    }

    #[test]
    fn a_clip_alone_is_not_content() {
        let mut b = Builder::new(BuildOptions::default().with_tagged(true));
        let mut page = b.page(PageSize::A4);
        let _ = page
            .surface()
            .save()
            .path(|p| {
                p.rect(rect(10.0, 10.0, 10.0, 10.0));
            })
            .clip(FillRule::NonZero)
            .restore();
        page.finish().unwrap();
        assert_eq!(b.tagging_diagnostics(), []);
    }
}

mod documents {
    use super::*;

    #[test]
    fn without_tags_have_no_structure_tree() {
        let mut b = Builder::new(BuildOptions::default());
        let helvetica = b.standard_font(Standard14::Helvetica).unwrap();
        let mut page = b.page(PageSize::A4);
        let _ = page
            .surface()
            .text("plain", &helvetica, Pt::new(12.0), PointPt::from_points(72.0, 72.0))
            .unwrap()
            .link(laminapdf_build::LinkTarget::Uri("https://example.com".into()), None);
        page.finish().unwrap();
        let doc = reopen(&save(b.finish().unwrap()));
        assert_eq!((doc.structure().unwrap(), marks(&doc, 0).seen.len()), (None, 0));
    }

    #[test]
    fn asked_to_be_tagged_have_a_document_element_even_when_nothing_is_tagged() {
        let mut b = Builder::new(BuildOptions::default().with_tagged(true));
        b.page(PageSize::A4).finish().unwrap();
        let doc = reopen(&save(b.finish().unwrap()));
        let tree = tree(&doc);
        assert_eq!(
            (tree.roots.len(), tree.roots[0].kids.len(), tree.parent_tree_next_key),
            (1, 0, Some(0))
        );
    }

    #[test]
    fn become_tagged_when_a_page_uses_an_element() {
        let mut b = Builder::new(BuildOptions::default());
        let helvetica = b.standard_font(Standard14::Helvetica).unwrap();
        let mut page = b.page(PageSize::A4);
        page.surface()
            .tag(TagKind::P, |s| {
                s.text("tagged", &helvetica, Pt::new(12.0), PointPt::from_points(72.0, 72.0))
                    .map(|_| ())
            })
            .unwrap();
        page.finish().unwrap();
        let doc = reopen(&save(b.finish().unwrap()));
        assert_eq!(below_document(&tree(&doc)).len(), 1);
    }

    #[test]
    fn an_element_left_open_is_ended_with_the_page() {
        let mut b = Builder::new(BuildOptions::default());
        let mut page = b.page(PageSize::A4);
        page.surface()
            .begin_tag(Tag::new(TagKind::Figure).with_alt("a square"))
            .path(|p| {
                p.rect(rect(10.0, 10.0, 10.0, 10.0));
            })
            .fill(FillRule::NonZero);
        page.finish().unwrap();
        let doc = reopen(&save(b.finish().unwrap()));
        let m = marks(&doc, 0);
        assert_eq!((m.seen.len(), m.open.len()), (1, 0));
    }

    #[test]
    fn a_failed_page_leaves_no_elements_or_keys_behind() {
        let mut b = Builder::new(BuildOptions::default().with_tagged(true));
        let mut page = b.page(PageSize::A4);
        page.surface()
            .tag(TagKind::P, |s| {
                s.path(|p| {
                    p.rect(rect(f64::NAN, 10.0, 10.0, 10.0));
                })
                .fill(FillRule::NonZero);
                Ok(())
            })
            .unwrap();
        assert!(page.finish().is_err());
        let doc = reopen(&save(b.finish().unwrap()));
        let tree = tree(&doc);
        assert_eq!((tree.roots[0].kids.len(), tree.parent_tree_next_key), (0, Some(0)));
    }

    #[test]
    fn an_element_inside_an_artifact_is_not_in_the_tree() {
        let mut b = Builder::new(BuildOptions::default());
        let mut page = b.page(PageSize::A4);
        page.surface()
            .artifact(ArtifactKind::Layout, |s| {
                s.tag(TagKind::P, |s| {
                    s.fill_color(Color::gray(0.5))
                        .path(|p| {
                            p.rect(rect(10.0, 10.0, 10.0, 10.0));
                        })
                        .fill(FillRule::NonZero);
                    Ok(())
                })
            })
            .unwrap();
        page.finish().unwrap();
        let doc = reopen(&save(b.finish().unwrap()));
        let seen: Vec<String> = marks(&doc, 0).seen.into_iter().map(|m| m.0).collect();
        assert_eq!(
            (below_document(&tree(&doc)).len(), seen),
            (0, vec!["Artifact".to_owned()])
        );
    }

    #[test]
    fn appended_pages_draw_elements_as_plain_content() {
        let mut doc = reopen(&fixture("golden/hello.pdf"));
        let mut appender = Builder::on(&mut doc, &BuildOptions::default().with_tagged(true)).unwrap();
        let mut page = appender.page(PageSize::A4);
        page.surface()
            .tag(TagKind::Figure, |s| {
                s.path(|p| {
                    p.rect(rect(10.0, 10.0, 10.0, 10.0));
                })
                .fill(FillRule::NonZero);
                Ok(())
            })
            .unwrap();
        page.finish().unwrap();
        appender.finish().unwrap();
        let last = doc.pages().unwrap().len() - 1;
        assert_eq!((marks(&doc, last).seen.len(), doc.structure().unwrap()), (0, None));
    }

    #[test]
    fn an_appended_page_with_a_tag_and_a_link_gets_no_tab_order() {
        let mut doc = reopen(&fixture("golden/hello.pdf"));
        let mut appender = Builder::on(&mut doc, &BuildOptions::default()).unwrap();
        let mut page = appender.page(PageSize::A4);
        page.surface()
            .tag(TagKind::Link, |s| {
                s.link(
                    rect(10.0, 10.0, 10.0, 10.0),
                    laminapdf_build::LinkTarget::Uri("https://example.com".into()),
                    None,
                );
                Ok(())
            })
            .unwrap();
        page.finish().unwrap();
        appender.finish().unwrap();
        let pages = doc.pages().unwrap();
        let last = pages.get(pages.len() - 1).unwrap();
        assert_eq!(last.dict().get("Tabs"), None);
    }

    #[test]
    fn tab_order_and_expansions_raise_a_1_4_document_to_1_5() {
        let mut b = Builder::new(BuildOptions::default().with_version(laminapdf_build::PdfVersion::V1_4));
        let mut page = b.page(PageSize::A4);
        page.surface()
            .tag(Tag::new(TagKind::Span).with_expansion("for example"), |s| {
                s.path(|p| {
                    p.rect(rect(10.0, 10.0, 10.0, 10.0));
                })
                .fill(FillRule::NonZero);
                Ok(())
            })
            .unwrap();
        page.finish().unwrap();
        let doc = reopen(&save(b.finish().unwrap()));
        assert_eq!(doc.catalog().unwrap().get_name("Version"), Some(&b"1.5"[..]));
    }

    #[test]
    fn a_tagged_1_4_document_without_links_or_expansions_stays_1_4() {
        let mut b = Builder::new(BuildOptions::default().with_version(laminapdf_build::PdfVersion::V1_4));
        let mut page = b.page(PageSize::A4);
        page.surface()
            .tag(TagKind::P, |s| {
                s.path(|p| {
                    p.rect(rect(10.0, 10.0, 10.0, 10.0));
                })
                .fill(FillRule::NonZero);
                Ok(())
            })
            .unwrap();
        page.finish().unwrap();
        let doc = reopen(&save(b.finish().unwrap()));
        assert_eq!(doc.catalog().unwrap().get_name("Version"), None);
    }

    #[test]
    fn a_custom_name_that_is_a_standard_type_is_written_unmapped() {
        let mut b = Builder::new(BuildOptions::default());
        let mut page = b.page(PageSize::A4);
        for name in ["Div", "BlockQuote", "THead"] {
            page.surface()
                .tag(Tag::custom(name, TagKind::Sect), |s| {
                    s.path(|p| {
                        p.rect(rect(10.0, 10.0, 10.0, 10.0));
                    })
                    .fill(FillRule::NonZero);
                    Ok(())
                })
                .unwrap();
        }
        page.finish().unwrap();
        let doc = reopen(&save(b.finish().unwrap()));
        let tree = tree(&doc);
        let kinds: Vec<Vec<u8>> = below_document(&tree).iter().map(|(_, e)| e.kind.clone()).collect();
        assert_eq!(
            (kinds, tree.role_map.len()),
            (vec![b"Div".to_vec(), b"BlockQuote".to_vec(), b"THead".to_vec()], 0)
        );
    }

    #[test]
    fn the_figure_alt_text_is_written_as_a_text_string() {
        let doc = reopen(&fixture("golden/tagged-report.pdf"));
        let figure = tree(&doc).elements().into_iter().find(|e| e.kind == b"Figure").cloned();
        assert_eq!(figure.and_then(|f| f.alt), Some(FIGURE_ALT.to_owned()));
    }

    #[test]
    fn text_with_an_embedded_font_inside_a_span_gets_its_own_mcid() {
        let mut b = Builder::new(BuildOptions::default());
        let noto = b.font(font("NotoSans-Regular.ttf")).unwrap();
        let mut page = b.page(PageSize::A4);
        page.surface()
            .tag(TagKind::P, |s| {
                let _ = s.text("a", &noto, Pt::new(12.0), PointPt::from_points(72.0, 72.0))?;
                s.tag(TagKind::Span, |s| {
                    s.text("b", &noto, Pt::new(12.0), PointPt::from_points(90.0, 72.0))
                        .map(|_| ())
                })
            })
            .unwrap();
        page.finish().unwrap();
        let doc = reopen(&save(b.finish().unwrap()));
        let tags: Vec<(String, Option<i64>)> = marks(&doc, 0).seen.into_iter().map(|m| (m.0, m.1)).collect();
        assert_eq!(tags, [("P".to_owned(), Some(0)), ("Span".to_owned(), Some(1))]);
    }
}

/// What pdf.js (`getStructTree`) and PDFium (`FPDF_StructTree_GetForPage`) read from the
/// tagged goldens: `fixtures/build/structure.json`, written by
/// `tools/fixtures/structure_facts.py` (the full-lane checks, precomputed so tests need neither
/// Node nor Python).
mod external {
    use std::fmt::Write as _;

    use serde_json::Value;
    use sha2::{Digest, Sha256};

    use super::*;

    fn facts() -> Value {
        serde_json::from_slice(&fixture("build/structure.json")).unwrap()
    }

    fn report() -> Value {
        facts()["files"]["tagged-report.pdf"].clone()
    }

    /// The version `pin` is set to in `text` (`name==1.2.3` or `NAME: &str = "1.2.3";`).
    fn pinned(text: &str, pin: &str) -> String {
        text.lines()
            .find_map(|l| l.trim().strip_prefix(pin))
            .map(|rest| rest.trim_matches(|c| c == '"' || c == ';' || c == ' ').to_owned())
            .unwrap()
    }

    /// The expected structure with custom types as both readers report them: through the
    /// `RoleMap`.
    fn expected() -> Vec<(usize, String, Option<String>)> {
        STRUCTURE
            .iter()
            .map(|(d, k, a)| {
                let kind = if *k == "Total" { "P" } else { *k };
                (*d, kind.to_owned(), a.map(str::to_owned))
            })
            .collect()
    }

    /// The elements below `Document` on every page, depth-first, as (depth, type, alt), with
    /// the MCIDs each page's elements hold in reading order.
    fn flat(node: &Value, type_key: &str, depth: usize, out: &mut Vec<(usize, String, Option<String>)>) {
        for child in node["children"].as_array().into_iter().flatten() {
            let Some(kind) = child[type_key].as_str() else { continue };
            out.push((depth, kind.to_owned(), child["alt"].as_str().map(str::to_owned)));
            flat(child, type_key, depth + 1, out);
        }
    }

    fn document<'v>(node: &'v Value, type_key: &str) -> &'v Value {
        if node[type_key] == "Document" {
            return node;
        }
        node["children"]
            .as_array()
            .and_then(|c| c.iter().find(|c| c[type_key] == "Document"))
            .unwrap()
    }

    #[test]
    fn describe_the_goldens_current_bytes() {
        let golden = fixture("golden/tagged-report.pdf");
        let hash: String = Sha256::digest(&golden).iter().fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        });
        assert_eq!(
            report()["sha256"].as_str(),
            Some(hash.as_str()),
            "stale fixtures/build/structure.json: rerun tools/fixtures/structure_facts.py"
        );
    }

    #[test]
    fn qpdf_check_passes() {
        assert_eq!(report()["qpdf_check"].as_bool(), Some(true));
    }

    #[test]
    fn pdfjs_reads_the_builder_input() {
        let mut read = Vec::new();
        for page in report()["pdfjs"]["pages"].as_array().unwrap() {
            flat(document(page, "role"), "role", 0, &mut read);
        }
        assert_eq!(read, expected());
    }

    #[test]
    fn pdfium_reads_the_builder_input() {
        let mut read = Vec::new();
        for page in report()["pdfium"]["pages"].as_array().unwrap() {
            let roots = page.as_array().unwrap();
            assert_eq!(roots.len(), 1);
            flat(document(&roots[0], "type"), "type", 0, &mut read);
        }
        assert_eq!(read, expected());
    }

    #[test]
    fn both_readers_find_every_mcid_of_every_page_once() {
        fn collect(node: &Value, out: &mut Vec<i64>) {
            out.extend(node["mcids"].as_array().into_iter().flatten().filter_map(Value::as_i64));
            for child in node["children"].as_array().into_iter().flatten() {
                if let Some(mcid) = child["content"].as_i64() {
                    out.push(mcid);
                }
                collect(child, out);
            }
        }
        let doc = reopen(&fixture("golden/tagged-report.pdf"));
        let report = report();
        for i in 0..2 {
            let in_content: Vec<i64> = marks(&doc, i).seen.iter().filter_map(|m| m.1).collect();
            let mut pdfjs = Vec::new();
            collect(&report["pdfjs"]["pages"][i], &mut pdfjs);
            let mut pdfium = Vec::new();
            collect(&report["pdfium"]["pages"][i][0], &mut pdfium);
            assert_eq!((&pdfjs, &pdfium), (&in_content, &in_content), "page {i}");
        }
    }

    #[test]
    fn pdfjs_sees_each_link_annotation_inside_its_link_element() {
        fn links(node: &Value, out: &mut Vec<String>) {
            for child in node["children"].as_array().into_iter().flatten() {
                if child["annotation"] == true {
                    out.push(node["role"].as_str().unwrap_or_default().to_owned());
                }
                links(child, out);
            }
        }
        let mut owners = Vec::new();
        for page in report()["pdfjs"]["pages"].as_array().unwrap() {
            links(page, &mut owners);
        }
        assert_eq!(owners, ["Link", "Link"]);
    }

    #[test]
    fn were_written_with_the_pinned_tools() {
        let facts = facts();
        let requirements = String::from_utf8(fixture("../tools/fixtures/requirements.txt")).unwrap();
        let script = String::from_utf8(fixture("../tools/fixtures/pdfjs_structure.mjs")).unwrap();
        let doctor = String::from_utf8(fixture("../tools/xtask/src/doctor.rs")).unwrap();
        assert_eq!(
            (
                facts["pdfjs_version"].as_str().unwrap(),
                facts["pypdfium2_version"].as_str().unwrap(),
                facts["qpdf_version"].as_str().unwrap()
            ),
            (
                pinned(&script, "const PINNED =").as_str(),
                pinned(&requirements, "pypdfium2==").as_str(),
                pinned(&doctor, "pub(crate) const QPDF_VERSION: &str =").as_str()
            )
        );
    }
}
