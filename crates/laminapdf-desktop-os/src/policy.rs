//! What an administrator set for Lamina PDF's updates (ADR 0014 §1), read where each OS keeps it:
//!
//! - **Windows:** `HKLM\Software\Policies\Lamina`, `UpdateChannel` (a string) and `DisableUpdates`
//!   (a `DWORD`, non-zero for off). The MSI installer writes
//!   `HKLM\Software\Lamina\InstallFlavour` = `msi`.
//! - **macOS:** the app's managed preferences, which a configuration profile sets for the bundle
//!   identifier's domain: `UpdateChannel` and `DisableUpdates`, read only when forced, so a value
//!   the user wrote with `defaults write` doesn't count as policy.
//! - **Linux:** `/etc/lamina/policy.json`, `{ "UpdateChannel": "beta", "DisableUpdates": true }`.
//!
//! The values are passed on as found; the app decides what an unknown channel means.

/// The update policy an administrator set; `None` where nothing is set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ManagedUpdates {
    /// The channel the install must follow (`stable`, `beta` …), as written.
    pub channel: Option<String>,
    /// Whether updates are off.
    pub disable: Option<bool>,
}

/// The update policy on this machine. A policy that can't be read counts as none, with a warning
/// for the caller to log.
#[must_use]
pub fn managed_updates() -> ManagedUpdates {
    #[cfg(windows)]
    return crate::windows::managed_updates();
    #[cfg(target_os = "macos")]
    return crate::macos::managed_updates();
    #[cfg(target_os = "linux")]
    return linux_policy(LINUX_POLICY);
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    ManagedUpdates::default()
}

/// Whether the MSI installer installed the app (Windows only). MSI installs don't update
/// themselves: the update manifest carries only the per-user NSIS installer, so an MSI install
/// would gain a second, per-user copy.
#[must_use]
pub fn installed_by_msi() -> bool {
    #[cfg(windows)]
    return crate::windows::install_flavour().is_some_and(|flavour| flavour.eq_ignore_ascii_case("msi"));
    #[cfg(not(windows))]
    false
}

/// Linux's policy file.
#[cfg(target_os = "linux")]
const LINUX_POLICY: &str = "/etc/lamina/policy.json";

#[cfg(target_os = "linux")]
fn linux_policy(path: &str) -> ManagedUpdates {
    match std::fs::read(path) {
        Ok(bytes) => parse_policy_json(&bytes),
        Err(_) => ManagedUpdates::default(),
    }
}

/// `/etc/lamina/policy.json`'s update keys. A file that isn't a JSON object counts as no policy;
/// a key of the wrong type counts as unset.
#[must_use]
pub fn parse_policy_json(bytes: &[u8]) -> ManagedUpdates {
    let Ok(serde_json::Value::Object(map)) = serde_json::from_slice(bytes) else {
        return ManagedUpdates::default();
    };
    ManagedUpdates {
        channel: map
            .get("UpdateChannel")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        disable: map.get("DisableUpdates").and_then(serde_json::Value::as_bool),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_policy_file_should_give_both_keys() {
        assert_eq!(
            parse_policy_json(br#"{ "UpdateChannel": "beta", "DisableUpdates": true }"#),
            ManagedUpdates {
                channel: Some("beta".to_owned()),
                disable: Some(true),
            }
        );
    }

    #[test]
    fn a_key_of_the_wrong_type_should_count_as_unset() {
        assert_eq!(
            parse_policy_json(br#"{ "UpdateChannel": 2, "DisableUpdates": "yes" }"#),
            ManagedUpdates::default()
        );
    }

    #[test]
    fn a_file_that_isnt_an_object_should_count_as_no_policy() {
        assert_eq!(parse_policy_json(b"[true]"), ManagedUpdates::default());
        assert_eq!(parse_policy_json(b"{ not json"), ManagedUpdates::default());
    }
}
