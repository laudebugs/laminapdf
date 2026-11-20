// Included by this crate's build script and the desktop app's (`include!`), so the rule has one home.

/// Signs the binaries being linked ad hoc on Intel Macs, as the linker already does on Apple
/// silicon. Security-scoped bookmarks need a code-signed process (the agent that makes them reads
/// its signature), so without this the store build's tests fail there. Every binary a user runs is
/// signed again by Tauri's bundler or the store.
fn sign_ad_hoc_on_intel_macs() {
    if std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "macos")
        && std::env::var("CARGO_CFG_TARGET_ARCH").is_ok_and(|arch| arch == "x86_64")
    {
        println!("cargo:rustc-link-arg=-Wl,-adhoc_codesign");
    }
}
