fn main() {
    // App commands go through an app manifest, so each needs an `allow-*` grant in
    // capabilities/default.json (the permission model D2a adopts).
    let manifest = tauri_build::AppManifest::new().commands(&[
        "plan",
        "print_webview",
        "print_native_raster",
        "print_native_vector",
        "webview_ready",
        "report",
    ]);
    if let Err(error) = tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest)) {
        println!("cargo:warning={error:#}");
        std::process::exit(1);
    }
}
