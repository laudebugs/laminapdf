//! An in-memory Typst `World` (experiments B and C): the main source, the files it reads, the
//! repository's Noto fonts and a fixed date. Nothing touches the disk or the clock.

use std::collections::HashMap;

use typst::diag::{FileError, FileResult, SourceResult, Warned};
use typst::foundations::{Bytes, Datetime, Duration};
use typst_layout::PagedDocument;
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};


pub struct MemoryWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    main: FileId,
    sources: HashMap<FileId, Source>,
    files: HashMap<FileId, Bytes>,
}

fn id(path: &str) -> FileId {
    // A spike: the paths are ours and valid.
    let vpath = VirtualPath::new(path).unwrap_or_else(|_| panic!("bad path {path}"));
    RootedPath::new(VirtualRoot::Project, vpath).intern()
}

impl MemoryWorld {
    /// A world whose main file is `main` (Typst markup) and that can read `files` by name.
    pub fn new(main: String, files: &[(&str, Vec<u8>)], fonts: &[Vec<u8>]) -> Self {
        let fonts: Vec<Font> =
            fonts.iter().flat_map(|bytes| Font::iter(Bytes::new(bytes.clone()))).collect();
        let main_id = id("main.typ");
        let mut sources = HashMap::new();
        sources.insert(main_id, Source::new(main_id, main));
        let files = files.iter().map(|(name, bytes)| (id(name), Bytes::new(bytes.clone()))).collect();
        Self {
            library: LazyHash::new(Library::builder().build()),
            book: LazyHash::new(FontBook::from_fonts(&fonts)),
            fonts,
            main: main_id,
            sources,
            files,
        }
    }

    /// Lays the document out.
    pub fn compile(&self) -> Warned<SourceResult<PagedDocument>> {
        typst::compile::<PagedDocument>(self)
    }
}

impl World for MemoryWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }
    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }
    fn main(&self) -> FileId {
        self.main
    }
    fn source(&self, id: FileId) -> FileResult<Source> {
        self.sources.get(&id).cloned().ok_or_else(|| FileError::NotFound(id.get().vpath().get_with_slash().into()))
    }
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files.get(&id).cloned().ok_or_else(|| FileError::NotFound(id.get().vpath().get_with_slash().into()))
    }
    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }
    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        Datetime::from_ymd(2026, 11, 8)
    }
}
