//! The app's self-update (ADR 0014; Paper S06): checks the update Worker 10 s after start and every
//! 24 hours, downloads an update in the background, and installs it when the app quits or when the
//! user chooses Restart to update.
//!
//! The updater plugin is Rust's own: the webview is granted none of its commands (the capability
//! guard forbids `updater:`), so every request goes through [`Updates::check`], which sets the
//! channel's endpoint and the install ID header. The plugin verifies the download's minisign
//! signature against the public key compiled into the app before anything is ready, and again
//! before installing; a tampered bundle never installs. Automatic failures are silent (logged at
//! `debug`) and retried at the next check; only Check now reports one.

pub(crate) mod commands;
#[cfg(feature = "e2e")]
mod e2e;
#[cfg(not(feature = "e2e"))]
#[path = "e2e_off.rs"]
mod e2e;
mod plan;
pub(crate) mod policy;
pub(crate) mod settings;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{Emitter as _, Manager as _};
use tauri_plugin_updater::UpdaterExt as _;

use self::plan::{CHECK_TIMEOUT, UPDATE_SERVER, next_check, plan_check};
use self::policy::{Build, Effective, Install, Line, Off, Policy, STABLE_AVAILABLE, effective};
use self::settings::{Channel, UpdateStore};

/// The event every window gets when the update status changes.
pub(crate) const STATUS_EVENT: &str = "update://status";

/// Where the update is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub(crate) enum State {
    /// Nothing to show: no check running and no update waiting.
    Idle,
    /// A check is running.
    Checking,
    /// An update is downloading; `percent` once the server said its size.
    Downloading { version: String, percent: Option<u8> },
    /// An update is downloaded and its signature checked: it installs on quit.
    Ready { version: String, notes: Option<String> },
    /// Check now failed. Automatic checks never show this.
    Failed,
}

/// What Settings › Updates and the status bar show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "a plain snapshot for the UI: each flag is a separate fact Settings shows"
)]
pub(crate) struct UpdateStatus {
    pub state: State,
    /// The running version.
    pub current: String,
    /// The channel checks ask for.
    pub channel: Channel,
    /// Whether this is a nightly build, which stays on the nightly line.
    pub nightly: bool,
    /// The user's Automatic updates switch.
    pub enabled: bool,
    /// Why automatic updates are off, if they are.
    pub off: Option<Off>,
    /// Whether an administrator decides: the controls are disabled.
    pub managed: bool,
    /// Whether Check now may run.
    pub can_check: bool,
    /// Whether the Stable channel can be chosen yet.
    pub stable_available: bool,
    /// When the last check ran, in seconds since the Unix epoch.
    pub last_checked: Option<u64>,
}

/// A downloaded, verified update, waiting to install.
struct Ready {
    update: tauri_plugin_updater::Update,
    bytes: Vec<u8>,
}

/// The update state the app keeps.
pub(crate) struct Updates {
    store: UpdateStore,
    policy: Policy,
    install: Install,
    build: Build,
    state: Mutex<State>,
    ready: Mutex<Option<Ready>>,
    /// A check or download is running: a second one waits for the next time.
    busy: AtomicBool,
}

impl Updates {
    /// The settings in `config`'s `updates.json`, this machine's policy and install, and this build.
    pub(crate) fn load(config: &std::path::Path, version: &str) -> Self {
        e2e::load(config);
        Self {
            store: UpdateStore::load(config.join("updates.json")),
            policy: Policy::from_managed(&laminapdf_desktop_os::managed_updates()),
            install: Install::current(),
            build: Build::current(version),
            state: Mutex::new(State::Idle),
            ready: Mutex::new(None),
            busy: AtomicBool::new(false),
        }
    }

    fn applied(&self) -> Effective {
        effective(&self.store.get(), self.policy, self.install, &self.build)
    }

    /// The status, as the UI shows it.
    pub(crate) fn status(&self) -> UpdateStatus {
        let settings = self.store.get();
        let applied = effective(&settings, self.policy, self.install, &self.build);
        UpdateStatus {
            state: lock(&self.state).clone(),
            current: self.build.version.clone(),
            channel: applied.channel,
            nightly: self.build.line == Line::Nightly,
            enabled: settings.enabled,
            off: applied.off,
            managed: applied.managed,
            can_check: applied.can_check(),
            stable_available: STABLE_AVAILABLE,
            last_checked: settings.last_check,
        }
    }

    /// Whether the user may change the switch and the channel: not when an administrator, a package
    /// manager, an MSI install or the App Store decides.
    fn user_decides(&self) -> bool {
        let applied = self.applied();
        !applied.managed && !matches!(applied.off, Some(Off::PackageManager | Off::AppStore))
    }

    fn set_state(&self, app: &tauri::AppHandle, state: State) {
        *lock(&self.state) = state;
        self.announce(app);
    }

    /// Sends every window the status.
    fn announce(&self, app: &tauri::AppHandle) {
        if let Err(error) = app.emit(STATUS_EVENT, self.status()) {
            tracing::debug!("could not send the update status: {error}");
        }
    }

    /// Checks for an update and downloads it, unless one is waiting or a check is running. With
    /// `manual` (Check now) a failure shows in Settings; otherwise it is only logged.
    pub(crate) async fn check(&self, app: &tauri::AppHandle, manual: bool) {
        if lock(&self.ready).is_some() || self.busy.swap(true, Ordering::AcqRel) {
            return;
        }
        // Cleared however the check ends, a panic included, so later checks still run.
        let _busy = Busy(&self.busy);
        if let Some(request) = plan_check(&self.store, self.applied(), manual, UPDATE_SERVER) {
            // The update test (e2e builds) asks its own server, on the staging channel.
            let request = e2e::request(request);
            self.set_state(app, State::Checking);
            let outcome = self.fetch(app, request, manual).await;
            // "Last checked" counts only checks the server answered.
            if outcome.is_ok() {
                self.store.update(|settings| settings.last_check = Some(now()));
            }
            let install_now = e2e::checked(&outcome);
            let state = match outcome {
                Ok(Some(ready)) => {
                    let state = State::Ready {
                        version: ready.update.version.clone(),
                        notes: ready.update.body.clone(),
                    };
                    *lock(&self.ready) = Some(ready);
                    state
                }
                Ok(None) => State::Idle,
                Err(error) => {
                    tracing::debug!("the update check failed: {error}");
                    if manual { State::Failed } else { State::Idle }
                }
            };
            self.set_state(app, state);
            if install_now {
                e2e::install_now(app);
            }
        }
    }

    /// Changes the user's settings with `change`, unless someone else decides, and tells the windows.
    fn change_settings(
        &self,
        app: &tauri::AppHandle,
        change: impl FnOnce(&mut settings::UpdateSettings),
    ) -> Result<UpdateStatus, crate::DesktopError> {
        if !self.user_decides() {
            return Err(crate::DesktopError::UpdatesManaged);
        }
        self.store.update(change);
        self.announce(app);
        Ok(self.status())
    }

    /// One check and, when an update applies, its download: verified before it counts as ready.
    async fn fetch(
        &self,
        app: &tauri::AppHandle,
        request: plan::CheckRequest,
        manual: bool,
    ) -> Result<Option<Ready>, tauri_plugin_updater::Error> {
        let url = tauri::Url::parse(&request.url)?;
        let updater = app
            .updater_builder()
            .endpoints(vec![url])?
            .header(request.header.0, request.header.1)?
            .timeout(CHECK_TIMEOUT)
            // Windows: the installer exits the app as soon as it starts, before any `Exit` event, so
            // the engines stop here, once it has started; a failed install leaves them running.
            .on_before_exit({
                let app = app.clone();
                move || app.state::<Arc<crate::engine::Engines>>().shutdown()
            })
            .build()?;
        let Some(update) = updater.check().await? else {
            return Ok(None);
        };
        // Updates turned off while the check ran: nothing more is fetched (Check now goes on).
        if !self.still_wanted(manual) {
            return Ok(None);
        }
        let version = update.version.clone();
        self.set_state(
            app,
            State::Downloading {
                version: version.clone(),
                percent: None,
            },
        );
        let mut received: u64 = 0;
        let mut shown: Option<u8> = None;
        let bytes = update
            .download(
                |chunk, total| {
                    received = received.saturating_add(chunk as u64);
                    let percent = total.filter(|total| *total > 0).map(|total| {
                        u8::try_from(received.saturating_mul(100) / total)
                            .unwrap_or(100)
                            .min(100)
                    });
                    if percent != shown {
                        shown = percent;
                        self.set_state(
                            app,
                            State::Downloading {
                                version: version.clone(),
                                percent,
                            },
                        );
                    }
                },
                || {},
            )
            .await?;
        // Turned off during the download: the update is dropped, so nothing installs on quit.
        Ok(self.still_wanted(manual).then_some(Ready { update, bytes }))
    }

    /// Whether a check that began (by hand when `manual`) may go on: an automatic one stops once
    /// updates are off; Check now goes on unless policy or the install turned them off.
    fn still_wanted(&self, manual: bool) -> bool {
        let applied = self.applied();
        if manual {
            applied.can_check()
        } else {
            applied.automatic()
        }
    }

    /// Installs the waiting update, if any: the app is quitting. On Windows the installer takes over
    /// and the process exits; on macOS and Linux the new version starts next time.
    pub(crate) fn install_on_quit(&self) {
        if let Err(error) = self.install() {
            tracing::warn!("could not install the update: {error}");
        }
    }

    /// Installs the waiting update; `Ok` with none waiting. A failed install keeps it waiting.
    fn install(&self) -> Result<(), tauri_plugin_updater::Error> {
        let ready = lock(&self.ready);
        match ready.as_ref() {
            Some(ready) => ready.update.install(&ready.bytes),
            None => Ok(()),
        }
    }

    /// Whether an update is waiting.
    fn has_ready(&self) -> bool {
        lock(&self.ready).is_some()
    }
}

/// Clears the busy flag when dropped.
struct Busy<'a>(&'a AtomicBool);

impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// Starts the checks: the first 10 s after start, then one a day. Each is skipped while updates are
/// off, so turning them off stops every request. A thread of its own, which sleeps between checks.
pub(crate) fn start(app: &tauri::AppHandle) {
    // The store updates the Mac App Store build: it checks nothing (ADR 0014 §11).
    if Install::current() == Install::AppStore {
        return;
    }
    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("laminapdf-updates".to_owned())
        .spawn(move || {
            let first = e2e::first_check(&app.state::<Arc<Updates>>().build.version);
            std::thread::sleep(first);
            loop {
                let updates = Arc::clone(&app.state::<Arc<Updates>>());
                tauri::async_runtime::block_on(updates.check(&app, false));
                // A fresh random seed per wait, from the OS's random numbers via a UUID v4.
                let seed = uuid::Uuid::new_v4().as_u64_pair().0;
                std::thread::sleep(next_check(seed));
            }
        });
    if let Err(error) = spawned {
        tracing::warn!("could not start the update checks: {error}");
    }
}
