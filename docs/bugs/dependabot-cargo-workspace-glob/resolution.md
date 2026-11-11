# Resolution: Dependabot failed every Cargo update

[Diagnosis](detail.md).

## The fix

The root `Cargo.toml` lists the desktop crate by its path, `"apps/desktop/src-tauri"`. A new Tauri app adds its own line. ADR 0008 and the code-organisation guide say so.

## How it was checked

`cargo metadata` still lists `laminapdf-desktop` with its engine and protocol crates. Dependabot's next weekly run is the real check.
