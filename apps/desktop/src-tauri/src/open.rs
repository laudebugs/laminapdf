//! Opening the PDFs the OS hands the app (todo D3a): a path on the command line, a second launch
//! that the single-instance plugin forwards, macOS's `RunEvent::Opened` (Finder, the Dock icon),
//! and files dropped on a window. Each source calls [`route_open`], and nothing else opens them.
//!
//! Requests go to the frontmost window, each file in a new tab there, or each in a new
//! window when the user chose that; a file open already shows its tab instead. Until a window's
//! webview has mounted, nothing could show a document, so [`route_open`] queues its paths in
//! [`PendingOpens`]; the webview listens for [`OPEN_EVENT`], then calls `take_pending_opens`, which
//! drains its queue and lets later requests through at once. Paths stay in the main process
//! (ADR 0008 §6): the webview gets the document's id and its file name.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError, mpsc};

use serde::Serialize;
use tauri::{AppHandle, Emitter as _, EventTarget, Manager as _};

use crate::DesktopError;
use crate::engine::DocumentMeta;

/// A document opened at the OS's request, or why it didn't open: sent to the window that shows it.
/// These event names are repeated in `src/engine/open.ts`.
pub(crate) const OPEN_EVENT: &str = "open://document";
/// Files are being dragged over the window: how many are PDFs, and how many aren't.
pub(crate) const DRAG_ENTER_EVENT: &str = "open://drag-enter";
/// The drag left the window, or the files were dropped.
pub(crate) const DRAG_LEAVE_EVENT: &str = "open://drag-leave";
/// Dropped files that aren't PDFs, by name: the window says so and opens none of them.
pub(crate) const REFUSED_EVENT: &str = "open://refused";

/// Whether `path` names a PDF by its extension, in any case.
fn is_pdf(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
}

/// The PDFs a command line names, in order and each once: `args[0]` (the program) is skipped, and
/// so are flags (`-psn_…` from old macOS, `Tauri`'s and `WebDriver`'s own) unless one is an existing
/// file (`-scan.pdf`); `file://` URLs become paths, and relative paths resolve against `cwd`, the
/// launching process's folder. Whether each other file exists is checked when it opens, so a missing
/// one is reported by name rather than dropped.
pub(crate) fn paths_from_args<A: AsRef<OsStr>>(args: &[A], cwd: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = Vec::new();
    for arg in args.iter().skip(1).map(AsRef::as_ref) {
        let text = arg.to_str();
        if text.is_some_and(|text| text.starts_with('-')) && !cwd.join(arg).is_file() {
            continue;
        }
        let path = match text.filter(|text| text.starts_with("file:")) {
            Some(url) => match tauri::Url::parse(url).ok().and_then(|url| url.to_file_path().ok()) {
                Some(path) => path,
                None => continue,
            },
            None => cwd.join(arg),
        };
        if is_pdf(&path) && !paths.contains(&path) {
            paths.push(path);
        }
    }
    paths
}

/// The macOS `RunEvent::Opened` URLs that are PDF files, as paths.
#[cfg(target_os = "macos")]
pub(crate) fn paths_from_urls(urls: &[tauri::Url]) -> Vec<PathBuf> {
    urls.iter()
        .filter_map(|url| url.to_file_path().ok())
        .filter(|path| is_pdf(path))
        .collect()
}

/// The OS's open requests that came before a window's webview could show them, by window.
#[derive(Debug, Default)]
pub(crate) struct PendingOpens(Mutex<BTreeMap<String, Queue>>);

#[derive(Debug, Default)]
struct Queue {
    /// Whether the webview has taken the queue: requests then open at once.
    ready: bool,
    paths: Vec<PathBuf>,
    /// Whether a request took this window while it showed nothing ([`PendingOpens::claim_empty`]).
    claimed: bool,
}

impl PendingOpens {
    fn queues(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, Queue>> {
        // The queues hold no invariant a panic could break halfway.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Queues `paths` while `window`'s webview isn't ready (`None`); once it is, hands them back
    /// to open now. The check and the push hold one lock, so a request can't slip between the
    /// webview's take and its readiness.
    pub(crate) fn offer(&self, window: &str, paths: Vec<PathBuf>) -> Option<Vec<PathBuf>> {
        let mut queues = self.queues();
        let queue = queues.entry(window.to_owned()).or_default();
        if queue.ready {
            return Some(paths);
        }
        queue.paths.extend(paths);
        None
    }

    /// Everything queued for `window`, in arrival order; from now on [`Self::offer`] hands its
    /// paths straight back.
    pub(crate) fn take(&self, window: &str) -> Vec<PathBuf> {
        let mut queues = self.queues();
        let queue = queues.entry(window.to_owned()).or_default();
        queue.ready = true;
        std::mem::take(&mut queue.paths)
    }

    /// Claims `window`, which shows nothing, for a file to open in, once: two requests close
    /// together can't both take it, as a request opens later than it is routed. False when
    /// paths already wait for it or it was claimed before (window labels aren't reused).
    pub(crate) fn claim_empty(&self, window: &str) -> bool {
        let mut queues = self.queues();
        let queue = queues.entry(window.to_owned()).or_default();
        if queue.claimed || !queue.paths.is_empty() {
            return false;
        }
        queue.claimed = true;
        true
    }

    /// `window` closed: its queue goes (a reused label would start not ready).
    pub(crate) fn closed(&self, window: &str) {
        self.queues().remove(window);
    }
}

/// Why a document didn't open, as a command's error reaches the webview: `{ kind, message }`.
/// [`DesktopError`] serialises the same way but can't be cloned, which emitting needs.
#[derive(Debug, Clone, Serialize)]
struct Failure {
    kind: &'static str,
    message: String,
}

impl From<&DesktopError> for Failure {
    fn from(error: &DesktopError) -> Self {
        Self {
            kind: error.kind(),
            message: error.to_string(),
        }
    }
}

/// What [`OPEN_EVENT`] carries: the document, or the file's name and why it didn't open. The
/// name is never the folder.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
enum Opened {
    Document { name: String, document: DocumentMeta },
    Failed { name: String, error: Failure },
}

/// One file to open, and the window that asked for it.
struct OpenJob {
    window: String,
    path: PathBuf,
}

/// Opens the OS's requests one at a time, in the order they came, on a thread of its own: an open
/// waits for the engine, and two opens on separate threads could finish in either order, so the
/// window could end up showing the earlier request.
pub(crate) struct OpenWorker(mpsc::Sender<OpenJob>);

impl OpenWorker {
    /// Starts the worker thread. It holds the app, whose state holds the sender, so it waits for
    /// jobs until the process exits; a job for a window that has closed opens nothing.
    ///
    /// # Errors
    /// The OS's error when it can't start a thread.
    pub(crate) fn start(app: AppHandle) -> std::io::Result<Self> {
        let (send, receive) = mpsc::channel::<OpenJob>();
        std::thread::Builder::new()
            .name("open-worker".to_owned())
            .spawn(move || {
                for job in receive {
                    open_and_tell(&app, &job);
                }
            })?;
        Ok(Self(send))
    }
}

/// Brings `window` to the front: unminimised, shown, focused. A second launch does this even with
/// no file, as launching a running app does on every OS.
pub(crate) fn bring_forward(app: &AppHandle, window: &str) {
    let Some(window) = app.get_webview_window(window) else {
        return;
    };
    for result in [window.unminimize(), window.show(), window.set_focus()] {
        if let Err(error) = result {
            tracing::debug!("could not bring the window forward: {error}");
        }
    }
}

/// The one way an OS request opens documents: each file in a new tab of `window` (the frontmost,
/// [`crate::windows::target_window`]), or each in a new window of its own when the user chose
/// that. A window's requests wait until its webview is ready, then open in order.
pub(crate) fn route_open(app: &AppHandle, window: &str, paths: Vec<PathBuf>) {
    if paths.is_empty() {
        return;
    }
    if app.state::<crate::prefs::Prefs>().get().open_in == crate::prefs::OpenIn::Windows {
        // The first file takes `window` if it shows nothing yet (a cold start, a window just made
        // for these files), so no window is left empty.
        let mut empty = Some(window).filter(|w| !holds_any(app, w) && app.state::<PendingOpens>().claim_empty(w));
        for path in paths {
            match empty
                .take()
                .map_or_else(|| crate::windows::new_window(app), |w| Ok(w.to_owned()))
            {
                Ok(label) => queue_or_open(app, &label, vec![path]),
                Err(error) => tracing::warn!("could not make a window for a document: {error}"),
            }
        }
        return;
    }
    queue_or_open(app, window, paths);
}

fn holds_any(app: &AppHandle, window: &str) -> bool {
    app.state::<std::sync::Arc<crate::engine::Engines>>().holds_any(window)
}

/// Opens `paths` in `window` now if its webview is ready, else when it is.
fn queue_or_open(app: &AppHandle, window: &str, paths: Vec<PathBuf>) {
    if let Some(paths) = app.state::<PendingOpens>().offer(window, paths) {
        open_now(app, window, paths);
    }
}

/// Opens each of `paths` in a tab of `window`, in order: the last becomes the shown tab.
fn open_now(app: &AppHandle, window: &str, paths: Vec<PathBuf>) {
    if paths.is_empty() {
        return;
    }
    bring_forward(app, window);
    let Some(worker) = app.try_state::<OpenWorker>() else {
        tracing::warn!("a document the OS asked for came before the app could open it");
        return;
    };
    for path in paths {
        let job = OpenJob {
            window: window.to_owned(),
            path,
        };
        if worker.0.send(job).is_err() {
            tracing::warn!("the worker that opens documents has stopped");
            return;
        }
    }
}

/// Opens one job's file in its window's engine and sends the window the document or the failure.
/// A window that has closed gets nothing: opening would start an engine that nothing stops.
fn open_and_tell(app: &AppHandle, job: &OpenJob) {
    if app.get_webview_window(&job.window).is_none() {
        tracing::debug!("a document the OS asked for came after its window closed");
        return;
    }
    let Some(payload) = opened(app, job) else {
        return;
    };
    if let Err(error) = app.emit_to(EventTarget::webview_window(&job.window), OPEN_EVENT, payload) {
        tracing::warn!("could not hand the window its document: {error}");
    }
}

/// Opens one job's file and says what came of it, by the file's name; `None` when the file was
/// open already and its tab came to the front instead.
fn opened(app: &AppHandle, job: &OpenJob) -> Option<Opened> {
    let name = file_name(&job.path);
    tracing::debug!(path = %job.path.display(), "opening a document the OS asked for");
    match open_file(app, &job.window, &job.path) {
        Ok(document) => Some(Opened::Document { name, document }),
        Err(DesktopError::AlreadyOpen) => None,
        Err(error) => {
            tracing::info!(kind = error.kind(), "a document the OS asked for didn't open");
            Some(Opened::Failed {
                name,
                error: Failure::from(&error),
            })
        }
    }
}

/// Opens `path` in `window`, recorded and watched; a missing file is an `io` error, which the
/// window reports by name.
fn open_file(app: &AppHandle, window: &str, path: &Path) -> Result<DocumentMeta, DesktopError> {
    if !path.is_file() {
        return Err(DesktopError::Io(std::io::ErrorKind::NotFound.into()));
    }
    crate::opened::open_path(app, window, path)
}

/// `path`'s last part, for the window's words: never its folder.
fn file_name(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
}

/// The PDFs among dropped `paths`, and the names of the rest.
fn split_drop(paths: &[PathBuf]) -> (Vec<PathBuf>, Vec<String>) {
    let (pdfs, others): (Vec<&PathBuf>, Vec<&PathBuf>) = paths.iter().partition(|path| is_pdf(path) && !path.is_dir());
    (
        pdfs.into_iter().cloned().collect(),
        others.into_iter().map(|path| file_name(path)).collect(),
    )
}

/// What [`DRAG_ENTER_EVENT`] carries: counts only, so the overlay can say what a drop will do.
#[derive(Debug, Clone, Copy, Serialize)]
struct Dragging {
    pdfs: usize,
    others: usize,
}

/// Files dragged over or dropped on `window`, as Tauri reports them. Rust opens dropped PDFs, so
/// the webview never needs their paths (Tauri still emits them to its `tauri://drag-*` events,
/// which the app doesn't listen to: ADR 0008 §6).
pub(crate) fn on_drag_drop(window: &tauri::Window, event: &tauri::DragDropEvent) {
    let target = EventTarget::webview_window(window.label());
    let sent = match event {
        tauri::DragDropEvent::Enter { paths, .. } => {
            let (pdfs, others) = split_drop(paths);
            let payload = Dragging {
                pdfs: pdfs.len(),
                others: others.len(),
            };
            window.emit_to(target, DRAG_ENTER_EVENT, payload)
        }
        tauri::DragDropEvent::Leave => window.emit_to(target, DRAG_LEAVE_EVENT, ()),
        tauri::DragDropEvent::Drop { paths, .. } => {
            let (pdfs, others) = split_drop(paths);
            let left = window.emit_to(target.clone(), DRAG_LEAVE_EVENT, ());
            if !others.is_empty()
                && let Err(error) = window.emit_to(target, REFUSED_EVENT, others)
            {
                tracing::warn!("could not tell the window a drop was refused: {error}");
            }
            route_open(window.app_handle(), window.label(), pdfs);
            left
        }
        _ => Ok(()),
    };
    if let Err(error) = sent {
        tracing::warn!("could not tell the window about a drag: {error}");
    }
}

/// Hands the webview the requests that came before it was ready, through [`OPEN_EVENT`], and lets
/// later ones through at once. The webview calls it once, after it listens for the event.
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "Tauri hands a command its window by value")]
pub(crate) fn take_pending_opens(window: tauri::WebviewWindow) {
    let paths = window.state::<PendingOpens>().take(window.label());
    open_now(window.app_handle(), window.label(), paths);
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{PendingOpens, is_pdf, paths_from_args, split_drop};

    fn cwd() -> PathBuf {
        if cfg!(windows) {
            PathBuf::from(r"C:\Users\ada\Documents")
        } else {
            PathBuf::from("/home/ada/Documents")
        }
    }

    #[test]
    fn paths_from_args_should_skip_the_program_and_flags() {
        let args = ["laminapdf", "-psn_0_12345", "--webdriver-port=4445", "a.pdf"];
        assert_eq!(paths_from_args(&args, &cwd()), vec![cwd().join("a.pdf")]);
    }

    #[test]
    fn paths_from_args_should_resolve_relative_paths_against_cwd() {
        let args = ["laminapdf", "reports/q3.pdf"];
        assert_eq!(
            paths_from_args(&args, &cwd()),
            vec![cwd().join("reports").join("q3.pdf")]
        );
    }

    #[test]
    fn paths_from_args_should_keep_absolute_paths() {
        let absolute = if cfg!(windows) {
            r"D:\scans\in.pdf"
        } else {
            "/tmp/in.pdf"
        };
        assert_eq!(
            paths_from_args(&["laminapdf", absolute], &cwd()),
            vec![PathBuf::from(absolute)]
        );
    }

    #[test]
    fn paths_from_args_should_turn_file_urls_into_paths() {
        let (url, path) = if cfg!(windows) {
            ("file:///C:/My%20Files/caf%C3%A9.pdf", r"C:\My Files\café.pdf")
        } else {
            (
                "file:///home/ada/My%20Files/caf%C3%A9.pdf",
                "/home/ada/My Files/café.pdf",
            )
        };
        assert_eq!(paths_from_args(&["laminapdf", url], &cwd()), vec![PathBuf::from(path)]);
    }

    #[cfg(unix)]
    #[test]
    fn paths_from_args_should_skip_urls_on_another_host() {
        // A network share's URL has no local path on Unix; Windows makes it a UNC path (below).
        let none: Vec<PathBuf> = Vec::new();
        assert_eq!(
            paths_from_args(&["laminapdf", "file://server/share/a.pdf"], &cwd()),
            none
        );
    }

    #[cfg(windows)]
    #[test]
    fn paths_from_args_should_turn_a_share_url_into_a_unc_path() {
        let expected = vec![PathBuf::from(r"\\server\share\a.pdf")];
        assert_eq!(
            paths_from_args(&["laminapdf", "file://server/share/a.pdf"], &cwd()),
            expected
        );
    }

    #[test]
    fn paths_from_args_should_keep_a_file_whose_name_starts_with_a_dash() {
        let dir = std::env::temp_dir().join(format!("laminapdf-open-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("-scan.pdf"), b"%PDF-1.7").unwrap();
        let paths = paths_from_args(&["laminapdf", "-scan.pdf", "-flag.pdf"], &dir);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(paths, vec![dir.join("-scan.pdf")]);
    }

    #[test]
    fn paths_from_args_should_keep_spaces_and_non_ascii_names() {
        let args = ["laminapdf", "Relevé de compte 2026.PDF", "報告書.pdf"];
        let expected = vec![cwd().join("Relevé de compte 2026.PDF"), cwd().join("報告書.pdf")];
        assert_eq!(paths_from_args(&args, &cwd()), expected);
    }

    #[test]
    fn paths_from_args_should_drop_files_that_are_not_pdfs() {
        let args = ["laminapdf", "notes.txt", "pdf", "archive.pdf.zip", "scan.pdf"];
        assert_eq!(paths_from_args(&args, &cwd()), vec![cwd().join("scan.pdf")]);
    }

    #[test]
    fn paths_from_args_should_name_each_file_once() {
        let args = ["laminapdf", "a.pdf", "b.pdf", "a.pdf"];
        assert_eq!(
            paths_from_args(&args, &cwd()),
            vec![cwd().join("a.pdf"), cwd().join("b.pdf")]
        );
    }

    #[cfg(windows)]
    #[test]
    fn paths_from_args_should_keep_long_windows_paths() {
        let long = format!(r"\\?\C:\{}\deep.pdf", "folder".repeat(50));
        assert_eq!(
            paths_from_args(&["laminapdf", long.as_str()], &cwd()),
            vec![PathBuf::from(&long)]
        );
    }

    #[cfg(windows)]
    #[test]
    fn paths_from_args_should_keep_drive_relative_paths_on_their_drive() {
        // `D:a.pdf` is relative to D:'s own folder, not to the launch folder on C:.
        assert_eq!(
            paths_from_args(&["laminapdf", "D:a.pdf"], &cwd()),
            vec![PathBuf::from("D:a.pdf")]
        );
    }

    #[cfg(unix)]
    #[test]
    fn paths_from_args_should_keep_names_that_are_not_unicode() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt as _;
        let name = OsString::from_vec(b"caf\xe9.pdf".to_vec());
        let args = [OsString::from("laminapdf"), name.clone()];
        assert_eq!(paths_from_args(&args, &cwd()), vec![cwd().join(name)]);
    }

    #[test]
    fn is_pdf_should_ignore_the_extensions_case() {
        assert!(is_pdf(Path::new("A.Pdf")));
    }

    #[test]
    fn offer_should_queue_until_the_webview_takes_the_queue() {
        let pending = PendingOpens::default();
        assert_eq!(pending.offer("main", vec![PathBuf::from("a.pdf")]), None);
    }

    #[test]
    fn take_should_return_queued_paths_in_arrival_order() {
        let pending = PendingOpens::default();
        // Cold-start argv first, then a second launch's paths, both before the webview is ready.
        let _ = pending.offer("main", vec![PathBuf::from("argv.pdf")]);
        let _ = pending.offer("main", vec![PathBuf::from("second.pdf"), PathBuf::from("third.pdf")]);
        let expected: Vec<PathBuf> = ["argv.pdf", "second.pdf", "third.pdf"].map(PathBuf::from).into();
        assert_eq!(pending.take("main"), expected);
    }

    #[test]
    fn take_should_drain_the_queue_once() {
        let pending = PendingOpens::default();
        let _ = pending.offer("main", vec![PathBuf::from("a.pdf")]);
        let _ = pending.take("main");
        assert_eq!(pending.take("main"), Vec::<PathBuf>::new());
    }

    #[test]
    fn offer_should_hand_paths_back_once_the_webview_is_ready() {
        let pending = PendingOpens::default();
        let _ = pending.take("main");
        assert_eq!(
            pending.offer("main", vec![PathBuf::from("b.pdf")]),
            Some(vec![PathBuf::from("b.pdf")])
        );
    }

    #[test]
    fn a_new_window_waits_for_its_own_webview() {
        let pending = PendingOpens::default();
        let _ = pending.take("main");
        assert_eq!(pending.offer("doc-1", vec![PathBuf::from("c.pdf")]), None);
        assert_eq!(pending.take("doc-1"), vec![PathBuf::from("c.pdf")]);
        // A closed window's label starts over, not ready.
        pending.closed("doc-1");
        assert_eq!(pending.offer("doc-1", vec![PathBuf::from("d.pdf")]), None);
    }

    #[test]
    fn split_drop_should_name_what_is_not_a_pdf() {
        let dropped = [PathBuf::from("/x/a.pdf"), PathBuf::from("/x/photo.jpg")];
        assert_eq!(
            split_drop(&dropped),
            (vec![PathBuf::from("/x/a.pdf")], vec!["photo.jpg".to_owned()])
        );
    }
}
