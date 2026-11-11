# Dependabot failed every Cargo update

[Resolution](resolution.md).

## What we saw

The Dependabot run on 2026-11-11 failed. For every Cargo dependency it tried (`aes`, `pdfium-render`, `tauri-plugin-dialog`, `wasm-bindgen` and more) it logged `dependency_file_not_resolvable`: "failed to read `dependabot_tmp_dir/apps/*/src-tauri/Cargo.toml`".

## How to reproduce

Run Dependabot's Cargo update on main before the fix; any Cargo dependency fails the same way.

## Cause

The root `Cargo.toml` listed the Tauri apps' crates as the member `"apps/*/src-tauri"`. Cargo expands a wildcard anywhere in a member's path, but Dependabot expands one only at the path's end (as in `"crates/*"`) and read `apps/*/src-tauri` as a folder's literal name.
