//! The main process's side of the engine process (ADR 0008 §2): one engine per window, the files
//! its documents come from, and the passwords kept so a restarted engine can unlock them again.
//!
//! [`Engines`] knows nothing of Tauri: the app gives it a way to start engines and a
//! [`Notify`] for the webview, so its tests run against the protocol crate's fake engine.

mod bytesource;
mod documents;
mod secret;
mod tiles;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use laminapdf_desktop_protocol::supervisor::{Config, Event, Spawner, Supervisor, SupervisorError};
use laminapdf_desktop_protocol::{
    DocId, FailureKind, FromEngine, PageDimensions, ReqId, TileFormat, TileRequest, ToEngine,
};
use serde::Serialize;

pub(crate) use documents::DocumentRegistry;
use secret::SessionPassword;
pub(crate) use tiles::{Tile, TileQuery, UrlError as TileUrlError, parse as parse_tile, respond as tile_response};

use crate::DesktopError;

/// The event a window gets after its engine was replaced: show "document process restarted"
/// and ask for the visible tiles again.
pub(crate) const RESTARTED_EVENT: &str = "engine://restarted";

/// The event a window gets when its engine crashed too often and stays down.
pub(crate) const FAILED_EVENT: &str = "engine://failed";

/// What the webview learns about an open document.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DocumentMeta {
    /// The id the webview names it by.
    pub doc: u32,
    /// The file's name, for the window's title: never its folder (paths stay in the main process).
    pub name: String,
    /// The file's size in bytes.
    pub size: u64,
    /// The number of pages (0 while it waits for a password).
    pub page_count: u32,
    /// Whether the file is encrypted.
    pub encrypted: bool,
    /// Whether it waits for a password.
    pub needs_password: bool,
    /// Whether reading it needed repairs (ADR 0003 warnings).
    pub repaired: bool,
    /// Each page's size in points and rotation (empty while locked).
    pub pages: Vec<PageMeta>,
}

/// One page's displayed size in points and `/Rotate`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub(crate) struct PageMeta {
    pub width: f32,
    pub height: f32,
    pub rotate: u16,
}

impl From<&PageDimensions> for PageMeta {
    fn from(size: &PageDimensions) -> Self {
        Self {
            width: size.width,
            height: size.height,
            rotate: size.rotate,
        }
    }
}

/// [`RESTARTED_EVENT`]'s payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Restarted {
    pub window: String,
    pub docs: Vec<RestartedDoc>,
}

/// A document after a restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct RestartedDoc {
    pub doc: u32,
    pub state: Reopened,
}

/// How a document came back after a restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Reopened {
    /// Open again, unlocked with its kept password if it had one.
    Open,
    /// Open, but waiting for its password (its kept one was refused, or it never had one).
    Locked,
    /// Closed: it failed to reopen, or the new engine died reopening it, so it can't take the
    /// window's other documents down with it on every restart.
    Closed,
}

/// [`FAILED_EVENT`]'s payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Failed {
    pub window: String,
}

/// Tells one window about its engine. The app sends Tauri events; tests record them.
pub(crate) trait Notify: Send + Sync {
    fn restarted(&self, payload: Restarted);
    fn failed(&self, payload: Failed);
}

/// How [`Engines`] starts an engine.
pub(crate) type Launcher = Arc<dyn Fn() -> Box<dyn Spawner> + Send + Sync>;

/// Every window's engine and every open document.
pub(crate) struct Engines {
    registry: Arc<DocumentRegistry>,
    windows: Mutex<BTreeMap<String, Supervisor>>,
    /// Each window's tiles in flight: the viewer's ticket to the engine's request, for `cancel_tiles`.
    tickets: Mutex<BTreeMap<(String, u64), ReqId>>,
    launcher: Launcher,
    config: Config,
    notify: Arc<dyn Notify>,
    this: Weak<Self>,
}

impl Engines {
    pub(crate) fn new(launcher: Launcher, config: Config, notify: Arc<dyn Notify>) -> Arc<Self> {
        Arc::new_cyclic(|this| Self {
            registry: Arc::new(DocumentRegistry::default()),
            windows: Mutex::new(BTreeMap::new()),
            tickets: Mutex::new(BTreeMap::new()),
            launcher,
            config,
            notify,
            this: Weak::clone(this),
        })
    }

    /// Opens `path` in `window`'s engine: its page count and page sizes, computed there.
    pub(crate) fn open(&self, window: &str, path: &Path) -> Result<DocumentMeta, DesktopError> {
        let (doc, len) = self.registry.register(path, window)?;
        let opened = self.open_in(window, doc, len);
        if opened.is_err() {
            self.registry.remove(doc);
        }
        opened
    }

    fn open_in(&self, window: &str, doc: DocId, len: u64) -> Result<DocumentMeta, DesktopError> {
        let engine = self.engine(window)?;
        let (page_count, encrypted, needs_password, repaired) =
            match ask(&engine, |req| ToEngine::OpenDocument { req, doc, len })? {
                FromEngine::Opened {
                    page_count,
                    encrypted,
                    needs_password,
                    repaired,
                    ..
                } => (page_count, encrypted, needs_password, repaired),
                other => return Err(unexpected(&other)),
            };
        let pages = if needs_password {
            Vec::new()
        } else {
            self.page_info(window, doc)?
        };
        Ok(DocumentMeta {
            doc: doc.0,
            name: self.registry.name_of(doc).unwrap_or_default(),
            size: len,
            page_count,
            encrypted,
            needs_password,
            repaired,
            pages,
        })
    }

    /// Reopens `window`'s document `old` from `path`, whose file changed on disk: the new file opens
    /// as a new document, unlocked with `old`'s kept password if it had one, and only then does
    /// `old` close. On any failure `old` stays open as it was and the new one is closed.
    pub(crate) fn reload(&self, window: &str, old: DocId, path: &Path) -> Result<DocumentMeta, DesktopError> {
        self.check(window, old)?;
        let password = self.registry.password_of(old);
        let mut opened = self.open(window, path)?;
        if let (true, Some(password)) = (opened.needs_password, password) {
            match self.unlock(window, DocId(opened.doc), password.expose()) {
                Ok(unlocked) => opened = unlocked,
                Err(error) => {
                    let _ = self.close(window, DocId(opened.doc));
                    return Err(error);
                }
            }
        }
        if let Err(error) = self.close(window, old) {
            let _ = self.close(window, DocId(opened.doc));
            return Err(error);
        }
        Ok(opened)
    }

    /// Draws one tile of `window`'s document: the engine's straight RGBA, checked against the
    /// size asked for (the engine reads untrusted files, so its answers are untrusted too).
    pub(crate) fn render_tile(&self, window: &str, query: &TileQuery) -> Result<Tile, DesktopError> {
        self.check(window, query.doc)?;
        let engine = self.engine(window)?;
        let key = (window.to_owned(), query.ticket);
        let mut sent = None;
        let reply = engine.request(|req| {
            sent = Some(req);
            self.tickets().insert(key.clone(), req);
            ToEngine::RenderTile(TileRequest {
                req,
                doc: query.doc,
                page: query.page,
                scale: query.scale,
                rotation: query.rotation,
                appearance: query.appearance,
                x: query.x,
                y: query.y,
                w: query.w,
                h: query.h,
                format: TileFormat::Rgba8,
                priority: query.priority,
            })
        });
        // Only this render's entry: a reloaded webview numbers its tickets from 1 again, and a newer
        // tile may have taken the key.
        {
            let mut tickets = self.tickets();
            if sent.is_some() && tickets.get(&key).copied() == sent {
                tickets.remove(&key);
            }
        }
        match reply {
            Ok((
                FromEngine::Tile {
                    w,
                    h,
                    format: TileFormat::Rgba8,
                    ..
                },
                pixels,
            )) if w <= query.w && h <= query.h && pixels.len() as u64 == u64::from(w) * u64::from(h) * 4 => {
                Ok(Tile { w, h, pixels })
            }
            Ok((FromEngine::NeedsPassword { .. }, _)) => Err(needs_password()),
            Ok((FromEngine::Error { kind, .. }, _)) => Err(engine_error(kind)),
            Ok((other, _)) => Err(unexpected(&other)),
            Err(error) => Err(supervisor_error(&error)),
        }
    }

    /// Drops the tiles `tickets` names that `window` still waits for: the engine skips them, or
    /// answers them cancelled. A ticket already answered, or not yet sent to the engine, is
    /// ignored: such a tile is drawn anyway, and the client, whose render was aborted, drops it.
    pub(crate) fn cancel_tiles(&self, window: &str, tickets: &[u64]) {
        let Some(engine) = self.windows().get(window).cloned() else {
            return;
        };
        let reqs: Vec<ReqId> = {
            let waiting = self.tickets();
            tickets
                .iter()
                .filter_map(|ticket| waiting.get(&(window.to_owned(), *ticket)).copied())
                .collect()
        };
        for req in reqs {
            engine.send(&ToEngine::Cancel { req });
        }
    }

    /// Every page's size of `window`'s document `doc`.
    pub(crate) fn page_info(&self, window: &str, doc: DocId) -> Result<Vec<PageMeta>, DesktopError> {
        self.check(window, doc)?;
        match ask(&self.engine(window)?, |req| ToEngine::GetPageInfo { req, doc })? {
            FromEngine::PageInfo { sizes, .. } => Ok(sizes.iter().map(PageMeta::from).collect()),
            FromEngine::NeedsPassword { .. } => Err(needs_password()),
            other => Err(unexpected(&other)),
        }
    }

    /// Unlocks `doc` with the password the user typed, and keeps the password while it stays
    /// open. The password prompt's command calls it; the restart path reuses the kept password.
    pub(crate) fn unlock(&self, window: &str, doc: DocId, password: &str) -> Result<DocumentMeta, DesktopError> {
        self.check(window, doc)?;
        let kept = SessionPassword::new(password);
        let engine = self.engine(window)?;
        let unlocked = ask(&engine, |req| ToEngine::UnlockDocument {
            req,
            doc,
            password: kept.to_password(),
        })?;
        let (page_count, repaired) = match unlocked {
            FromEngine::Unlocked {
                page_count, repaired, ..
            } => (page_count, repaired),
            other => return Err(unexpected(&other)),
        };
        self.registry.keep_password(doc, kept);
        Ok(DocumentMeta {
            doc: doc.0,
            name: self.registry.name_of(doc).unwrap_or_default(),
            size: self.registry.len_of(doc).unwrap_or(0),
            page_count,
            encrypted: true,
            needs_password: false,
            repaired,
            pages: self.page_info(window, doc)?,
        })
    }

    /// The document already open from `path`, and its window.
    pub(crate) fn open_from(&self, path: &Path) -> Option<(DocId, String)> {
        self.registry.open_from(path)
    }

    /// `doc`'s file, for the watcher; never sent to a webview or an engine.
    pub(crate) fn path_of(&self, doc: DocId) -> Option<std::path::PathBuf> {
        self.registry.path_of(doc)
    }

    /// Moves `doc` from window `from` to window `to` (a tab moved there): the same document, file
    /// and kept password, opened in `to`'s engine. `to` becomes the owner first, so its engine may
    /// read the file; the document opens there and, with a kept password, unlocks, so a moved
    /// tab never asks for it again; only then does `from`'s engine forget it. That close is the
    /// engine's alone: the registry keeps the entry and its password. On any failure `from`
    /// owns it again, `to`'s engine forgets it, and the tab stays where it was.
    pub(crate) fn move_doc(&self, doc: DocId, from: &str, to: &str) -> Result<DocumentMeta, DesktopError> {
        self.check(from, doc)?;
        let len = self.registry.len_of(doc).ok_or(DesktopError::UnknownDocument)?;
        let password = self.registry.password_of(doc);
        self.registry.set_window(doc, to);
        let moved = self
            .open_in(to, doc, len)
            .and_then(|opened| match (opened.needs_password, password) {
                (true, Some(password)) => self.unlock(to, doc, password.expose()),
                _ => Ok(opened),
            });
        let (keep, drop_from) = match &moved {
            Ok(_) => (to, from),
            Err(_) => (from, to),
        };
        self.registry.set_window(doc, keep);
        if let Some(engine) = self.windows().get(drop_from).cloned() {
            engine.send(&ToEngine::CloseDocument { doc });
        }
        moved
    }

    /// Closes `doc`: the engine forgets it, the file closes and its password is wiped.
    pub(crate) fn close(&self, window: &str, doc: DocId) -> Result<(), DesktopError> {
        self.check(window, doc)?;
        self.registry.remove(doc);
        if let Some(engine) = self.windows().get(window).cloned() {
            engine.send(&ToEngine::CloseDocument { doc });
        }
        Ok(())
    }

    /// `window` closed: its documents close and its engine stops.
    pub(crate) fn close_window(&self, window: &str) {
        self.registry.remove_window(window);
        let engine = self.windows().remove(window);
        if let Some(engine) = engine {
            engine.shutdown();
        }
    }

    /// The app is exiting: every engine stops and every password is wiped.
    pub(crate) fn shutdown(&self) {
        let windows: Vec<String> = self.windows().keys().cloned().collect();
        for window in windows {
            self.close_window(&window);
        }
    }

    /// Kills `window`'s engine as a crash would (the e2e tests' `debug_kill_engine`).
    #[cfg_attr(
        not(any(test, feature = "e2e")),
        expect(dead_code, reason = "only e2e builds and tests kill engines")
    )]
    pub(crate) fn kill(&self, window: &str) {
        if let Some(engine) = self.windows().get(window).cloned() {
            engine.kill();
        }
    }

    /// `window`'s engine, started on first use, and again after one that failed for good, so the
    /// window's Reopen gets a fresh engine instead of the dead one.
    fn engine(&self, window: &str) -> Result<Supervisor, DesktopError> {
        let failed = {
            let mut windows = self.windows();
            match windows.get(window) {
                Some(engine) if !engine.has_failed() => return Ok(engine.clone()),
                Some(_) => windows.remove(window),
                None => None,
            }
        };
        if let Some(failed) = failed {
            failed.shutdown();
        }
        // Started without the map's lock: starting waits for the engine's `Ready`.
        let this = Weak::clone(&self.this);
        let label = window.to_owned();
        let engine = Supervisor::start(
            (self.launcher)(),
            Arc::new(self.registry.files_of(window)),
            self.config.clone(),
            move |engine, event| {
                if let Some(engines) = this.upgrade() {
                    engines.on_event(&label, engine, event);
                }
            },
        )
        .map_err(|error| supervisor_error(&error))?;
        let mut windows = self.windows();
        if let Some(raced) = windows.get(window) {
            // Another command started one meanwhile: keep that one.
            let raced = raced.clone();
            drop(windows);
            engine.shutdown();
            return Ok(raced);
        }
        windows.insert(window.to_owned(), engine.clone());
        Ok(engine)
    }

    fn on_event(&self, window: &str, engine: &Supervisor, event: Event) {
        match event {
            Event::Restarted { pid } => {
                tracing::warn!(window, pid, "document process restarted");
                let docs = self.reopen(window, engine);
                self.notify.restarted(Restarted {
                    window: window.to_owned(),
                    docs,
                });
            }
            Event::Failed => {
                tracing::error!(window, "document process failed repeatedly; not restarting it");
                self.notify.failed(Failed {
                    window: window.to_owned(),
                });
            }
            Event::Log(line) => tracing::info!(target: "laminapdf_desktop::engine", window, "{line}"),
        }
    }

    /// Reopens `window`'s documents in its new engine, unlocking each with its kept password.
    /// A refused password is wiped and its document reported locked, so the UI asks again. A
    /// document that fails to reopen is closed; if the engine died reopening it, the documents
    /// after it wait for the next restart, which reopens them without it.
    fn reopen(&self, window: &str, engine: &Supervisor) -> Vec<RestartedDoc> {
        let mut docs = Vec::new();
        for reopen in self.registry.reopen_list(window) {
            let doc = reopen.doc;
            let state = match self.reopen_one(engine, reopen) {
                Ok(state) => state,
                Err(error) => {
                    tracing::warn!(window, doc = doc.0, "a document did not reopen; closing it: {error}");
                    self.registry.remove(doc);
                    docs.push(RestartedDoc {
                        doc: doc.0,
                        state: Reopened::Closed,
                    });
                    if matches!(error.kind(), "engine-restarted" | "engine-timeout" | "engine-failed") {
                        break;
                    }
                    continue;
                }
            };
            docs.push(RestartedDoc { doc: doc.0, state });
        }
        docs
    }

    /// Reopens one document and unlocks it with its kept password.
    fn reopen_one(&self, engine: &Supervisor, reopen: documents::Reopen) -> Result<Reopened, DesktopError> {
        let doc = reopen.doc;
        let needs_password = match ask(engine, |req| ToEngine::OpenDocument {
            req,
            doc,
            len: reopen.len,
        })? {
            FromEngine::Opened { needs_password, .. } => needs_password,
            other => return Err(unexpected(&other)),
        };
        let password = match (needs_password, reopen.password) {
            (false, _) => return Ok(Reopened::Open),
            (true, None) => return Ok(Reopened::Locked),
            (true, Some(password)) => password,
        };
        match ask(engine, |req| ToEngine::UnlockDocument { req, doc, password }) {
            Ok(FromEngine::Unlocked { .. }) => Ok(Reopened::Open),
            Err(DesktopError::Engine {
                kind: "wrong-password", ..
            }) => {
                tracing::warn!(doc = doc.0, "the kept password was refused; asking again");
                self.registry.forget_password(doc);
                Ok(Reopened::Locked)
            }
            Ok(other) => Err(unexpected(&other)),
            Err(error) => Err(error),
        }
    }

    /// Fails unless `doc` is open in `window`.
    fn check(&self, window: &str, doc: DocId) -> Result<(), DesktopError> {
        if self.registry.owns(window, doc) {
            Ok(())
        } else {
            Err(DesktopError::UnknownDocument)
        }
    }

    fn windows(&self) -> MutexGuard<'_, BTreeMap<String, Supervisor>> {
        self.windows.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn tickets(&self) -> MutexGuard<'_, BTreeMap<(String, u64), ReqId>> {
        self.tickets.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Sends a request and turns the engine's `Error` reply into a [`DesktopError`].
fn ask(engine: &Supervisor, make: impl FnOnce(ReqId) -> ToEngine) -> Result<FromEngine, DesktopError> {
    match engine.request(make) {
        Ok((FromEngine::Error { kind, .. }, _)) => Err(engine_error(kind)),
        Ok((reply, _)) => Ok(reply),
        Err(error) => Err(supervisor_error(&error)),
    }
}

fn needs_password() -> DesktopError {
    DesktopError::Engine {
        kind: "needs-password",
        message: "this document needs its password".into(),
    }
}

fn engine_error(kind: FailureKind) -> DesktopError {
    let (kind, message) = match kind {
        FailureKind::Io => ("io", "the file could not be read"),
        FailureKind::Timeout => ("timeout", "reading the file took too long"),
        FailureKind::WrongPassword => ("wrong-password", "the password doesn't open this document"),
        FailureKind::Invalid => ("invalid", "this file isn't a PDF Lamina PDF can read"),
        FailureKind::Limit => ("limit", "this document is too large or complex to open"),
        FailureKind::UnknownDocument => ("unknown-document", "no such document"),
        FailureKind::PageOutOfRange => ("page-out-of-range", "no such page"),
        FailureKind::TooLarge => ("too-large", "the tile is too large"),
        FailureKind::Cancelled => ("cancelled", "cancelled"),
        FailureKind::RenderUnavailable => ("render-unavailable", "rendering isn't available"),
        FailureKind::RenderFailed => ("render-failed", "the page could not be drawn"),
        FailureKind::Protocol => ("protocol", "the document process misbehaved"),
    };
    DesktopError::Engine {
        kind,
        message: message.into(),
    }
}

fn supervisor_error(error: &SupervisorError) -> DesktopError {
    let kind = match error {
        SupervisorError::Failed => "engine-failed",
        SupervisorError::Lost => "engine-restarted",
        SupervisorError::Timeout => "engine-timeout",
        _ => "engine-start",
    };
    DesktopError::Engine {
        kind,
        message: error.to_string(),
    }
}

fn unexpected(reply: &FromEngine) -> DesktopError {
    tracing::warn!("unexpected reply from the engine: {reply:?}");
    engine_error(FailureKind::Protocol)
}

#[cfg(test)]
mod tests;
