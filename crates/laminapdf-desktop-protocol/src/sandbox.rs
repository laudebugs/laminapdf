//! [`SandboxLevel`]: the layers of its sandbox an engine reports in `Ready` (ADR 0008 §5).

use std::fmt;

use serde::{Deserialize, Serialize};

/// One layer of the engine's sandbox. Each OS has its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Layer {
    /// Turned off for diagnosis (`LAMINA_PDF_ENGINE_SANDBOX=off`, debug builds only).
    Off,
    /// Linux: `PR_SET_NO_NEW_PRIVS`.
    NoNewPrivs,
    /// Linux: the seccomp-bpf allow-list.
    Seccomp,
    /// Linux: the Landlock ruleset, at the ABI the kernel enforced, and whether it enforced every
    /// right the ruleset handles.
    Landlock {
        /// The Landlock ABI.
        abi: u8,
        /// Every handled right enforced.
        full: bool,
    },
    /// Linux: `RLIMIT_AS`; Windows: the job's committed-memory limit; macOS in the Mac App Store
    /// build: the engine's own watch of its footprint.
    MemoryLimit,
    /// macOS: the Seatbelt profile.
    Seatbelt,
    /// Windows: the main process's job (one active process, kill on close, UI limits).
    Job,
    /// Windows: Low integrity.
    LowIntegrity,
    /// Windows: the locked-down restricted token, after `RevertToSelf`.
    RestrictedToken,
    /// Windows: the child-process mitigation policy.
    NoChildProcesses,
    /// Windows: only Microsoft-signed images load from now on.
    SignedImagesOnly,
    /// Windows: no dynamic code.
    NoDynamicCode,
    /// macOS: App Sandbox, the Mac App Store build's, which the engine inherits from the app and
    /// which allows no Seatbelt profile on top (ADR 0014 §11). Last, so the wire's indices of the
    /// others never move.
    AppSandbox,
    /// macOS, in the Mac App Store build: `RLIMIT_NOFILE` capped at the descriptors the engine
    /// holds once it is ready, so it can open no socket, file or pipe. App Sandbox lets the app
    /// connect out (its web view needs that), and the engine inherits it, so this stands in for
    /// the Seatbelt profile's network rule (ADR 0014 §11).
    NoNewDescriptors,
}

/// Whether this process runs in macOS's App Sandbox, which names the container of every process
/// it sandboxes in `APP_SANDBOX_CONTAINER_ID`. Only the Mac App Store build does (ADR 0014 §11).
#[must_use]
pub fn in_app_sandbox() -> bool {
    cfg!(target_os = "macos") && std::env::var_os("APP_SANDBOX_CONTAINER_ID").is_some()
}

impl Layer {
    /// The layers an engine on this OS must report, or the main process refuses it. On macOS,
    /// the Mac App Store build's main process (feature `app-sandbox`) running in App Sandbox,
    /// which its engines inherit, requires [`Self::STORE_BUILD`] instead (ADR 0014 §11).
    #[must_use]
    pub fn required() -> &'static [Self] {
        if cfg!(all(target_os = "macos", feature = "app-sandbox")) && in_app_sandbox() {
            Self::STORE_BUILD
        } else {
            Self::REQUIRED
        }
    }

    /// What an engine in the Mac App Store build's App Sandbox must report.
    pub const STORE_BUILD: &'static [Self] = &[Self::AppSandbox, Self::NoNewDescriptors, Self::MemoryLimit];

    /// The layers an engine on this OS must report outside the Mac App Store build.
    pub const REQUIRED: &'static [Self] = if cfg!(target_os = "linux") {
        &[Self::NoNewPrivs, Self::Seccomp, Self::MemoryLimit]
    } else if cfg!(target_os = "macos") {
        &[Self::Seatbelt]
    } else if cfg!(windows) {
        &[
            Self::Job,
            Self::MemoryLimit,
            Self::LowIntegrity,
            Self::RestrictedToken,
            Self::NoChildProcesses,
            Self::SignedImagesOnly,
            Self::NoDynamicCode,
        ]
    } else {
        &[]
    };

    /// Whether `self` and `other` are the same layer, whatever its details.
    fn same(self, other: Self) -> bool {
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
    }
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Off => f.write_str("off"),
            Self::NoNewPrivs => f.write_str("no-new-privs"),
            Self::Seccomp => f.write_str("seccomp"),
            Self::Landlock { abi, full: true } => write!(f, "landlock-v{abi}"),
            Self::Landlock { abi, full: false } => write!(f, "landlock-v{abi} (partial)"),
            Self::MemoryLimit => f.write_str("memory-limit"),
            Self::Seatbelt => f.write_str("seatbelt"),
            Self::AppSandbox => f.write_str("app-sandbox"),
            Self::NoNewDescriptors => f.write_str("no-new-descriptors"),
            Self::Job => f.write_str("job"),
            Self::LowIntegrity => f.write_str("low-integrity"),
            Self::RestrictedToken => f.write_str("restricted-token"),
            Self::NoChildProcesses => f.write_str("no-child-processes"),
            Self::SignedImagesOnly => f.write_str("signed-images-only"),
            Self::NoDynamicCode => f.write_str("no-dynamic-code"),
        }
    }
}

/// The layers an engine's sandbox enforces, in the order it applied them.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SandboxLevel {
    /// The layers that are on.
    pub layers: Vec<Layer>,
}

impl SandboxLevel {
    /// The sandbox turned off for diagnosis.
    #[must_use]
    pub fn off() -> Self {
        Self {
            layers: vec![Layer::Off],
        }
    }

    /// Whether it was turned off for diagnosis.
    #[must_use]
    pub fn is_off(&self) -> bool {
        self.has(Layer::Off)
    }

    /// Whether `layer` is on (any Landlock ABI counts as Landlock).
    #[must_use]
    pub fn has(&self, layer: Layer) -> bool {
        self.layers.iter().any(|on| on.same(layer))
    }

    /// The layers [`Layer::required`] on this OS that are missing.
    #[must_use]
    pub fn missing(&self) -> Vec<Layer> {
        Layer::required()
            .iter()
            .copied()
            .filter(|layer| !self.has(*layer))
            .collect()
    }

    /// The layers, for the log: `no-new-privs seccomp landlock-v8 …`, `off` or `none`.
    #[must_use]
    pub fn describe(&self) -> String {
        if self.layers.is_empty() {
            return "none".into();
        }
        let names: Vec<String> = self.layers.iter().map(ToString::to_string).collect();
        names.join(" ")
    }
}

/// The level an engine on this OS reports with every required layer on (the fake engine's).
#[cfg(any(test, feature = "test-support"))]
#[must_use]
pub fn complete_level() -> SandboxLevel {
    SandboxLevel {
        layers: Layer::required().to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_level_with_every_required_layer_should_miss_none() {
        assert_eq!(complete_level().missing(), Vec::<Layer>::new());
    }

    #[test]
    fn landlock_should_count_whatever_its_abi() {
        let level = SandboxLevel {
            layers: vec![Layer::Landlock { abi: 4, full: false }],
        };
        assert!(level.has(Layer::Landlock { abi: 0, full: true }));
    }

    #[test]
    fn describe_should_list_the_layers_in_order() {
        let level = SandboxLevel {
            layers: vec![Layer::Seccomp, Layer::Landlock { abi: 8, full: false }],
        };
        assert_eq!(level.describe(), "seccomp landlock-v8 (partial)");
    }
}
