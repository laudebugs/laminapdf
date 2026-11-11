//! The main process's side of the engine's fonts (ADR 0008 §5, fonts): the sandboxed engine
//! can't open the system's font files, so the main process lists their faces and serves each
//! face's bytes by id ([`FontServer`]). The engine never names a path, so a compromised engine can
//! read only the font faces listed here.
//!
//! The faces come from the OS's font folders, read header by header: macOS's three, Windows'
//! system and per-user ones, and on Linux PDFium's own list plus the `<dir>`s fontconfig's
//! configuration names.

mod sfnt;

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use laminapdf_desktop_protocol::FontFace;
use laminapdf_desktop_protocol::supervisor::FontServer;

use self::sfnt::Table;

/// The largest face served: a frame carries at most 64 MiB, and no real face comes near this.
const MAX_FACE: u64 = 48 << 20;

/// The most faces listed: a system with every Noto and CJK font has a few thousand.
const MAX_FACES: usize = 8_192;

/// The most bytes of names in the list, which goes to every engine in one frame (64 MiB at most):
/// a crafted font file can't push the list past it and stop engines from starting.
const MAX_LIST_BYTES: usize = 4 << 20;

/// Folders nested deeper than this aren't searched.
const MAX_DEPTH: usize = 6;

/// The system's font faces, listed once.
#[derive(Debug, Default)]
pub struct SystemFonts {
    faces: Vec<Listed>,
    /// The names' bytes so far, against [`MAX_LIST_BYTES`].
    names: usize,
    /// Faces left out past [`MAX_FACES`] or [`MAX_LIST_BYTES`].
    pub dropped: usize,
    /// How long listing took.
    pub took: Duration,
}

#[derive(Debug)]
struct Listed {
    face: FontFace,
    path: PathBuf,
    offset: u32,
    tables: Vec<Table>,
}

impl SystemFonts {
    /// Lists the faces in this OS's font folders.
    #[must_use]
    pub fn scan() -> Self {
        Self::scan_folders(&folders())
    }

    /// Lists the faces in `folders` and the folders under them.
    #[must_use]
    pub fn scan_folders(folders: &[PathBuf]) -> Self {
        let started = Instant::now();
        let mut fonts = Self::default();
        let mut files = Vec::new();
        for folder in folders {
            collect(folder, 0, &mut files);
        }
        files.sort();
        files.dedup();
        for path in files {
            // An unreadable or malformed file is skipped: the engine falls back for its fonts.
            let _ = fonts.add_file(&path);
        }
        fonts.took = started.elapsed();
        fonts
    }

    /// The number of faces listed.
    #[must_use]
    pub fn len(&self) -> usize {
        self.faces.len()
    }

    /// Whether no face was found.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }

    fn add_file(&mut self, path: &Path) -> io::Result<()> {
        let file = File::open(path)?;
        let mut read = |offset: u64, buf: &mut [u8]| read_at(&file, offset, buf);
        for offset in sfnt::faces(&mut read)? {
            let Ok(tables) = sfnt::tables(&mut read, offset) else {
                continue;
            };
            let size: u64 = tables.iter().map(|t| u64::from(t.len)).sum();
            if size > MAX_FACE {
                continue;
            }
            let Ok(described) = sfnt::describe(&mut read, &tables) else {
                continue;
            };
            // A face's share of the encoded list: its names and about 16 bytes of numbers.
            let bytes = described.family.len() + described.style.len() + 16;
            if self.faces.len() >= MAX_FACES || self.names + bytes > MAX_LIST_BYTES {
                self.dropped += 1;
                continue;
            }
            self.names += bytes;
            let id = u32::try_from(self.faces.len()).map_err(|_| io::Error::other("too many faces"))?;
            self.faces.push(Listed {
                face: FontFace {
                    id,
                    family: described.family,
                    style: described.style,
                    weight: described.weight,
                    italic: described.italic,
                    charsets: described.charsets,
                },
                path: path.to_path_buf(),
                offset,
                tables,
            });
        }
        Ok(())
    }
}

impl FontServer for SystemFonts {
    fn faces(&self) -> Vec<FontFace> {
        self.faces.iter().map(|listed| listed.face.clone()).collect()
    }

    fn dropped(&self) -> usize {
        self.dropped
    }

    fn read(&self, font: u32) -> io::Result<Vec<u8>> {
        let listed = self
            .faces
            .get(font as usize)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no such font face"))?;
        let file = File::open(&listed.path)?;
        let mut read = |offset: u64, buf: &mut [u8]| read_at(&file, offset, buf);
        sfnt::extract(&mut read, listed.offset, &listed.tables)
    }
}

/// [`SystemFonts`] listed on a thread of its own, so the app's start doesn't wait for the scan; the
/// first engine does, if it comes before the scan ends.
#[derive(Debug)]
pub struct BackgroundFonts {
    list: Arc<OnceLock<SystemFonts>>,
    scan: Mutex<Option<JoinHandle<()>>>,
}

impl BackgroundFonts {
    /// Starts listing this OS's fonts.
    #[must_use]
    pub fn start() -> Self {
        let list = Arc::new(OnceLock::new());
        let filled = Arc::clone(&list);
        let scan = thread::Builder::new()
            .name("system-fonts".into())
            .spawn(move || {
                let _ = filled.set(SystemFonts::scan());
            })
            .ok();
        Self {
            list,
            scan: Mutex::new(scan),
        }
    }

    /// The list, once the scan has ended; an empty one if the scan couldn't run.
    fn list(&self) -> Option<&SystemFonts> {
        let scan = self.scan.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(scan) = scan {
            let _ = scan.join();
        }
        self.list.get()
    }
}

impl FontServer for BackgroundFonts {
    fn faces(&self) -> Vec<FontFace> {
        self.list().map(FontServer::faces).unwrap_or_default()
    }

    fn dropped(&self) -> usize {
        self.list().map_or(0, FontServer::dropped)
    }

    fn read(&self, font: u32) -> io::Result<Vec<u8>> {
        self.list()
            .ok_or_else(|| io::Error::other("the font scan didn't run"))?
            .read(font)
    }
}

/// Fills `buf` from `offset`, by positioned read.
fn read_at(file: &File, offset: u64, buf: &mut [u8]) -> io::Result<()> {
    #[cfg(unix)]
    std::os::unix::fs::FileExt::read_exact_at(file, buf, offset)?;
    #[cfg(windows)]
    {
        let mut filled = 0;
        while filled < buf.len() {
            let Some(rest) = buf.get_mut(filled..) else { break };
            let n = std::os::windows::fs::FileExt::seek_read(file, rest, offset + filled as u64)?;
            if n == 0 {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            filled += n;
        }
    }
    Ok(())
}

/// Every font file under `folder`.
fn collect(folder: &Path, depth: usize, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else { continue };
        if kind.is_dir() && depth < MAX_DEPTH {
            collect(&path, depth + 1, files);
        } else if is_font(&path) {
            files.push(path);
        }
    }
}

fn is_font(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| {
        ["ttf", "otf", "ttc", "otc"]
            .iter()
            .any(|font| ext.eq_ignore_ascii_case(font))
    })
}

fn home() -> Option<PathBuf> {
    std::env::home_dir()
}

/// This OS's font folders.
fn folders() -> Vec<PathBuf> {
    let mut folders = Vec::new();
    if cfg!(target_os = "macos") {
        folders.extend(["/System/Library/Fonts", "/Library/Fonts"].map(PathBuf::from));
        folders.extend(home().map(|home| home.join("Library/Fonts")));
    } else if cfg!(windows) {
        let windir = std::env::var_os("WINDIR").map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
        folders.push(windir.join("Fonts"));
        folders.extend(std::env::var_os("LOCALAPPDATA").map(|dir| PathBuf::from(dir).join(r"Microsoft\Windows\Fonts")));
    } else {
        // PDFium's own folders (fx_linux_impl.cpp), then fontconfig's.
        folders.extend(
            [
                "/usr/share/fonts",
                "/usr/share/X11/fonts/Type1",
                "/usr/share/X11/fonts/TTF",
                "/usr/local/share/fonts",
            ]
            .map(PathBuf::from),
        );
        folders.extend(fontconfig_folders());
    }
    folders
}

/// The `<dir>` entries of fontconfig's configuration (`/etc/fonts/fonts.conf` and `conf.d`), with
/// `~` and `prefix="xdg"` resolved. Only folders: fontconfig's other rules (aliases, rejects)
/// don't change which files exist.
fn fontconfig_folders() -> Vec<PathBuf> {
    let mut configs = vec![PathBuf::from("/etc/fonts/fonts.conf")];
    if let Ok(entries) = std::fs::read_dir("/etc/fonts/conf.d") {
        configs.extend(entries.flatten().map(|entry| entry.path()));
    }
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| home().map(|home| home.join(".local/share")));
    let mut folders = Vec::new();
    for config in configs {
        let Ok(text) = std::fs::read_to_string(&config) else {
            continue;
        };
        for element in text.split("<dir").skip(1) {
            let Some((attributes, rest)) = element.split_once('>') else {
                continue;
            };
            let Some((dir, _)) = rest.split_once("</dir>") else {
                continue;
            };
            let dir = dir.trim();
            let folder = if attributes.contains("prefix=\"xdg\"") {
                data_home.as_ref().map(|base| base.join(dir))
            } else if let Some(rest) = dir.strip_prefix("~/") {
                home().map(|home| home.join(rest))
            } else if Path::new(dir).is_absolute() {
                Some(PathBuf::from(dir))
            } else {
                // fontconfig's rule: a relative <dir> is relative to the file that names it.
                config.parent().map(|parent| parent.join(dir))
            };
            folders.extend(folder);
        }
    }
    folders
}
