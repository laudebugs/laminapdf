fn main() {
    // App commands go through an app manifest, so each needs an explicit `allow-*` grant in
    // capabilities/default.json: the permission model D2a adopts.
    let manifest = tauri_build::AppManifest::new().commands(&["plan", "tile", "stream_tiles", "metrics"]);
    if let Err(error) = tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest)) {
        println!("cargo:warning={error:#}");
        std::process::exit(1);
    }
}
