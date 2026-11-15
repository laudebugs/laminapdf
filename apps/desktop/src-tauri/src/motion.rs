//! The system's reduce-motion setting where the webview can't read it (motion plan §5).
//! `WebView2` and `WKWebView` report `prefers-reduced-motion` from the OS, but `WebKitGTK` on GTK 3
//! doesn't follow GNOME's "Reduce animations" under Wayland (GTK issue 5813). On Linux the main
//! process reads that setting itself, `org.gnome.desktop.interface enable-animations`, through
//! `gsettings` (dconf over D-Bus), and the webview adds it to its own reading. The engine never
//! reads it: it holds no settings.

/// Whether the system asks for reduced motion: `Some(true)` when GNOME's animations are off, and
/// `None` where the webview reads the setting itself or it can't be read (no GNOME, no gsettings).
pub(crate) fn system_reduces_motion() -> Option<bool> {
    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "enable-animations"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        reduces_motion(&String::from_utf8_lossy(&output.stdout))
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// What `gsettings get … enable-animations` printed, as "reduce motion": animations off asks for it.
#[cfg_attr(
    not(any(target_os = "linux", test)),
    expect(dead_code, reason = "only Linux reads gsettings")
)]
fn reduces_motion(printed: &str) -> Option<bool> {
    match printed.trim() {
        "true" => Some(false),
        "false" => Some(true),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animations_off_should_ask_for_reduced_motion() {
        assert_eq!(reduces_motion("false\n"), Some(true));
        assert_eq!(reduces_motion("true\n"), Some(false));
    }

    #[test]
    fn anything_else_should_read_as_unknown() {
        for printed in ["", "No such schema “org.gnome.desktop.interface”\n", "1"] {
            assert_eq!(reduces_motion(printed), None, "{printed:?}");
        }
    }
}
