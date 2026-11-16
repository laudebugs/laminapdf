//! Settings › Graphics, on Linux: the `WebKitGTK` workarounds for blank windows and slow drawing
//! (ADR 0008's Linux fallback; [Tauri's Linux graphics notes]). `WebKitGTK` reads them from the
//! process's environment once, as it starts, so [`prepare`] sets them before the Tauri builder
//! runs. Without `unsafe` (`std::env::set_var` is in Rust 2024): when the environment isn't the
//! one the choice needs, the app runs itself again in place, with `exec`, and the new image starts
//! with it. It keeps the process ID, so the `AppImage` runtime and a test driver waiting on it see
//! the same process.
//!
//! - **Automatic** (the default) sets the DMABUF workaround, and NVIDIA's explicit-sync one, on
//!   NVIDIA's own driver under Wayland, where `WebKitGTK`'s windows are known to stay blank.
//! - **Compatibility** turns `WebKitGTK`'s DMABUF renderer off.
//! - **Safe mode** turns compositing off too: slower, the last resort.
//! - `--safe-graphics` on the command line starts in Safe mode until the app quits (a restart
//!   keeps the flag), for a user who can't reach Settings.
//!
//! A variable the user set themselves is left alone. The ones the app set are named in
//! [`MARKER`], so choosing Automatic again and restarting drops them.
//!
//! The crash-loop guard: each start counts in `graphics-starts.json` until it has run for
//! [`SURVIVED_AFTER`]. When two starts in a row ended before that, the next one switches Automatic
//! to Compatibility and saves it; the app says so once at start, and Settings › Graphics says
//! why. A second launch, which hands its files to the running app and ends, is taken back out.
//! End-to-end builds, which the tests start and stop within seconds, have no guard.
//!
//! [Tauri's Linux graphics notes]: https://v2.tauri.app/develop/debug/linux-graphics/

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::store::{read_json, write_json};

/// Settings › Graphics. Saved in `preferences.json`; it acts on Linux only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Graphics {
    /// `WebKitGTK`'s DMABUF renderer off.
    Compatibility,
    /// DMABUF and compositing off.
    Safe,
    /// The workarounds the machine is known to need, if any; also any value a later version
    /// wrote (serde's `other` variant, which must come last).
    #[default]
    #[serde(other)]
    Automatic,
}

const DMABUF: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";
const COMPOSITING: &str = "WEBKIT_DISABLE_COMPOSITING_MODE";
const NVIDIA_SYNC: &str = "__NV_DISABLE_EXPLICIT_SYNC";

/// The variable that names the ones the app set, comma-separated, so it can tell them from the
/// user's own.
const MARKER: &str = "LAMINA_GRAPHICS_ENV";

/// How long a start must run to count as survived.
pub(crate) const SURVIVED_AFTER: Duration = Duration::from_secs(10);

/// The starts file, beside the preferences.
const STARTS_FILE: &str = "graphics-starts.json";

/// What the machine is, for Automatic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Machine {
    /// NVIDIA's own driver is loaded (`/proc/driver/nvidia/version` exists).
    pub nvidia: bool,
    /// The session is Wayland (`WAYLAND_DISPLAY` is set).
    pub wayland: bool,
}

/// The variables `choice` needs on `machine`, each with its value.
pub(crate) fn graphics_env(choice: Graphics, machine: Machine) -> Vec<(&'static str, &'static str)> {
    match choice {
        Graphics::Automatic if machine.nvidia && machine.wayland => vec![(DMABUF, "1"), (NVIDIA_SYNC, "1")],
        Graphics::Automatic => Vec::new(),
        Graphics::Compatibility => vec![(DMABUF, "1")],
        Graphics::Safe => vec![(DMABUF, "1"), (COMPOSITING, "1")],
    }
}

/// The choice this start uses: Safe mode with the flag; at least Compatibility after a crash loop;
/// else the saved one.
pub(crate) fn effective(saved: Graphics, safe_flag: bool, crash_loop: bool) -> Graphics {
    if safe_flag {
        Graphics::Safe
    } else if crash_loop && saved == Graphics::Automatic {
        Graphics::Compatibility
    } else {
        saved
    }
}

/// The starts that haven't yet run for [`SURVIVED_AFTER`], and whether the guard switched to
/// Compatibility and hasn't told the user yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct Starts {
    pub unfinished: u32,
    pub fell_back: bool,
}

impl Starts {
    /// A new start: counted, and `true` when the two before it both ended early. Whether the
    /// guard then switched anything is the caller's to say ([`Starts::fell_back`]).
    pub(crate) const fn started(self) -> (Self, bool) {
        if self.unfinished >= 2 {
            (Self { unfinished: 1, ..self }, true)
        } else {
            (
                Self {
                    unfinished: self.unfinished + 1,
                    ..self
                },
                false,
            )
        }
    }

    /// A second launch handed its files to the running app and ended: its own start no longer
    /// counts, and the running app's still does until it survives.
    pub(crate) const fn handed_over(self) -> Self {
        Self {
            unfinished: self.unfinished.saturating_sub(1),
            fell_back: self.fell_back,
        }
    }

    /// The start ran long enough, or quit cleanly: no longer counted.
    pub(crate) const fn survived(self) -> Self {
        Self {
            unfinished: 0,
            fell_back: self.fell_back,
        }
    }

    /// The starts saved in `config`'s starts file, or none.
    pub(crate) fn load(config: &Path) -> Self {
        read_json(&config.join(STARTS_FILE), "graphics starts").unwrap_or_default()
    }

    /// Saves them in `config`'s starts file; a failure is logged (on stderr before the log is up).
    pub(crate) fn save(self, config: &Path) {
        if let Err(error) = write_json(&config.join(STARTS_FILE), &self) {
            tracing::warn!("could not save the graphics starts: {error}");
            eprintln!("Lamina PDF could not save the graphics starts: {error}");
        }
    }
}

/// The environment to run with, when it differs from the current one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Rerun {
    pub set: Vec<(&'static str, &'static str)>,
    pub remove: Vec<&'static str>,
    /// [`MARKER`]'s new value: the names in `set`.
    pub marker: String,
}

/// What to change so the process has `wanted`: the user's own variables stay; the app's earlier
/// ones (named in [`MARKER`]) that are no longer wanted go. `None` when nothing changes.
pub(crate) fn rerun(
    wanted: &[(&'static str, &'static str)],
    current: impl Fn(&str) -> Option<String>,
) -> Option<Rerun> {
    let ours = current(MARKER).unwrap_or_default();
    let ours: Vec<&str> = ours.split(',').filter(|name| !name.is_empty()).collect();
    let users = |name: &str| current(name).is_some() && !ours.contains(&name);
    let set: Vec<_> = wanted.iter().copied().filter(|(name, _)| !users(name)).collect();
    let remove: Vec<_> = [DMABUF, COMPOSITING, NVIDIA_SYNC]
        .into_iter()
        .filter(|name| ours.contains(name) && !set.iter().any(|(wanted, _)| wanted == name))
        .collect();
    let unchanged = remove.is_empty() && set.iter().all(|(name, value)| current(name).as_deref() == Some(value));
    if unchanged {
        return None;
    }
    let marker = set.iter().map(|(name, _)| *name).collect::<Vec<_>>().join(",");
    Some(Rerun { set, remove, marker })
}

/// The config folder Tauri gives an app with `identifier` on Linux: `$XDG_CONFIG_HOME` when it is
/// absolute, else `~/.config`, then the identifier.
pub(crate) fn config_dir(identifier: &str, current: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    // XDG's "absolute" is Linux's: a leading `/` (so the tests agree on every OS).
    let base = current("XDG_CONFIG_HOME")
        .filter(|path| path.starts_with('/'))
        .map(PathBuf::from)
        .or_else(|| current("HOME").map(|home| Path::new(&home).join(".config")))?;
    Some(base.join(identifier))
}

/// What this start chose, for the log and Settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Status {
    /// Whether the Graphics section applies: on Linux only.
    pub applies: bool,
    /// The choice this start runs with.
    pub running: Graphics,
    /// The variables the app set for it.
    pub set: Vec<String>,
    /// The guard switched to Compatibility after two failed starts.
    pub fell_back: bool,
}

static STATUS: OnceLock<Status> = OnceLock::new();

/// This start's graphics; Automatic with nothing set before [`prepare`] ran, and off Linux.
pub(crate) fn status() -> Status {
    STATUS.get().cloned().unwrap_or(Status {
        applies: cfg!(target_os = "linux"),
        running: Graphics::Automatic,
        set: Vec::new(),
        fell_back: false,
    })
}

/// Before anything else starts: counts this start, chooses the graphics, and, when the
/// environment isn't the one they need, runs the app again in place with it. Off Linux, nothing.
pub(crate) fn prepare(identifier: &str) {
    #[cfg(target_os = "linux")]
    linux::prepare(identifier);
    #[cfg(not(target_os = "linux"))]
    let _ = identifier;
}

/// The start survived: after [`SURVIVED_AFTER`] on a thread of its own, and at a clean quit or a
/// restart.
pub(crate) fn survived(config: &Path) {
    if !cfg!(target_os = "linux") {
        return;
    }
    let starts = Starts::load(config);
    if starts.unfinished > 0 {
        starts.survived().save(config);
    }
}

/// A second launch reached the running app: the single-instance plugin ends that process at once,
/// so its own count is taken back here, and the running app's start still counts.
pub(crate) fn handed_over(config: &Path) {
    if !cfg!(target_os = "linux") {
        return;
    }
    let starts = Starts::load(config);
    if starts.unfinished > 0 {
        starts.handed_over().save(config);
    }
}

/// Counts the start as survived once it has run for [`SURVIVED_AFTER`].
pub(crate) fn watch_start(config: PathBuf) {
    if !cfg!(target_os = "linux") {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("graphics-start".into())
        .spawn(move || {
            std::thread::sleep(SURVIVED_AFTER);
            survived(&config);
        });
    if let Err(error) = spawned {
        tracing::warn!("could not watch the start: {error}");
    }
}

/// Whether to tell the user the guard switched to Compatibility: `true` once per run, while the
/// starts file says it hasn't been told yet; it is then marked told. Settings › Graphics still says
/// so for the rest of this run ([`Status::fell_back`]).
pub(crate) fn take_notice(config: &Path) -> bool {
    static TOLD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    let starts = Starts::load(config);
    if !starts.fell_back || TOLD.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return false;
    }
    Starts {
        fell_back: false,
        ..starts
    }
    .save(config);
    true
}

/// Logs this start's graphics, once the log is up.
pub(crate) fn log() {
    let status = status();
    if status.applies {
        tracing::info!(choice = ?status.running, set = ?status.set, fell_back = status.fell_back, "graphics");
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::os::unix::process::CommandExt as _;
    use std::path::Path;

    use crate::prefs::Preferences;

    use super::{
        Graphics, MARKER, Machine, STATUS, Starts, Status, config_dir, effective, graphics_env, read_json, rerun,
        write_json,
    };

    /// Set to the process ID by the `exec`: the new image is the same start, not another one.
    const RERUN: &str = "LAMINA_GRAPHICS_RERUN";

    /// The command-line flag for Safe mode.
    const SAFE_FLAG: &str = "--safe-graphics";

    fn var(name: &str) -> Option<String> {
        std::env::var(name).ok().filter(|value| !value.is_empty())
    }

    pub(super) fn prepare(identifier: &str) {
        let Some(config) = config_dir(identifier, var) else {
            return;
        };
        let me = std::process::id().to_string();
        // The `exec` below starts this same start again: it is counted once.
        let again = var(RERUN).as_deref() == Some(me.as_str());
        let prefs_file = config.join("preferences.json");
        let mut prefs: Preferences = read_json(&prefs_file, "preferences").unwrap_or_default();
        let starts = Starts::load(&config);
        // End-to-end builds are started and stopped within seconds, test after test: no guard.
        let counted = !again && !cfg!(feature = "e2e");
        let (mut starts, crash_loop) = if counted { starts.started() } else { (starts, false) };
        // Only a switch is news: Compatibility and Safe mode stay as they are.
        let switched = crash_loop && prefs.graphics == Graphics::Automatic;
        starts.fell_back |= switched;
        if counted {
            starts.save(&config);
        }
        let safe_flag = std::env::args_os().any(|arg| arg == SAFE_FLAG);
        let running = effective(prefs.graphics, safe_flag, crash_loop);
        // The guard's switch is saved; the flag's lasts until the app quits (a restart keeps it).
        if switched {
            prefs.graphics = Graphics::Compatibility;
            if let Err(error) = write_json(&prefs_file, &prefs) {
                eprintln!("Lamina PDF could not save the graphics choice: {error}");
            }
        }
        let machine = Machine {
            nvidia: Path::new("/proc/driver/nvidia/version").exists(),
            wayland: var("WAYLAND_DISPLAY").is_some(),
        };
        let wanted = graphics_env(running, machine);
        let set = var(MARKER)
            .map(|names| names.split(',').map(str::to_owned).collect())
            .unwrap_or_default();
        let _ = STATUS.set(Status {
            applies: true,
            running,
            set,
            fell_back: starts.fell_back,
        });
        let Some(change) = rerun(&wanted, var) else {
            return;
        };
        if again {
            // The environment the `exec` gave isn't the one wanted: never loop.
            eprintln!("Lamina PDF could not set its graphics environment");
            return;
        }
        let Ok(program) = std::env::current_exe() else {
            return;
        };
        let mut command = std::process::Command::new(program);
        command
            .args(std::env::args_os().skip(1))
            .envs(change.set.iter().copied())
            .env(MARKER, &change.marker)
            .env(RERUN, &me);
        for name in &change.remove {
            command.env_remove(name);
        }
        // Returns only when it failed; the app then starts with the environment it has.
        let error = command.exec();
        eprintln!("Lamina PDF could not restart with its graphics settings: {error}");
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: BTreeMap<String, String> = pairs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
        move |name| map.get(name).cloned()
    }

    const NVIDIA_WAYLAND: Machine = Machine {
        nvidia: true,
        wayland: true,
    };

    #[test]
    fn automatic_should_set_nothing_on_an_ordinary_machine() {
        assert_eq!(graphics_env(Graphics::Automatic, Machine::default()), vec![]);
    }

    #[test]
    fn automatic_should_set_the_dmabuf_and_sync_workarounds_on_nvidia_under_wayland() {
        assert_eq!(
            graphics_env(Graphics::Automatic, NVIDIA_WAYLAND),
            vec![(DMABUF, "1"), (NVIDIA_SYNC, "1")]
        );
    }

    #[test]
    fn automatic_should_set_nothing_on_nvidia_under_x11() {
        let x11 = Machine {
            nvidia: true,
            wayland: false,
        };
        assert_eq!(graphics_env(Graphics::Automatic, x11), vec![]);
    }

    #[test]
    fn compatibility_should_turn_the_dmabuf_renderer_off() {
        assert_eq!(
            graphics_env(Graphics::Compatibility, Machine::default()),
            vec![(DMABUF, "1")]
        );
    }

    #[test]
    fn safe_mode_should_turn_compositing_off_too() {
        assert_eq!(
            graphics_env(Graphics::Safe, Machine::default()),
            vec![(DMABUF, "1"), (COMPOSITING, "1")]
        );
    }

    #[test]
    fn the_flag_should_start_in_safe_mode_whatever_is_saved() {
        assert_eq!(effective(Graphics::Automatic, true, false), Graphics::Safe);
    }

    #[test]
    fn a_crash_loop_should_raise_automatic_to_compatibility() {
        assert_eq!(effective(Graphics::Automatic, false, true), Graphics::Compatibility);
    }

    #[test]
    fn a_crash_loop_should_keep_safe_mode() {
        assert_eq!(effective(Graphics::Safe, false, true), Graphics::Safe);
    }

    #[test]
    fn the_settings_page_should_offer_exactly_the_choices_rust_reads() {
        let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../src/graphics.tsx")).unwrap();
        let line = source
            .lines()
            .find(|line| line.starts_with("const GRAPHICS = ["))
            .unwrap();
        let offered: Vec<&str> = line.split('"').skip(1).step_by(2).collect();
        // A new choice stops this compiling until it joins the list below.
        let in_list = |choice: Graphics| match choice {
            Graphics::Automatic | Graphics::Compatibility | Graphics::Safe => (),
        };
        let all = [Graphics::Automatic, Graphics::Compatibility, Graphics::Safe];
        all.iter().copied().for_each(in_list);
        let rust: Vec<String> = all
            .iter()
            .map(|choice| {
                serde_json::to_value(choice)
                    .unwrap()
                    .as_str()
                    .unwrap_or_default()
                    .to_owned()
            })
            .collect();
        assert_eq!(offered, rust);
    }

    #[test]
    fn an_unknown_choice_should_read_as_automatic() {
        let read: Graphics = serde_json::from_str(r#""vulkan""#).unwrap();
        assert_eq!(read, Graphics::Automatic);
    }

    #[test]
    fn one_early_exit_should_not_be_a_crash_loop() {
        let (_, crash_loop) = Starts::default().started().0.started();
        assert!(!crash_loop);
    }

    #[test]
    fn two_early_exits_in_a_row_should_be_a_crash_loop() {
        let (starts, crash_loop) = Starts::default().started().0.started().0.started();
        assert_eq!((starts.unfinished, crash_loop), (1, true));
    }

    #[test]
    fn a_handed_over_launch_should_leave_the_running_apps_start_counted() {
        let (starts, _) = Starts::default().started().0.started();
        assert_eq!(starts.handed_over().unfinished, 1);
    }

    #[test]
    fn a_start_that_survives_should_reset_the_count() {
        let (starts, _) = Starts::default().started().0.started();
        let (_, crash_loop) = starts.survived().started();
        assert!(!crash_loop);
    }

    #[test]
    fn a_wanted_variable_should_be_set_and_named_in_the_marker() {
        assert_eq!(
            rerun(&[(DMABUF, "1")], env(&[])),
            Some(Rerun {
                set: vec![(DMABUF, "1")],
                remove: vec![],
                marker: DMABUF.to_owned()
            })
        );
    }

    #[test]
    fn the_environment_the_choice_needs_should_change_nothing() {
        assert_eq!(rerun(&[(DMABUF, "1")], env(&[(DMABUF, "1"), (MARKER, DMABUF)])), None);
    }

    #[test]
    fn the_users_own_variable_should_be_left_alone() {
        assert_eq!(rerun(&[(DMABUF, "1")], env(&[(DMABUF, "0")])), None);
    }

    #[test]
    fn the_apps_variable_should_go_when_no_longer_wanted() {
        assert_eq!(
            rerun(&[], env(&[(DMABUF, "1"), (MARKER, DMABUF)])),
            Some(Rerun {
                set: vec![],
                remove: vec![DMABUF],
                marker: String::new()
            })
        );
    }

    #[test]
    fn the_config_folder_should_follow_an_absolute_xdg_config_home() {
        assert_eq!(
            config_dir("app.id", env(&[("XDG_CONFIG_HOME", "/x"), ("HOME", "/home/u")])),
            Some(PathBuf::from("/x/app.id"))
        );
    }

    #[test]
    fn the_config_folder_should_fall_back_to_dot_config() {
        assert_eq!(
            config_dir("app.id", env(&[("XDG_CONFIG_HOME", "rel"), ("HOME", "/home/u")])),
            Some(PathBuf::from("/home/u/.config/app.id"))
        );
    }
}
