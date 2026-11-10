//! The main process's side of the engine process (ADR 0008 §2): one engine per window, the files
//! its documents come from, and the passwords kept so a restarted engine can unlock them again.
//!
//! [`Engines`] knows nothing of Tauri: the app gives it a way to start engines and a
//! [`Notify`] for the webview, so its tests run against the protocol crate's fake engine.

mod bytesource;
mod documents;
mod secret;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use laminapdf_desktop_protocol::supervisor::{Config, Event, Spawner, Supervisor, SupervisorError};
use laminapdf_desktop_protocol::{DocId, ErrorKind, FromEngine, PageSize, ToEngine};
use serde::Serialize;

pub(crate) use documents::DocumentRegistry;
use secret::SessionPassword;

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

impl From<&PageSize> for PageMeta {
    fn from(size: &PageSize) -> Self {
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

/// A document after a restart: reopened, and whether it waits for a password again (its kept
/// password was refused, or it never had one).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct RestartedDoc {
    pub doc: u32,
    pub locked: bool,
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
            page_count,
            encrypted,
            needs_password,
            repaired,
            pages,
        })
    }

    /// Every page's size of `window`'s document `doc`.
    pub(crate) fn page_info(&self, window: &str, doc: DocId) -> Result<Vec<PageMeta>, DesktopError> {
        self.check(window, doc)?;
        match ask(&self.engine(window)?, |req| ToEngine::GetPageInfo { req, doc })? {
            FromEngine::PageInfo { sizes, .. } => Ok(sizes.iter().map(PageMeta::from).collect()),
            FromEngine::NeedsPassword { .. } => Err(DesktopError::Engine {
                kind: "needs-password",
                message: "this document needs its password".into(),
            }),
            other => Err(unexpected(&other)),
        }
    }

    /// Unlocks `doc` with the password the user typed, and keeps the password while it stays
    /// open. D2d's password prompt calls this; the restart path reuses the kept password.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "D2d's `unlock_document` command calls it with the user's password"
        )
    )]
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
            page_count,
            encrypted: true,
            needs_password: false,
            repaired,
            pages: self.page_info(window, doc)?,
        })
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

    /// `window`'s engine, started on first use.
    fn engine(&self, window: &str) -> Result<Supervisor, DesktopError> {
        if let Some(engine) = self.windows().get(window) {
            return Ok(engine.clone());
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
    /// A refused password is wiped and its document reported locked, so the UI asks again.
    fn reopen(&self, window: &str, engine: &Supervisor) -> Vec<RestartedDoc> {
        let mut docs = Vec::new();
        for reopen in self.registry.reopen_list(window) {
            let doc = reopen.doc;
            let opened = ask(engine, |req| ToEngine::OpenDocument {
                req,
                doc,
                len: reopen.len,
            });
            let locked = match (opened, reopen.password) {
                (
                    Ok(FromEngine::Opened {
                        needs_password: false, ..
                    }),
                    _,
                ) => false,
                (
                    Ok(FromEngine::Opened {
                        needs_password: true, ..
                    }),
                    Some(password),
                ) => {
                    let unlocked = ask(engine, |req| ToEngine::UnlockDocument { req, doc, password });
                    let refused = !matches!(unlocked, Ok(FromEngine::Unlocked { .. }));
                    if refused {
                        tracing::warn!(window, doc = doc.0, "the kept password was refused; asking again");
                        self.registry.forget_password(doc);
                    }
                    refused
                }
                (
                    Ok(FromEngine::Opened {
                        needs_password: true, ..
                    }),
                    None,
                ) => true,
                (failed, _) => {
                    tracing::warn!(window, doc = doc.0, "a document did not reopen: {failed:?}");
                    true
                }
            };
            docs.push(RestartedDoc { doc: doc.0, locked });
        }
        docs
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
}

/// Sends a request and turns the engine's `Error` reply into a [`DesktopError`].
fn ask(
    engine: &Supervisor,
    make: impl FnOnce(laminapdf_desktop_protocol::ReqId) -> ToEngine,
) -> Result<FromEngine, DesktopError> {
    match engine.request(make) {
        Ok((FromEngine::Error { kind, .. }, _)) => Err(engine_error(kind)),
        Ok((reply, _)) => Ok(reply),
        Err(error) => Err(supervisor_error(&error)),
    }
}

fn engine_error(kind: ErrorKind) -> DesktopError {
    let (kind, message) = match kind {
        ErrorKind::Io => ("io", "the file could not be read"),
        ErrorKind::Timeout => ("timeout", "reading the file took too long"),
        ErrorKind::WrongPassword => ("wrong-password", "the password doesn't open this document"),
        ErrorKind::Invalid => ("invalid", "this file isn't a PDF Lamina PDF can read"),
        ErrorKind::Limit => ("limit", "this document is too large or complex to open"),
        ErrorKind::UnknownDocument => ("unknown-document", "no such document"),
        ErrorKind::PageOutOfRange => ("page-out-of-range", "no such page"),
        ErrorKind::TooLarge => ("too-large", "the tile is too large"),
        ErrorKind::Cancelled => ("cancelled", "cancelled"),
        ErrorKind::RenderUnavailable => ("render-unavailable", "rendering isn't available"),
        ErrorKind::RenderFailed => ("render-failed", "the page could not be drawn"),
        ErrorKind::Protocol => ("protocol", "the document process misbehaved"),
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
    engine_error(ErrorKind::Protocol)
}

#[cfg(test)]
mod tests;
