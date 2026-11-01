//! `cargo xtask verapdf [--dir <dir>]` (ADR 0018 tier 10): validates every generated
//! example against the profiles it claims with veraPDF, and fails on any violation.
//!
//! Without `--dir`, the `lamina` examples are run into `target/examples-out/` first (as
//! `cargo xtask examples` does). Each PDF's profiles are read from its XMP identification
//! (`pdfaid:part` + `pdfaid:conformance`, `pdfuaid:part`), so an example declares what it is
//! validated against by what it writes; a PDF that claims none is listed and skipped.
//!
//! veraPDF runs as its Java command-line application, an external process only (ADR 0019: it
//! is GPL-3.0-or-later / MPL-2.0, never linked or bundled). The pinned greenfield release
//! ([`VERAPDF_VERSION`](crate::doctor::VERAPDF_VERSION)) is downloaded once, its SHA-256
//! checked, and installed with its own installer's unattended mode into
//! `$RUNNER_TOOL_CACHE/lamina-verapdf` (CI) or `target/verapdf/` (locally). Java is
//! `$LAMINA_JAVA`, else Homebrew's `openjdk@21`, else `java` on `PATH`; veraPDF's
//! configuration goes in the cache folder, never the user's home.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::doctor::{VERAPDF_INSTALLER_SHA256, VERAPDF_VERSION};
use crate::step;
use crate::util::sha256_hex;
use crate::workspace::Workspace;

/// Where the report goes.
const REPORT_DIR: &str = "target/xtask/verapdf";

/// Homebrew's `openjdk@21` (keg-only, so not on `PATH`), tried after `$LAMINA_JAVA`.
const HOMEBREW_JAVA: &str = "/opt/homebrew/opt/openjdk@21/bin/java";

/// The veraPDF flavours of a PDF's XMP identification: `2b`, `2u`, `3b`, `ua1`.
pub(crate) fn flavours(pdf: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(pdf);
    let value = |tag: &str| {
        let open = format!("<{tag}>");
        let start = text.find(&open)? + open.len();
        let end = text.get(start..)?.find('<')? + start;
        text.get(start..end).map(str::trim)
    };
    let mut out = Vec::new();
    if let (Some(part), Some(level)) = (value("pdfaid:part"), value("pdfaid:conformance")) {
        out.push(format!("{part}{}", level.to_ascii_lowercase()));
    }
    if value("pdfuaid:part") == Some("1") {
        out.push("ua1".to_owned());
    }
    out
}

/// One file's verdict for one flavour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Verdict {
    pub(crate) file: String,
    pub(crate) flavour: String,
    pub(crate) compliant: bool,
    pub(crate) passed_rules: u32,
    pub(crate) failed_rules: u32,
    pub(crate) failed_checks: u32,
    /// The failed rules as `clause-testNumber`, such as `7.1-10`.
    pub(crate) failures: Vec<String>,
}

/// The `name="value"` attribute in `tag`.
fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!(" {name}=\"");
    let start = tag.find(&key)? + key.len();
    let end = tag.get(start..)?.find('"')? + start;
    tag.get(start..end)
}

/// The first `<name …>` start tag in `text`.
fn start_tag<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let start = text.find(&format!("<{name} "))?;
    let end = text.get(start..)?.find('>')? + start;
    text.get(start..=end)
}

/// The verdicts in veraPDF's machine-readable report (`--format mrr`), one per job.
pub(crate) fn parse_report(xml: &str, flavour: &str) -> Result<Vec<Verdict>> {
    let mut out = Vec::new();
    for job in xml.split("<job>").skip(1) {
        let name = job
            .split_once("<name>")
            .and_then(|(_, rest)| rest.split_once("</name>"))
            .map(|(name, _)| name.to_owned())
            .context("a veraPDF job without a file name")?;
        let file = Path::new(&name)
            .file_name()
            .map_or(name.clone(), |f| f.to_string_lossy().into_owned());
        let Some(report) = start_tag(job, "validationReport") else {
            bail!("veraPDF couldn't validate {file} ({flavour}):\n{job}");
        };
        let details = start_tag(job, "details").context("a veraPDF report without details")?;
        let number = |key: &str| attr(details, key).and_then(|v| v.parse().ok()).unwrap_or(0);
        let failures = job
            .split("<rule ")
            .skip(1)
            .filter_map(|rule| {
                let tag = format!("<rule {}", rule.split_once('>')?.0);
                if attr(&tag, "status") != Some("failed") {
                    return None;
                }
                Some(format!("{}-{}", attr(&tag, "clause")?, attr(&tag, "testNumber")?))
            })
            .collect();
        out.push(Verdict {
            file,
            flavour: flavour.to_owned(),
            compliant: attr(report, "isCompliant") == Some("true"),
            passed_rules: number("passedRules"),
            failed_rules: number("failedRules"),
            failed_checks: number("failedChecks"),
            failures,
        });
    }
    Ok(out)
}

/// The cache folder.
fn cache(ws: &Workspace) -> PathBuf {
    match env::var_os("RUNNER_TOOL_CACHE") {
        Some(dir) => PathBuf::from(dir).join("lamina-verapdf"),
        None => ws.root.join("target/verapdf"),
    }
}

/// The Java to run veraPDF with: `$LAMINA_JAVA`, Homebrew's `openjdk@21`, or `java` on `PATH`,
/// the first that runs at the version doctor pins.
fn java(ws: &Workspace) -> Result<String> {
    let tool = crate::doctor::java();
    let mut candidates = Vec::new();
    if let Some(java) = env::var_os("LAMINA_JAVA") {
        candidates.push(java.to_string_lossy().into_owned());
    }
    candidates.push(HOMEBREW_JAVA.to_owned());
    candidates.push("java".to_owned());
    let mut found = None;
    for candidate in candidates {
        let Ok(out) = Command::new(&candidate).arg("--version").current_dir(&ws.root).output() else {
            continue;
        };
        let version = String::from_utf8_lossy(&out.stdout).into_owned();
        if out.status.success() && tool.accepts(&version) {
            return Ok(candidate);
        }
        if out.status.success() {
            found.get_or_insert(version.lines().next().unwrap_or_default().to_owned());
        }
    }
    let probe = match found {
        Some(version) => crate::doctor::Probe::OffPin(version),
        None => crate::doctor::Probe::Missing,
    };
    bail!("{}", tool.problem(&probe))
}

/// The installer's unattended-install file: the command-line application only.
fn auto_install(target: &Path) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>\n\
         <AutomatedInstallation langpack=\"eng\">\n\
         <com.izforge.izpack.panels.htmlhello.HTMLHelloPanel id=\"welcome\"/>\n\
         <com.izforge.izpack.panels.target.TargetPanel id=\"install_dir\"><installpath>{}</installpath></com.izforge.izpack.panels.target.TargetPanel>\n\
         <com.izforge.izpack.panels.packs.PacksPanel id=\"sdk_pack_select\">\n\
         <pack index=\"0\" name=\"veraPDF GUI\" selected=\"true\"/>\n\
         <pack index=\"1\" name=\"veraPDF Batch files\" selected=\"true\"/>\n\
         <pack index=\"2\" name=\"veraPDF Validation model\" selected=\"false\"/>\n\
         <pack index=\"3\" name=\"veraPDF Documentation\" selected=\"false\"/>\n\
         <pack index=\"4\" name=\"veraPDF Sample Plugins\" selected=\"false\"/>\n\
         </com.izforge.izpack.panels.packs.PacksPanel>\n\
         <com.izforge.izpack.panels.install.InstallPanel id=\"install\"/>\n\
         <com.izforge.izpack.panels.finish.FinishPanel id=\"finish\"/>\n\
         </AutomatedInstallation>\n",
        target.display()
    )
}

/// Ensures the pinned veraPDF is installed; returns its command-line jar and the folder that
/// stands in for the user's home (where veraPDF keeps its configuration).
fn install(ws: &Workspace, java: &str) -> Result<(PathBuf, PathBuf)> {
    let dir = cache(ws).join(VERAPDF_VERSION);
    let home = dir.join("home");
    let inst = dir.join("install");
    let jar = inst.join(format!("bin/cli-{VERAPDF_VERSION}.jar"));
    let stamp = dir.join("SHA256");
    if jar.is_file() && fs::read_to_string(&stamp).is_ok_and(|s| s.trim() == VERAPDF_INSTALLER_SHA256) {
        return Ok((jar, home));
    }
    fs::create_dir_all(&home).with_context(|| format!("creating {}", home.display()))?;
    let zip_name = format!("verapdf-greenfield-{VERAPDF_VERSION}-installer.zip");
    let zip = dir.join(&zip_name);
    let ok = |path: &Path| -> Result<bool> {
        Ok(path.is_file()
            && sha256_hex(&fs::read(path).with_context(|| format!("reading {}", path.display()))?)
                == VERAPDF_INSTALLER_SHA256)
    };
    if !ok(&zip)? {
        let series = VERAPDF_VERSION.rsplit_once('.').map_or(VERAPDF_VERSION, |(s, _)| s);
        let url = format!("https://software.verapdf.org/releases/{series}/{zip_name}");
        // A per-process name, so two jobs sharing the cache never write the same partial file.
        let part = dir.join(format!("{zip_name}.{}.part", std::process::id()));
        crate::run(
            &dir,
            "curl",
            &["-fsSL", "--retry", "3", "-o", &part.to_string_lossy(), &url],
        )?;
        if !ok(&part)? {
            let _ = fs::remove_file(&part);
            bail!("{zip_name}: SHA-256 mismatch (expected {VERAPDF_INSTALLER_SHA256}); the download was discarded");
        }
        fs::rename(&part, &zip).with_context(|| format!("moving {}", part.display()))?;
    }
    let unpacked = dir.join("installer");
    for old in [&unpacked, &inst] {
        if old.exists() {
            fs::remove_dir_all(old).with_context(|| format!("removing {}", old.display()))?;
        }
    }
    crate::run(&dir, "unzip", &["-q", &zip_name, "-d", "installer"])?;
    let auto = dir.join("auto-install.xml");
    fs::write(&auto, auto_install(&inst)).with_context(|| format!("writing {}", auto.display()))?;
    let installer = unpacked.join(format!(
        "verapdf-greenfield-{VERAPDF_VERSION}/verapdf-izpack-installer-{VERAPDF_VERSION}.jar"
    ));
    let user_home = format!("-Duser.home={}", home.display());
    crate::run(
        &dir,
        java,
        &[
            &user_home,
            "-Djava.awt.headless=true",
            "-jar",
            &installer.to_string_lossy(),
            &auto.to_string_lossy(),
        ],
    )?;
    if !jar.is_file() {
        bail!("the veraPDF installer finished without {}", jar.display());
    }
    fs::write(&stamp, VERAPDF_INSTALLER_SHA256).with_context(|| format!("writing {}", stamp.display()))?;
    step(&format!("veraPDF {VERAPDF_VERSION} installed in {}", inst.display()));
    Ok((jar, home))
}

/// Runs veraPDF over `files` for one flavour.
fn validate(java: &str, jar: &Path, home: &Path, flavour: &str, files: &[PathBuf]) -> Result<Vec<Verdict>> {
    let app_home = jar.parent().and_then(Path::parent).unwrap_or(home);
    let out = Command::new(java)
        .arg(format!("-Duser.home={}", home.display()))
        .arg(format!("-Dapp.home={}", app_home.display()))
        .arg("-Djava.awt.headless=true")
        .arg("-cp")
        .arg(jar)
        .arg("org.verapdf.apps.GreenfieldCliWrapper")
        .args(["--flavour", flavour, "--format", "mrr"])
        .args(files)
        .output()
        .context("running veraPDF")?;
    let xml = String::from_utf8_lossy(&out.stdout);
    if !xml.contains("<jobs>") {
        bail!(
            "veraPDF ({flavour}) wrote no report ({}):\n{}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    parse_report(&xml, flavour)
}

/// The summary table (Markdown), one row per file and flavour.
pub(crate) fn summary(verdicts: &[Verdict], skipped: &[String]) -> String {
    let mut s = format!(
        "## veraPDF {VERAPDF_VERSION}\n\n| File | Profile | Result | Rules passed | Rules failed |\n| --- | --- | --- | --- | --- |\n"
    );
    for v in verdicts {
        let result = if v.compliant {
            "pass".to_owned()
        } else {
            format!("**fail** ({})", v.failures.join(", "))
        };
        let _ = writeln!(
            s,
            "| `{}` | {} | {result} | {} | {} |",
            v.file, v.flavour, v.passed_rules, v.failed_rules
        );
    }
    if !skipped.is_empty() {
        let _ = writeln!(s, "\nNo profile claimed (not validated): {}", skipped.join(", "));
    }
    s
}

/// The PDFs in `dir`, sorted.
fn pdfs_in(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "pdf"))
        .collect();
    files.sort();
    Ok(files)
}

/// `cargo xtask verapdf [--dir <dir>]`.
pub(crate) fn run(ws: &Workspace, dir: Option<&str>) -> Result<()> {
    let java = java(ws)?;
    let files = if let Some(d) = dir {
        pdfs_in(&ws.root.join(d))?
    } else {
        let out = ws.root.join(crate::examples::OUT_DIR);
        if out.exists() {
            fs::remove_dir_all(&out).with_context(|| format!("removing {}", out.display()))?;
        }
        crate::examples::run_all(ws, &out)?
    };
    let (jar, home) = install(ws, &java)?;
    // Files grouped by flavour, in a fixed order.
    let mut by_flavour: std::collections::BTreeMap<String, Vec<PathBuf>> = std::collections::BTreeMap::new();
    let mut skipped = Vec::new();
    for file in &files {
        let claimed = flavours(&fs::read(file).with_context(|| format!("reading {}", file.display()))?);
        let name = file
            .file_name()
            .map_or_else(String::new, |f| f.to_string_lossy().into_owned());
        if claimed.is_empty() {
            skipped.push(name);
        }
        for flavour in claimed {
            by_flavour.entry(flavour).or_default().push(file.clone());
        }
    }
    if by_flavour.is_empty() {
        bail!("no PDF in the set claims a PDF/A or PDF/UA profile");
    }
    let mut verdicts = Vec::new();
    for (flavour, files) in &by_flavour {
        verdicts.extend(validate(&java, &jar, &home, flavour, files)?);
    }
    let table = summary(&verdicts, &skipped);
    let report = ws.root.join(REPORT_DIR);
    fs::create_dir_all(&report).with_context(|| format!("creating {}", report.display()))?;
    fs::write(report.join("summary.md"), &table).context("writing the summary")?;
    if let Some(path) = env::var_os("GITHUB_STEP_SUMMARY") {
        let mut current = fs::read_to_string(&path).unwrap_or_default();
        current.push_str(&table);
        fs::write(&path, current).context("writing the job summary")?;
    }
    println!("{table}");
    let failed: Vec<String> = verdicts
        .iter()
        .filter(|v| !v.compliant)
        .map(|v| format!("{} ({})", v.file, v.flavour))
        .collect();
    if !failed.is_empty() {
        bail!("veraPDF found violations in: {}", failed.join(", "));
    }
    step(&format!(
        "verapdf: {} validations over {} files pass ({REPORT_DIR}/summary.md)",
        verdicts.len(),
        files.len() - skipped.len()
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flavours_come_from_the_xmp_identification() {
        let xmp =
            b"<pdfaid:part>2</pdfaid:part><pdfaid:conformance>U</pdfaid:conformance><pdfuaid:part>1</pdfuaid:part>";
        assert_eq!(flavours(xmp), ["2u", "ua1"]);
    }

    #[test]
    fn a_pdf_without_identification_claims_nothing() {
        assert_eq!(
            flavours(b"%PDF-1.7 <dc:format>application/pdf</dc:format>"),
            Vec::<String>::new()
        );
    }

    const REPORT: &str = r#"<report><jobs>
    <job><item size="1"><name>/x/a.pdf</name></item>
      <validationReport jobEndStatus="normal" isCompliant="true">
        <details passedRules="106" failedRules="0" passedChecks="9" failedChecks="0"></details>
      </validationReport></job>
    <job><item size="1"><name>/x/b.pdf</name></item>
      <validationReport jobEndStatus="normal" isCompliant="false">
        <details passedRules="104" failedRules="2" passedChecks="9" failedChecks="3">
          <rule specification="ISO 14289-1:2014" clause="7.1" testNumber="10" status="failed" failedChecks="1"></rule>
          <rule specification="ISO 14289-1:2014" clause="5" testNumber="1" status="failed" failedChecks="2"></rule>
        </details>
      </validationReport></job>
    </jobs></report>"#;

    #[test]
    fn the_report_gives_one_verdict_per_file() {
        let v = parse_report(REPORT, "ua1").unwrap();
        assert_eq!(
            v.iter()
                .map(|v| (v.file.as_str(), v.compliant, v.passed_rules))
                .collect::<Vec<_>>(),
            [("a.pdf", true, 106), ("b.pdf", false, 104)]
        );
    }

    #[test]
    fn a_failed_report_lists_its_rules() {
        let v = parse_report(REPORT, "ua1").unwrap();
        assert_eq!(
            (v[1].failures.clone(), v[1].failed_checks),
            (vec!["7.1-10".to_owned(), "5-1".to_owned()], 3)
        );
    }

    #[test]
    fn a_job_without_a_report_is_an_error() {
        assert!(parse_report("<jobs><job><name>/x/c.pdf</name><taskException/></job></jobs>", "2b").is_err());
    }

    #[test]
    fn the_summary_marks_failures() {
        let v = parse_report(REPORT, "ua1").unwrap();
        let s = summary(&v, &["hello.pdf".to_owned()]);
        assert!(s.contains("| `b.pdf` | ua1 | **fail** (7.1-10, 5-1) | 104 | 2 |") && s.contains("hello.pdf"));
    }

    #[test]
    fn the_installer_is_pinned_by_a_sha256() {
        assert!(
            VERAPDF_INSTALLER_SHA256.len() == 64 && VERAPDF_INSTALLER_SHA256.bytes().all(|b| b.is_ascii_hexdigit())
        );
    }
}
