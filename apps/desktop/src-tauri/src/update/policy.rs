//! What decides whether and where the app updates, besides the user: the managed policy an
//! administrator set (ADR 0014 §1, read by `laminapdf_desktop_os`), how the app was installed, and
//! which build this is. Policy overrides the user's settings; an install the OS's package manager
//! or an MSI owns never updates itself.

use laminapdf_desktop_os::ManagedUpdates;

use super::settings::{Channel, UpdateSettings};

/// Whether a stable release exists yet. Until it does, nobody can be on Stable: no stable build
/// exists to install, so Settings shows the choice disabled (S06). The first stable release sets
/// this to `true`.
pub(crate) const STABLE_AVAILABLE: bool = false;

/// The install line this build belongs to (ADR 0014 §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Line {
    /// Stable and beta builds, signed with the release key.
    Release,
    /// Nightly builds, signed with the nightly key: they can install only nightlies.
    Nightly,
}

/// This build: its version, its line and the channel a fresh install follows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Build {
    pub version: String,
    pub line: Line,
    /// The channel of the download this install came from: `LAMINA_UPDATE_CHANNEL` at build time
    /// (the release builds set it), else Beta, as every download is until the first stable release.
    pub channel: Channel,
}

impl Build {
    /// The build of version `version`, whose release jobs set `channel` (`LAMINA_UPDATE_CHANNEL`).
    pub(crate) fn new(version: &str, channel: Option<&str>) -> Self {
        // Nightlies are versioned `X.Y.Z-nightly.YYYYMMDD.N` (ADR 0014 §1).
        let pre = version.split_once('-').map_or("", |(_, pre)| pre);
        let line = if pre.starts_with("nightly") {
            Line::Nightly
        } else {
            Line::Release
        };
        let channel = match line {
            Line::Nightly => Channel::Nightly,
            Line::Release => channel
                .and_then(Channel::parse)
                .filter(|channel| *channel != Channel::Nightly)
                .unwrap_or(Channel::Beta),
        };
        Self {
            version: version.to_owned(),
            line,
            channel,
        }
    }

    /// This running build.
    pub(crate) fn current(version: &str) -> Self {
        Self::new(version, option_env!("LAMINA_UPDATE_CHANNEL"))
    }
}

/// How the app was installed, as far as updating goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Install {
    /// One the updater can replace: macOS's app, Windows' per-user NSIS install, Linux's `AppImage`.
    Updatable,
    /// A Linux `.deb` or `.rpm`: its package manager updates it (Tauri replaces only an `AppImage`).
    PackageManager,
    /// A Windows MSI install, which an organisation updates.
    Msi,
    /// The Mac App Store build, which the store updates (ADR 0014 §11).
    AppStore,
}

impl Install {
    /// This install. On Linux, only an `AppImage` has `APPIMAGE` set in its environment.
    pub(crate) fn current() -> Self {
        if cfg!(feature = "app-store") {
            Self::AppStore
        } else if cfg!(target_os = "linux") && std::env::var_os("APPIMAGE").is_none() {
            Self::PackageManager
        } else if laminapdf_desktop_os::installed_by_msi() {
            Self::Msi
        } else {
            Self::Updatable
        }
    }
}

/// The administrator's update policy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Policy {
    /// Updates are off, and the user can't turn them on.
    pub disabled: bool,
    /// The channel the install must follow.
    pub channel: Option<Channel>,
}

impl Policy {
    /// The policy `managed` sets. A channel the app doesn't know counts as none, with a warning.
    pub(crate) fn from_managed(managed: &ManagedUpdates) -> Self {
        let channel = managed.channel.as_deref().and_then(|name| {
            let channel = Channel::parse(name);
            if channel.is_none() {
                tracing::warn!("the update policy names an unknown channel; ignoring it");
            }
            channel
        });
        Self {
            disabled: managed.disable.unwrap_or(false),
            channel,
        }
    }

    /// Whether an administrator set anything.
    pub(crate) const fn is_set(self) -> bool {
        self.disabled || self.channel.is_some()
    }
}

/// Why automatic updates are off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Off {
    /// The user turned them off; Check now still works.
    User,
    /// The administrator's policy turned them off.
    Policy,
    /// An MSI install: the organisation updates it.
    Msi,
    /// A Linux `.deb` or `.rpm`: the package manager updates it.
    PackageManager,
    /// The Mac App Store build: the store updates it.
    AppStore,
}

/// Whether the user may switch `build` to `channel`: Beta on a release build, Stable once it exists,
/// never Nightly, and nothing on a nightly build, which trusts only the nightly key.
pub(crate) fn may_choose(channel: Channel, build: &Build) -> bool {
    build.line == Line::Release
        && match channel {
            Channel::Beta => true,
            Channel::Stable => STABLE_AVAILABLE,
            Channel::Nightly => false,
        }
}

/// What the settings, policy, install and build add up to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Effective {
    /// The channel checks ask for.
    pub channel: Channel,
    /// Why automatic updates are off; `None` when they are on.
    pub off: Option<Off>,
    /// Whether an administrator decides (policy or an MSI install): Settings' controls are disabled.
    pub managed: bool,
}

impl Effective {
    /// Whether the scheduler may check now.
    pub(crate) const fn automatic(self) -> bool {
        self.off.is_none()
    }

    /// Whether Check now may check: always, unless someone other than the user turned updates off.
    pub(crate) const fn can_check(self) -> bool {
        matches!(self.off, None | Some(Off::User))
    }
}

/// What applies to `build`, installed as `install`, under `policy`, with the user's `settings`.
pub(crate) fn effective(settings: &UpdateSettings, policy: Policy, install: Install, build: &Build) -> Effective {
    let channel = match build.line {
        // A nightly can install only nightlies, whatever anyone chose.
        Line::Nightly => Channel::Nightly,
        Line::Release => policy
            .channel
            .or(settings.channel)
            .filter(|channel| *channel != Channel::Nightly)
            .unwrap_or(build.channel),
    };
    let off = match install {
        Install::PackageManager => Some(Off::PackageManager),
        Install::AppStore => Some(Off::AppStore),
        Install::Msi => Some(Off::Msi),
        Install::Updatable if policy.disabled => Some(Off::Policy),
        Install::Updatable if !settings.enabled => Some(Off::User),
        Install::Updatable => None,
    };
    Effective {
        channel,
        off,
        managed: policy.is_set() || install == Install::Msi,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release() -> Build {
        Build::new("0.2.0", None)
    }

    #[test]
    fn a_nightly_version_should_make_a_nightly_build() {
        let build = Build::new("0.1.1-nightly.20261028.42", Some("stable"));
        assert_eq!((build.line, build.channel), (Line::Nightly, Channel::Nightly));
    }

    #[test]
    fn a_release_build_should_follow_its_download_and_default_to_beta() {
        assert_eq!(Build::new("0.2.0", Some("stable")).channel, Channel::Stable);
        assert_eq!(Build::new("0.2.0-rc.1", None).channel, Channel::Beta);
        assert_eq!(Build::new("0.2.0", Some("nightly")).channel, Channel::Beta);
    }

    #[test]
    fn policy_should_override_the_users_choice() {
        let settings = UpdateSettings {
            channel: Some(Channel::Stable),
            ..UpdateSettings::default()
        };
        let policy = Policy {
            disabled: true,
            channel: Some(Channel::Beta),
        };
        let applied = effective(&settings, policy, Install::Updatable, &release());
        assert_eq!(
            applied,
            Effective {
                channel: Channel::Beta,
                off: Some(Off::Policy),
                managed: true,
            }
        );
        assert!(!applied.can_check(), "Check now is off too");
    }

    #[test]
    fn a_user_who_turned_updates_off_may_still_check_by_hand() {
        let settings = UpdateSettings {
            enabled: false,
            ..UpdateSettings::default()
        };
        let applied = effective(&settings, Policy::default(), Install::Updatable, &release());
        assert_eq!(applied.off, Some(Off::User));
        assert!(!applied.automatic() && applied.can_check());
    }

    #[test]
    fn a_nightly_should_stay_on_nightly() {
        let settings = UpdateSettings {
            channel: Some(Channel::Stable),
            ..UpdateSettings::default()
        };
        let policy = Policy {
            disabled: false,
            channel: Some(Channel::Beta),
        };
        let nightly = Build::new("0.1.1-nightly.20261028.42", None);
        assert_eq!(
            effective(&settings, policy, Install::Updatable, &nightly).channel,
            Channel::Nightly
        );
    }

    #[test]
    fn a_release_build_should_never_ask_for_nightly() {
        let settings = UpdateSettings {
            channel: Some(Channel::Nightly),
            ..UpdateSettings::default()
        };
        assert_eq!(
            effective(&settings, Policy::default(), Install::Updatable, &release()).channel,
            Channel::Beta
        );
    }

    #[test]
    fn a_release_build_may_choose_beta_and_a_nightly_nothing() {
        let nightly = Build::new("0.1.1-nightly.20261028.42", None);
        assert!(may_choose(Channel::Beta, &release()));
        assert_eq!(may_choose(Channel::Stable, &release()), STABLE_AVAILABLE);
        assert!(!may_choose(Channel::Nightly, &release()));
        assert!(!may_choose(Channel::Beta, &nightly));
    }

    #[test]
    fn package_manager_and_msi_installs_should_never_update_themselves() {
        let settings = UpdateSettings::default();
        let deb = effective(&settings, Policy::default(), Install::PackageManager, &release());
        let msi = effective(&settings, Policy::default(), Install::Msi, &release());
        assert_eq!((deb.off, deb.managed), (Some(Off::PackageManager), false));
        assert_eq!((msi.off, msi.managed), (Some(Off::Msi), true));
        assert!(!deb.can_check() && !msi.can_check());
    }

    #[test]
    fn a_policy_should_read_the_managed_values() {
        let managed = ManagedUpdates {
            channel: Some("Stable".to_owned()),
            disable: Some(false),
        };
        assert_eq!(
            Policy::from_managed(&managed),
            Policy {
                disabled: false,
                channel: Some(Channel::Stable),
            }
        );
        let unknown = ManagedUpdates {
            channel: Some("staging".to_owned()),
            disable: None,
        };
        assert_eq!(Policy::from_managed(&unknown), Policy::default());
        assert!(!Policy::default().is_set());
    }
}
