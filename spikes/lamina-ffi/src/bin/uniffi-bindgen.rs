//! The UniFFI bindings generator, built from the pinned `uniffi` (`cargo run --features bindgen
//! --bin uniffi-bindgen -- generate --library <lib> --language kotlin|swift --out-dir <dir>`).
fn main() {
    uniffi::uniffi_bindgen_main();
}
