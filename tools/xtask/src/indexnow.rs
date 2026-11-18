//! IndexNow pings from the production deploys (todo W3e): Bing and the engines that share its
//! IndexNow submissions hear about each page a deploy added or changed, instead of waiting for
//! their next crawl of the sitemap ([IndexNow documentation](https://www.indexnow.org/documentation)).
//!
//! Two steps around each production deploy:
//!
//! - `cargo xtask indexnow snapshot <sitemap> --out <file>`, before the deploy, records the live
//!   sitemap: each page's address and `<lastmod>`, a sitemap index followed to its sitemaps.
//! - `cargo xtask indexnow submit --before <snapshot|sitemap> --after <sitemap> --key-file <file>
//!   [--dry-run]`, after it, compares the two and posts the pages that are new or whose
//!   `<lastmod>` moved to `api.indexnow.org`, with the site's key, which each site serves at
//!   `/<key>.txt` from its `public/` folder (the key is public by design).
//!
//! A sitemap is a URL (fetched with `curl`) or a file; a file's sitemap index finds its sitemaps
//! beside it, by their addresses' paths, as in a site's `dist/`. Every failure once the
//! arguments are read (a fetch, an unreadable sitemap, a refused submission) is a warning
//! (`::warning::` under GitHub Actions) and the command still succeeds: a ping is a hint to the
//! engines, never a reason to fail a deploy. A snapshot that can't be read is empty, so every page
//! counts as new.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

/// Where submissions go: IndexNow's shared endpoint, which passes them to every engine that takes
/// part.
const ENDPOINT: &str = "https://api.indexnow.org/IndexNow";

/// The most addresses one submission may carry.
const MAX_URLS: usize = 10_000;

/// How many sitemaps a sitemap index may name before the rest are ignored: a bound on what one
/// snapshot fetches.
const MAX_SITEMAPS: usize = 50;

/// What the command does.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Task {
    /// Record a sitemap's pages.
    Snapshot { sitemap: String, out: PathBuf },
    /// Post the pages a deploy added or changed.
    Submit {
        before: String,
        after: String,
        key_file: PathBuf,
        dry_run: bool,
    },
}

/// A sitemap's pages: each address with its `<lastmod>`, if it has one.
#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Snapshot {
    pub(crate) urls: BTreeMap<String, Option<String>>,
}

/// Why a page goes in a submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Change {
    /// The page wasn't in the sitemap before.
    Added,
    /// Its `<lastmod>` moved.
    Changed,
}

impl Change {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Changed => "changed",
        }
    }
}

/// Reads `indexnow snapshot …` or `indexnow submit …`.
pub(crate) fn parse_args(args: &[String]) -> Result<Task> {
    let value = |name: &str| -> Result<Option<String>> {
        match args.iter().position(|a| a == name) {
            None => Ok(None),
            Some(at) => args
                .get(at + 1)
                .filter(|v| !v.starts_with("--"))
                .cloned()
                .map(Some)
                .with_context(|| format!("{name} needs a value")),
        }
    };
    let need = |name: &str| -> Result<String> { value(name)?.with_context(|| format!("indexnow: {name} is required")) };
    match args.first().map(String::as_str) {
        Some("snapshot") => {
            let sitemap = args
                .get(1)
                .filter(|a| !a.starts_with("--"))
                .cloned()
                .context("indexnow snapshot: name the sitemap (a URL or a file)")?;
            Ok(Task::Snapshot {
                sitemap,
                out: PathBuf::from(need("--out")?),
            })
        }
        Some("submit") => Ok(Task::Submit {
            before: need("--before")?,
            after: need("--after")?,
            key_file: PathBuf::from(need("--key-file")?),
            dry_run: args.iter().any(|a| a == "--dry-run"),
        }),
        _ => bail!(
            "usage: indexnow snapshot <sitemap> --out <file> | indexnow submit --before <snapshot|sitemap> --after <sitemap> --key-file <file> [--dry-run]"
        ),
    }
}

/// Runs the command. Only bad arguments and an unreadable key file fail it.
pub(crate) fn run(command: &Task) -> Result<()> {
    match command {
        Task::Snapshot { sitemap, out } => {
            let snapshot = read_sitemap(sitemap).unwrap_or_else(|e| {
                warn(&format!(
                    "indexnow: can't read the sitemap {sitemap} ({e:#}); every page will count as new"
                ));
                Snapshot::default()
            });
            let json = serde_json::to_string_pretty(&snapshot)?;
            std::fs::write(out, json + "\n").with_context(|| format!("writing {}", out.display()))?;
            println!("indexnow: {} pages in {sitemap}", snapshot.urls.len());
            Ok(())
        }
        Task::Submit {
            before,
            after,
            key_file,
            dry_run,
        } => submit(before, after, key_file, *dry_run),
    }
}

fn submit(before: &str, after: &str, key_file: &Path, dry_run: bool) -> Result<()> {
    let key = read_key(key_file)?;
    let before = read_before(before).unwrap_or_else(|e| {
        warn(&format!(
            "indexnow: can't read {before} ({e:#}); every page counts as new"
        ));
        Snapshot::default()
    });
    let after = match read_sitemap(after) {
        Ok(after) => after,
        Err(e) => {
            warn(&format!(
                "indexnow: can't read the new sitemap {after} ({e:#}); nothing submitted"
            ));
            return Ok(());
        }
    };
    let changes = changes(&before, &after);
    for (url, change) in &changes {
        println!("{} {url}", change.as_str());
    }
    if changes.is_empty() {
        println!("indexnow: no page added or changed");
        return Ok(());
    }
    let urls: Vec<&str> = changes.iter().map(|(url, _)| url.as_str()).collect();
    let host = match host_of(&urls) {
        Ok(host) => host,
        Err(e) => {
            warn(&format!("indexnow: {e:#}; nothing submitted"));
            return Ok(());
        }
    };
    for chunk in urls.chunks(MAX_URLS) {
        let body = request_body(&host, &key, chunk);
        if dry_run {
            println!(
                "indexnow: dry run, would post {} pages to {ENDPOINT}:\n{body}",
                chunk.len()
            );
            continue;
        }
        match post(&body) {
            Ok(status) if status == 200 || status == 202 => {
                println!("indexnow: {ENDPOINT} accepted {} pages (HTTP {status})", chunk.len());
            }
            Ok(status) => warn(&format!(
                "indexnow: {ENDPOINT} answered HTTP {status} ({})",
                meaning(status)
            )),
            Err(e) => warn(&format!("indexnow: posting to {ENDPOINT} failed ({e:#})")),
        }
    }
    Ok(())
}

/// The pages of `after` that `before` lacks, or whose `<lastmod>` differs, in address order.
pub(crate) fn changes(before: &Snapshot, after: &Snapshot) -> Vec<(String, Change)> {
    after
        .urls
        .iter()
        .filter_map(|(url, lastmod)| match before.urls.get(url) {
            None => Some((url.clone(), Change::Added)),
            Some(old) if old != lastmod => Some((url.clone(), Change::Changed)),
            Some(_) => None,
        })
        .collect()
}

/// The JSON IndexNow takes: `host`, `key`, `keyLocation` (the key file on that host) and
/// `urlList`.
pub(crate) fn request_body(host: &str, key: &str, urls: &[&str]) -> String {
    serde_json::json!({
        "host": host,
        "key": key,
        "keyLocation": format!("https://{host}/{key}.txt"),
        "urlList": urls,
    })
    .to_string()
}

/// The one host every address is on: IndexNow takes a submission's addresses from the key's host
/// only.
fn host_of(urls: &[&str]) -> Result<String> {
    let host = |url: &str| -> Option<String> {
        let rest = url.strip_prefix("https://")?;
        let end = rest.find('/').unwrap_or(rest.len());
        Some(rest.get(..end)?.to_owned()).filter(|h| !h.is_empty())
    };
    let first = urls
        .first()
        .and_then(|u| host(u))
        .context("an address isn't https://")?;
    if let Some(other) = urls.iter().find(|u| host(u).as_deref() != Some(first.as_str())) {
        bail!("{other} isn't on {first}, the host the key belongs to");
    }
    Ok(first)
}

/// The key a key file holds: its name is `<key>.txt` and its text the key (IndexNow's
/// rules: 8 to 128 letters, digits and hyphens).
fn read_key(path: &Path) -> Result<String> {
    let key = path
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|_| path.extension().is_some_and(|e| e.eq_ignore_ascii_case("txt")))
        .with_context(|| format!("{} isn't a <key>.txt file", path.display()))?;
    let valid = (8..=128).contains(&key.len()) && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
    if !valid {
        bail!("{}: a key is 8 to 128 letters, digits and hyphens", path.display());
    }
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    if text.trim() != key {
        bail!("{} doesn't hold its own name, {key}", path.display());
    }
    Ok(key.to_owned())
}

/// The pages before the deploy: a snapshot (`.json`) or a sitemap.
fn read_before(source: &str) -> Result<Snapshot> {
    if Path::new(source)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("json"))
    {
        let text = std::fs::read_to_string(source)?;
        return Ok(serde_json::from_str(&text)?);
    }
    read_sitemap(source)
}

/// A sitemap's pages, a sitemap index followed to its sitemaps.
pub(crate) fn read_sitemap(source: &str) -> Result<Snapshot> {
    let mut snapshot = Snapshot::default();
    let text = fetch(source, source)?;
    let parsed = parse(&text);
    for (url, lastmod) in parsed.urls {
        snapshot.urls.insert(url, lastmod);
    }
    for child in parsed.sitemaps.iter().take(MAX_SITEMAPS) {
        let text = fetch(child, source)?;
        for (url, lastmod) in parse(&text).urls {
            snapshot.urls.insert(url, lastmod);
        }
    }
    Ok(snapshot)
}

/// The text of `source`, a URL or a file. A sitemap an index names (`source` is that index) is
/// read from the address itself when the index is a URL, else from the file beside the index
/// that has the address's file name.
fn fetch(source: &str, index: &str) -> Result<String> {
    let remote = |s: &str| s.starts_with("https://") || s.starts_with("http://");
    if remote(source) && remote(index) {
        return curl_get(source);
    }
    let path = if source == index {
        PathBuf::from(source)
    } else {
        let name = source.rsplit('/').next().unwrap_or_default();
        Path::new(index).with_file_name(name)
    };
    std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))
}

/// A sitemap's entries: `<url>` pages with their `<lastmod>`, and the `<sitemap>`s an index
/// names. Only these elements are read (sitemaps.org's protocol); XML entities in addresses are
/// decoded.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Parsed {
    pub(crate) urls: Vec<(String, Option<String>)>,
    pub(crate) sitemaps: Vec<String>,
}

pub(crate) fn parse(text: &str) -> Parsed {
    let mut parsed = Parsed::default();
    for (tag, sink) in [("url", true), ("sitemap", false)] {
        for entry in elements(text, tag) {
            let Some(loc) = elements(entry, "loc").next().map(unescape) else {
                continue;
            };
            if sink {
                let lastmod = elements(entry, "lastmod").next().map(|d| d.trim().to_owned());
                parsed.urls.push((loc, lastmod));
            } else {
                parsed.sitemaps.push(loc);
            }
        }
    }
    parsed
}

/// The contents of every `<tag>…</tag>` in `text` (also `<tag attr…>`), outermost only.
fn elements<'t>(text: &'t str, tag: &str) -> impl Iterator<Item = &'t str> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut rest = text;
    std::iter::from_fn(move || {
        loop {
            let at = rest.find(&open)?;
            let after = rest.get(at + open.len()..)?;
            // `<url>` or `<url …>`, not `<urlset>`.
            if !after.starts_with(['>', ' ', '\t', '\n', '\r']) {
                rest = after;
                continue;
            }
            let body_start = after.find('>')? + 1;
            let body = after.get(body_start..)?;
            let end = body.find(&close)?;
            rest = body.get(end + close.len()..)?;
            return body.get(..end);
        }
    })
}

/// XML's five entities decoded, and surrounding white space trimmed.
fn unescape(text: &str) -> String {
    text.trim()
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// The body of a `GET` to `url`, failing on anything but a 2xx answer.
fn curl_get(url: &str) -> Result<String> {
    let out = Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--fail",
            "--location",
            "--max-time",
            "30",
            url,
        ])
        .output()
        .context("running curl")?;
    if !out.status.success() {
        bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8(out.stdout)?)
}

/// Posts `body` to [`ENDPOINT`] and returns the HTTP status.
fn post(body: &str) -> Result<u16> {
    let mut child = Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--max-time",
            "30",
            "--output",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
            "--write-out",
            "%{http_code}",
            "--header",
            "Content-Type: application/json; charset=utf-8",
            "--data-binary",
            "@-",
            ENDPOINT,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("running curl")?;
    child
        .stdin
        .take()
        .context("curl's standard input")?
        .write_all(body.as_bytes())?;
    let out = child.wait_with_output()?;
    if !out.status.success() {
        bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().parse()?)
}

/// What IndexNow's documentation says an answer means.
const fn meaning(status: u16) -> &'static str {
    match status {
        400 => "bad request",
        403 => "the key file wasn't found or doesn't hold the key",
        422 => "an address isn't on the key's host, or the key doesn't match",
        429 => "too many requests",
        _ => "unexpected",
    }
}

/// A warning: GitHub Actions' annotation in a workflow, else a line on standard error.
fn warn(message: &str) {
    if std::env::var_os("GITHUB_ACTIONS").is_some() {
        println!("::warning::{message}");
    } else {
        eprintln!("warning: {message}");
    }
}

#[cfg(test)]
mod tests;
