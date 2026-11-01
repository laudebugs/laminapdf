//! `cargo xtask fetch-pdfium` (ADR 0004 §3, todo C2b): downloads the pinned native PDFium build
//! (`bblanchon/pdfium-binaries`, `chromium/7881`) for this host, checks its SHA-256 and unpacks
//! the library and its licence notices. `laminapdf-render-pdfium` binds it at runtime from the
//! folder this prints (`LAMINA_PDF_PDFIUM_DIR`).
//!
//! The cache is `$LAMINA_PDF_PDFIUM_CACHE`, else the CI runner's tool cache
//! (`$RUNNER_TOOL_CACHE/lamina-pdfium`, kept between jobs), else `target/pdfium/`. A cached
//! archive is re-hashed on every run; a mismatch is discarded and downloaded again. Nothing
//! from the archive is executed.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::step;
use crate::util::sha256_hex;
use crate::workspace::Workspace;

/// The pinned release (the build `pdfium-render`'s `pdfium_7881` feature targets).
pub(crate) const TAG: &str = "chromium/7881";

/// One downloadable build: OS, CPU, archive name, its SHA-256, and the archive folder holding
/// the library. The macOS arm64, macOS x64 and Linux x64 hashes are the hayro-parity spike's
/// (C1a); Linux arm64 and Windows x64 were recorded when C2b added them (2026-10-31).
const ASSETS: [(&str, &str, &str, &str, &str); 5] = [
    (
        "macos",
        "aarch64",
        "pdfium-mac-arm64.tgz",
        "52e94ca5aa8847934330daf3f8150c190682c5ca93831468794f8b90d4392e40",
        "lib",
    ),
    (
        "macos",
        "x86_64",
        "pdfium-mac-x64.tgz",
        "6dedf83990e0e3d6b7c93c9e7589c5a126b0ae14b7464d76120cff7a26afb18b",
        "lib",
    ),
    (
        "linux",
        "x86_64",
        "pdfium-linux-x64.tgz",
        "1470e21b8b4a3b4ad7f85684e2da11d94f3b69a86d81dee11b9b6709d927ac1d",
        "lib",
    ),
    (
        "linux",
        "aarch64",
        "pdfium-linux-arm64.tgz",
        "ee7f7b7d5468958336a818c1cd580bdd20972846b7377b13f9a923d92d1d4674",
        "lib",
    ),
    (
        "windows",
        "x86_64",
        "pdfium-win-x64.tgz",
        "73cc0de638ac2095e7445bf56a38200a5b7c7ca0e9f4ba144598f2457377ac08",
        "bin",
    ),
];

/// The cache folder.
fn cache(ws: &Workspace) -> PathBuf {
    if let Some(dir) = env::var_os("LAMINA_PDF_PDFIUM_CACHE") {
        return PathBuf::from(dir);
    }
    if let Some(dir) = env::var_os("RUNNER_TOOL_CACHE") {
        return PathBuf::from(dir).join("lamina-pdfium");
    }
    ws.root.join("target/pdfium")
}

/// Ensures the pinned build for this host is unpacked; returns the folder with the library.
pub(crate) fn fetch(ws: &Workspace) -> Result<PathBuf> {
    let Some(&(_, _, asset, sha, lib)) = ASSETS
        .iter()
        .find(|(os, arch, ..)| *os == env::consts::OS && *arch == env::consts::ARCH)
    else {
        bail!(
            "no pinned PDFium build for {}-{}; add one to ASSETS in tools/xtask/src/pdfium.rs",
            env::consts::OS,
            env::consts::ARCH
        );
    };
    let dir = cache(ws)
        .join(TAG.replace('/', "-"))
        .join(asset.trim_end_matches(".tgz"));
    let archive = dir.join(asset);
    let stamp = dir.join("SHA256");
    let unpacked = dir.join(lib);
    if unpacked.is_dir() && fs::read_to_string(&stamp).is_ok_and(|s| s.trim() == sha) && verify(&archive, sha)? {
        println!("LAMINA_PDF_PDFIUM_DIR={}", unpacked.display());
        return Ok(unpacked);
    }
    fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    if !verify(&archive, sha)? {
        // A per-process name, so two jobs sharing the cache never write the same partial file.
        let part = dir.join(format!("{asset}.{}.part", std::process::id()));
        let url = format!("https://github.com/bblanchon/pdfium-binaries/releases/download/{TAG}/{asset}");
        crate::run(
            &dir,
            "curl",
            &["-fsSL", "--retry", "3", "-o", &part.to_string_lossy(), &url],
        )?;
        if !verify(&part, sha)? {
            let _ = fs::remove_file(&part);
            bail!("{asset}: SHA-256 mismatch (expected {sha}); the download was discarded");
        }
        fs::rename(&part, &archive).with_context(|| format!("moving {}", part.display()))?;
    }
    for old in [lib, "licenses"] {
        let path = dir.join(old);
        if path.exists() {
            fs::remove_dir_all(&path).with_context(|| format!("removing {}", path.display()))?;
        }
    }
    // Only the library, its licence notices (ADR 0004 §3: shipped with every bundle) and the
    // version file.
    crate::run(&dir, "tar", &["-xzf", asset, lib, "licenses", "LICENSE", "VERSION"])?;
    fs::write(&stamp, sha).with_context(|| format!("writing {}", stamp.display()))?;
    step(&format!("PDFium {TAG} ({asset}) unpacked"));
    println!("LAMINA_PDF_PDFIUM_DIR={}", unpacked.display());
    Ok(unpacked)
}

/// Whether `path` exists with SHA-256 `sha`.
fn verify(path: &Path, sha: &str) -> Result<bool> {
    if !path.is_file() {
        return Ok(false);
    }
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(sha256_hex(&bytes) == sha)
}

#[cfg(test)]
mod tests {
    use super::ASSETS;

    /// xtask can't depend on laminapdf-render-pdfium (it would build PDFium's bindings for every
    /// task), so the build tag and the variable are checked against the crate's source.
    #[test]
    fn the_tag_and_variable_match_the_crate() {
        let crate_src = include_str!("../../../crates/laminapdf-render-pdfium/src/lib.rs");
        assert!(crate_src.contains(&format!("pub const PDFIUM_BUILD: &str = \"{}\";", super::TAG)));
        assert!(crate_src.contains(&format!(
            "pub const PDFIUM_DIR_ENV: &str = \"{}\";",
            crate::render::PDFIUM_DIR_ENV
        )));
    }

    #[test]
    fn every_asset_is_pinned_by_a_sha256() {
        for (os, arch, asset, sha, _) in ASSETS {
            assert_eq!(sha.len(), 64, "{os}-{arch} {asset}");
            assert!(sha.bytes().all(|b| b.is_ascii_hexdigit()), "{asset}");
        }
    }
}
