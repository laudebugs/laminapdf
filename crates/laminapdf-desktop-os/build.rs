//! Signs this crate's test binaries ad hoc on Intel Macs, as the linker already does on Apple
//! silicon. Security-scoped bookmarks need a code-signed process (the agent that makes them reads
//! its signature), and the `x86_64` linker leaves binaries unsigned, so without this the
//! `security-scoped` tests fail there while every binary a user runs is signed. The desktop app's
//! build script does the same for its own tests.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "macos")
        && std::env::var("CARGO_CFG_TARGET_ARCH").is_ok_and(|arch| arch == "x86_64")
    {
        println!("cargo:rustc-link-arg=-Wl,-adhoc_codesign");
    }
}
