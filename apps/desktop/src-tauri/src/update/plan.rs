//! What a check asks for and when checks run (ADR 0014 §3, §5; S06): the endpoint of the channel,
//! the install ID as a request header and never in the URL, a first check 10 s after start and one
//! every 24 hours ± 30 minutes after that.

use std::time::Duration;

use super::policy::Effective;
use super::settings::{Channel, UpdateStore};

/// The update Worker in production.
pub(crate) const UPDATE_SERVER: &str = "https://updates.laminapdf.com";

/// The header that carries the install ID (ADR 0014 §3).
pub(crate) const INSTALL_ID_HEADER: &str = "Lamina-Install-Id";

/// How long a check may take before it counts as failed.
pub(crate) const CHECK_TIMEOUT: Duration = Duration::from_secs(30);

/// The first check after the app starts.
pub(crate) const FIRST_CHECK: Duration = Duration::from_secs(10);

/// The time between checks, before jitter.
const INTERVAL: Duration = Duration::from_hours(24);

/// How far a check may move from the interval, either way, so installs don't all ask at once.
const JITTER: Duration = Duration::from_mins(30);

/// One check: where it goes and the install ID it sends as a header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CheckRequest {
    /// The endpoint, with the updater plugin's `{{target}}`, `{{arch}}` and `{{current_version}}`.
    pub url: String,
    /// The header's name and value.
    pub header: (&'static str, String),
}

/// The endpoint of `channel` at `server`. The updater plugin fills in the placeholders; nothing
/// about the install goes in the URL.
pub(crate) fn endpoint(server: &str, channel: Channel) -> String {
    endpoint_of(server, channel.as_str())
}

/// The endpoint of the channel named `channel` at `server`: [`endpoint`], or the update test's
/// `staging`, which no install follows.
pub(crate) fn endpoint_of(server: &str, channel: &str) -> String {
    format!(
        "{}/{channel}/{{{{target}}}}/{{{{arch}}}}/{{{{current_version}}}}",
        server.trim_end_matches('/'),
    )
}

/// The check to make now, or `None` when none may run: automatic checks only while updates are on,
/// one the user asked for (`manual`) unless policy or the install turned updates off. `None`
/// makes no network request at all.
pub(crate) fn plan_check(store: &UpdateStore, applied: Effective, manual: bool, server: &str) -> Option<CheckRequest> {
    let allowed = if manual {
        applied.can_check()
    } else {
        applied.automatic()
    };
    allowed.then(|| CheckRequest {
        url: endpoint(server, applied.channel),
        header: (INSTALL_ID_HEADER, store.install_id()),
    })
}

/// The wait before the next check: the interval moved by `seed`'s share of the jitter.
pub(crate) fn next_check(seed: u64) -> Duration {
    let span = JITTER.as_secs() * 2 + 1;
    INTERVAL.saturating_sub(JITTER) + Duration::from_secs(seed % span)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::update::policy::Off;
    use crate::update::settings::UpdateSettings;

    const ON: Effective = Effective {
        channel: Channel::Beta,
        off: None,
        managed: false,
    };

    fn store() -> UpdateStore {
        UpdateStore::in_memory(UpdateSettings::default())
    }

    #[test]
    fn the_endpoint_should_name_the_channel_and_the_plugins_placeholders() {
        assert_eq!(
            endpoint(UPDATE_SERVER, Channel::Stable),
            "https://updates.laminapdf.com/stable/{{target}}/{{arch}}/{{current_version}}"
        );
        assert_eq!(
            endpoint("http://127.0.0.1:8787/", Channel::Nightly),
            "http://127.0.0.1:8787/nightly/{{target}}/{{arch}}/{{current_version}}"
        );
    }

    #[test]
    fn the_install_id_should_go_in_the_header_and_never_the_url() {
        let store = store();
        let request = plan_check(&store, ON, false, UPDATE_SERVER).unwrap();
        let id = store.install_id();
        assert_eq!(request.header, (INSTALL_ID_HEADER, id.clone()));
        assert!(!request.url.contains(&id), "{}", request.url);
    }

    #[test]
    fn the_next_check_should_send_a_reset_id() {
        let store = store();
        let before = plan_check(&store, ON, false, UPDATE_SERVER).unwrap().header.1;
        store.reset_install_id();
        let after = plan_check(&store, ON, true, UPDATE_SERVER).unwrap().header.1;
        assert_ne!(before, after);
        assert_eq!(after, store.install_id());
    }

    #[test]
    fn updates_off_should_make_no_request() {
        let store = store();
        for off in [Off::User, Off::Policy, Off::Msi, Off::PackageManager] {
            let applied = Effective { off: Some(off), ..ON };
            assert_eq!(plan_check(&store, applied, false, UPDATE_SERVER), None, "{off:?}");
        }
    }

    #[test]
    fn check_now_should_work_only_when_the_user_turned_updates_off() {
        let store = store();
        let user = Effective {
            off: Some(Off::User),
            ..ON
        };
        assert!(plan_check(&store, user, true, UPDATE_SERVER).is_some());
        for off in [Off::Policy, Off::Msi, Off::PackageManager] {
            let applied = Effective { off: Some(off), ..ON };
            assert_eq!(plan_check(&store, applied, true, UPDATE_SERVER), None, "{off:?}");
        }
    }

    #[test]
    fn checks_should_come_a_day_apart_within_half_an_hour() {
        let day = INTERVAL.as_secs();
        let half_hour = JITTER.as_secs();
        for seed in [0, 1, 1800, 3600, 3601, u64::MAX] {
            let wait = next_check(seed).as_secs();
            assert!((day - half_hour..=day + half_hour).contains(&wait), "{seed}: {wait}");
        }
        assert_eq!(next_check(0).as_secs(), day - half_hour);
        assert_eq!(next_check(3600).as_secs(), day + half_hour);
    }
}
