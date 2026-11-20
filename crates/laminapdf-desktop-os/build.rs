//! Signs this crate's test binaries ad hoc on Intel Macs; the rule is in `adhoc_sign.rs`, which
//! the desktop app's build script includes too, for its own tests.

include!("adhoc_sign.rs");

fn main() {
    sign_ad_hoc_on_intel_macs();
}
