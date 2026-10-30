//! `cargo xtask docs-links --external`: every `http(s)` URL in the tracked Markdown answers.
//!
//! The nightly lane's link check (todo X3c). It needs the network and remote hosts are flaky, so
//! it never runs in the PR lanes (ADR 0018 §2). Each distinct URL (its `#fragment` dropped) is
//! fetched once with `curl`: a `HEAD`, then a one-byte ranged `GET` when the host refuses `HEAD`.
//! Timeouts, connection errors, 429 and 5xx answers are retried once after a pause; a URL that
//! passes only on the retry is reported as flaky, never hidden (ADR 0018 §4). Hosts are checked in
//! parallel, but the URLs of one host one at a time with a pause between them (the rate limit);
//! after [`HOST_GIVE_UP`] URLs of one host in a row fail without an HTTP answer, the rest of that
//! host is reported dead unfetched, so one stalled host cannot outlast the job's timeout.
//! URLs under a prefix in [`ALLOWLIST`] (hosts known to refuse bots) are listed, not fetched.
//!
//! The report (every URL with its outcome) goes to stdout, `target/nightly/links/report.json`
//! and `report.md`, and the job summary; `flaky=true|false` goes to `$GITHUB_OUTPUT`. The run
//! fails when any URL is dead.

use std::collections::{BTreeMap, VecDeque};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde_json::json;

use super::{markdown_files, scan};
use crate::tracked::Tracked;
use crate::util::{github_output, md_cell, summary};

/// The allowlist, relative to the repository root.
pub(crate) const ALLOWLIST: &str = ".config/link-check.toml";

/// Where the reports go, relative to the repository root.
const REPORT_DIR: &str = "target/nightly/links";

/// Hosts checked at once.
const WORKERS: usize = 8;

/// Consecutive URLs of one host failing without an HTTP answer before the rest are skipped.
const HOST_GIVE_UP: usize = 3;

/// The most `file:line` sites listed per URL in the Markdown report.
const SITES_SHOWN: usize = 3;

/// How the checks are paced: the per-host rate limit and the retry policy.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Pacing {
    /// The pause between two requests to the same host.
    pub(crate) per_host: Duration,
    /// Retries after a transient failure (so `retries + 1` attempts at most).
    pub(crate) retries: u32,
    /// The pause before retry `n` (1-based) is `n * backoff`.
    pub(crate) backoff: Duration,
}

impl Pacing {
    /// The nightly lane's pacing.
    pub(crate) const NIGHTLY: Self = Self {
        per_host: Duration::from_millis(1000),
        // ADR 0018 §4: one retry, only to report flakiness.
        retries: 1,
        backoff: Duration::from_secs(5),
    };
}

/// `true` for an `http:` or `https:` URL (case-insensitive scheme).
pub(crate) fn is_http(target: &str) -> bool {
    let lower = target.get(..8).unwrap_or(target).to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// The bare `http(s)` URLs in a run of text, as GitHub's autolink extension finds them: a URL
/// starts after whitespace, `(` or a line start, ends at whitespace or `<`, and loses trailing
/// punctuation (`?!.,:;*_~'"`) and any unbalanced closing `)`.
pub(crate) fn bare_urls(text: &str) -> impl Iterator<Item = &str> {
    text.match_indices("http")
        .filter(move |(at, _)| {
            let starts_word = text
                .get(..*at)
                .and_then(|head| head.chars().next_back())
                .is_none_or(|c| c.is_whitespace() || matches!(c, '(' | '*' | '_' | '~'));
            let tail = text.get(*at..).unwrap_or_default();
            starts_word && is_http(tail)
        })
        .filter_map(move |(at, _)| {
            let tail = text.get(at..)?;
            let end = tail.find(|c: char| c.is_whitespace() || c == '<').unwrap_or(tail.len());
            let mut url = tail.get(..end)?;
            loop {
                let trimmed = url.trim_end_matches(['?', '!', '.', ',', ':', ';', '*', '_', '~', '\'', '"']);
                let unbalanced = trimmed.ends_with(')') && trimmed.matches(')').count() > trimmed.matches('(').count();
                let trimmed = if unbalanced {
                    trimmed.get(..trimmed.len() - 1)?
                } else {
                    trimmed
                };
                if trimmed.len() == url.len() {
                    break;
                }
                url = trimmed;
            }
            // GFM needs a domain after the scheme (`https://…` or `https://”` is not a link).
            url.split_once("://")
                .is_some_and(|(_, rest)| rest.starts_with(|c: char| c.is_ascii_alphanumeric()))
                .then_some(url)
        })
}

/// `url` without its `#fragment`: anchors on other sites are not checked.
fn without_fragment(url: &str) -> &str {
    url.split_once('#').map_or(url, |(base, _)| base)
}

/// The lowercased host (and port) of an `http(s)` URL, used to group and pace requests.
fn host(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = rest.get(..end).unwrap_or(rest);
    // Drop any `user:password@`.
    let host = authority.rsplit_once('@').map_or(authority, |(_, host)| host);
    host.to_ascii_lowercase()
}

/// Where a URL is linked from: `file:line`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Site {
    pub(crate) file: String,
    pub(crate) line: usize,
}

/// Every distinct external URL (fragment dropped) in `files`, with the places that link to it.
pub(crate) fn collect(root: &Path, files: &[std::path::PathBuf]) -> Result<BTreeMap<String, Vec<Site>>> {
    let mut urls: BTreeMap<String, Vec<Site>> = BTreeMap::new();
    for file in files {
        let text = fs::read_to_string(root.join(file)).with_context(|| format!("reading {}", file.display()))?;
        let file = file.to_string_lossy().replace('\\', "/");
        for link in scan(&text).external {
            urls.entry(without_fragment(&link.target).to_owned())
                .or_default()
                .push(Site {
                    file: file.clone(),
                    line: link.line,
                });
        }
    }
    Ok(urls)
}

/// One allowlist entry: URLs starting with `prefix` are not fetched, for `reason`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Allow {
    pub(crate) prefix: String,
    pub(crate) reason: String,
}

/// Parses the allowlist: `[[allow]]` tables with a `prefix` (an `http(s)` URL prefix) and a
/// non-empty `reason`.
pub(crate) fn parse_allowlist(text: &str) -> Result<Vec<Allow>> {
    let table: toml::Table = text.parse().context("parsing the link-check allowlist")?;
    if let Some(key) = table.keys().find(|key| *key != "allow") {
        bail!("{ALLOWLIST}: unknown key `{key}` (only [[allow]] tables)");
    }
    let Some(entries) = table.get("allow") else {
        return Ok(Vec::new());
    };
    let entries = entries
        .as_array()
        .with_context(|| format!("{ALLOWLIST}: `allow` must be an array of tables ([[allow]])"))?;
    entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let field = |key: &str| {
                entry
                    .get(key)
                    .and_then(toml::Value::as_str)
                    .filter(|value| !value.trim().is_empty())
                    .map(str::to_owned)
                    .with_context(|| format!("{ALLOWLIST}: [[allow]] #{} needs a non-empty `{key}`", i + 1))
            };
            let prefix = field("prefix")?;
            if !is_http(&prefix) {
                bail!(
                    "{ALLOWLIST}: [[allow]] #{}: `prefix` must start with http:// or https://",
                    i + 1
                );
            }
            Ok(Allow {
                prefix,
                reason: field("reason")?,
            })
        })
        .collect()
}

/// `true` when `prefix` covers `url`: a case-insensitive prefix match, where a prefix ending in
/// `/` also covers the URL without that slash (`…/lamina/` covers `…/lamina`).
fn covers(prefix: &str, url: &str) -> bool {
    let starts = |p: &str| url.get(..p.len()).is_some_and(|head| head.eq_ignore_ascii_case(p));
    starts(prefix)
        || prefix
            .strip_suffix('/')
            .is_some_and(|p| p.len() == url.len() && starts(p))
}

/// The first allowlist entry covering `url`.
fn allowed<'a>(allowlist: &'a [Allow], url: &str) -> Option<&'a Allow> {
    allowlist.iter().find(|allow| covers(&allow.prefix, url))
}

/// The request method of one probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Method {
    Head,
    /// A `GET` of the first byte only (`Range: bytes=0-0`).
    Get,
}

/// What one request returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Probe {
    /// The final HTTP status, after redirects.
    Status(u16),
    /// No HTTP answer: DNS, connection, TLS, timeout or too many redirects (curl's message).
    Network(String),
}

/// How a status is treated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Ok,
    /// Worth asking again later: rate limits and server errors.
    Transient,
    /// The host may refuse `HEAD` only: ask again with `GET`.
    TryGet,
    Dead,
}

fn classify(method: Method, status: u16) -> Class {
    match status {
        200..=399 => Class::Ok,
        // Some servers answer HEAD with 400/403/404/405/501 while GET works.
        400 | 403 | 404 | 405 | 501 if method == Method::Head => Class::TryGet,
        408 | 425 | 429 | 500..=599 => Class::Transient,
        _ => Class::Dead,
    }
}

/// The outcome of checking one URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// It answered `status`; `attempts > 1` means it passed only on a retry (flaky).
    Alive { status: u16, attempts: u32 },
    /// Dead for good: the last answer, after every retry.
    Dead { last: Probe, attempts: u32 },
    /// Not fetched: covered by the allowlist.
    Allowed { reason: String },
}

impl Outcome {
    fn is_dead(&self) -> bool {
        matches!(self, Self::Dead { .. })
    }

    fn is_flaky(&self) -> bool {
        matches!(self, Self::Alive { attempts, .. } if *attempts > 1)
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Alive { attempts: 1, .. } => "ok",
            Self::Alive { .. } => "flaky",
            Self::Dead { .. } => "dead",
            Self::Allowed { .. } => "allowed",
        }
    }

    fn detail(&self) -> String {
        match self {
            Self::Alive { status, attempts: 1 } => status.to_string(),
            Self::Alive { status, attempts } => format!("{status} on attempt {attempts}"),
            Self::Dead {
                last: Probe::Status(status),
                attempts,
            } => format!("{status} after {attempts} attempt(s)"),
            Self::Dead {
                last: Probe::Network(message),
                attempts: 0,
            } => message.clone(),
            Self::Dead {
                last: Probe::Network(message),
                attempts,
            } => format!("{message} after {attempts} attempt(s)"),
            Self::Allowed { reason } => reason.clone(),
        }
    }
}

/// Checks one URL with `probe`, retrying transient failures per `pacing`; `sleep` waits (tests
/// pass a no-op).
pub(crate) fn check_url(
    url: &str,
    pacing: Pacing,
    probe: &impl Fn(&str, Method) -> Probe,
    sleep: &impl Fn(Duration),
) -> Outcome {
    let mut attempts = 0;
    loop {
        attempts += 1;
        let mut answer = probe(url, Method::Head);
        let mut class = match answer {
            Probe::Status(status) => classify(Method::Head, status),
            // Some servers hang up on or time out HEAD only: ask with GET before retrying.
            Probe::Network(_) => Class::TryGet,
        };
        if class == Class::TryGet {
            sleep(pacing.per_host);
            answer = probe(url, Method::Get);
            class = match answer {
                Probe::Status(status) => classify(Method::Get, status),
                Probe::Network(_) => Class::Transient,
            };
        }
        match (class, &answer) {
            (Class::Ok, Probe::Status(status)) => {
                return Outcome::Alive {
                    status: *status,
                    attempts,
                };
            }
            (Class::Transient, _) if attempts <= pacing.retries => {
                sleep(pacing.backoff.saturating_mul(attempts));
            }
            _ => return Outcome::Dead { last: answer, attempts },
        }
    }
}

/// The real probe: one `curl` request. `curl` ships with macOS, Windows 10+ and the CI images.
fn curl(url: &str, method: Method) -> Probe {
    let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
    let mut command = Command::new("curl");
    command.args([
        "--silent",
        "--show-error",
        "--location",
        "--max-redirs",
        "10",
        "--connect-timeout",
        "15",
        "--max-time",
        "30",
        // HTTP/2 makes some servers drop the stream (curl exit 92) where HTTP/1.1 works.
        "--http1.1",
        "--user-agent",
        "lamina-link-check/1 (+https://github.com/laudebugs/lamina)",
        // Without an HTML Accept header some sites (crates.io) answer 404 to every page.
        "--header",
        "Accept: text/html,application/xhtml+xml,*/*;q=0.8",
        "--output",
        null,
        "--write-out",
        "%{http_code}",
        // A host that ignores `Range` would send the whole file: stop after the headers.
        "--max-filesize",
        "1",
    ]);
    match method {
        Method::Head => command.arg("--head"),
        Method::Get => command.args(["--range", "0-0"]),
    };
    // `--` so a URL can never be read as an option.
    command.arg("--").arg(url);
    let output = match command.output() {
        Ok(output) => output,
        Err(error) => return Probe::Network(format!("could not run curl: {error}")),
    };
    let code = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u16>()
        .unwrap_or(0);
    // Exit 63: the body was over `--max-filesize`, after a status was received.
    if code != 0 && (output.status.success() || output.status.code() == Some(63)) {
        Probe::Status(code)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = stderr.lines().last().unwrap_or("").trim();
        Probe::Network(if message.is_empty() {
            format!("curl exit {}", output.status.code().unwrap_or(-1))
        } else {
            message.to_owned()
        })
    }
}

/// Checks every URL: hosts in parallel on [`WORKERS`] threads, one host's URLs in turn with
/// `pacing.per_host` between requests. `on_done` sees each result as it lands (progress).
pub(crate) fn check_all(
    urls: &BTreeMap<String, Vec<Site>>,
    allowlist: &[Allow],
    pacing: Pacing,
    probe: &(impl Fn(&str, Method) -> Probe + Sync),
    sleep: &(impl Fn(Duration) + Sync),
    on_done: &(impl Fn(&str, &Outcome) + Sync),
) -> BTreeMap<String, Outcome> {
    let mut results = BTreeMap::new();
    let mut by_host: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for url in urls.keys() {
        if let Some(allow) = allowed(allowlist, url) {
            let outcome = Outcome::Allowed {
                reason: allow.reason.clone(),
            };
            on_done(url, &outcome);
            results.insert(url.clone(), outcome);
        } else {
            by_host.entry(host(url)).or_default().push(url);
        }
    }
    let queue = Mutex::new(by_host.into_values().collect::<VecDeque<_>>());
    let done = Mutex::new(Vec::new());
    thread::scope(|scope| {
        for _ in 0..WORKERS {
            scope.spawn(|| {
                // A poisoned lock means another worker panicked: stop taking hosts.
                while let Some(host_urls) = queue.lock().ok().and_then(|mut queue| queue.pop_front()) {
                    let mut unreachable = 0;
                    for (i, url) in host_urls.into_iter().enumerate() {
                        let outcome = if unreachable >= HOST_GIVE_UP {
                            Outcome::Dead {
                                last: Probe::Network(format!(
                                    "not fetched: the host's previous {HOST_GIVE_UP} URLs got no HTTP answer"
                                )),
                                attempts: 0,
                            }
                        } else {
                            if i > 0 {
                                sleep(pacing.per_host);
                            }
                            check_url(url, pacing, probe, sleep)
                        };
                        if matches!(
                            outcome,
                            Outcome::Dead {
                                last: Probe::Network(_),
                                ..
                            }
                        ) {
                            unreachable += 1;
                        } else {
                            unreachable = 0;
                        }
                        on_done(url, &outcome);
                        if let Ok(mut done) = done.lock() {
                            done.push((url.to_owned(), outcome));
                        }
                    }
                }
            });
        }
    });
    results.extend(done.into_inner().unwrap_or_default());
    results
}

/// Allowlist prefixes that cover no URL any more: candidates for removal.
fn unused<'a>(allowlist: &'a [Allow], urls: &BTreeMap<String, Vec<Site>>) -> Vec<&'a str> {
    allowlist
        .iter()
        .filter(|allow| !urls.keys().any(|url| covers(&allow.prefix, url)))
        .map(|allow| allow.prefix.as_str())
        .collect()
}

/// The Markdown report: dead and flaky URLs with where they are linked from, then counts.
fn markdown_report(urls: &BTreeMap<String, Vec<Site>>, results: &BTreeMap<String, Outcome>) -> String {
    let count = |label: &str| results.values().filter(|o| o.label() == label).count();
    let mut out = String::from("## External links\n\n");
    let _ = writeln!(
        out,
        "{} URLs: {} ok, {} flaky, {} dead, {} allowed ({ALLOWLIST}).\n",
        results.len(),
        count("ok"),
        count("flaky"),
        count("dead"),
        count("allowed"),
    );
    for (title, keep) in [
        ("Dead", Outcome::is_dead as fn(&Outcome) -> bool),
        ("Flaky (passed on a retry)", Outcome::is_flaky),
    ] {
        let rows: Vec<_> = results.iter().filter(|(_, o)| keep(o)).collect();
        if rows.is_empty() {
            continue;
        }
        let _ = writeln!(
            out,
            "### {title}\n\n| URL | Result | Linked from |\n| --- | --- | --- |"
        );
        for (url, outcome) in rows {
            let sites = urls.get(url).map_or_else(String::new, |sites| {
                let mut shown = sites
                    .iter()
                    .take(SITES_SHOWN)
                    .map(|s| format!("`{}:{}`", s.file, s.line))
                    .collect::<Vec<_>>()
                    .join(", ");
                if sites.len() > SITES_SHOWN {
                    let _ = write!(shown, " and {} more", sites.len() - SITES_SHOWN);
                }
                shown
            });
            let _ = writeln!(
                out,
                "| {} | {} | {} |",
                md_cell(url),
                md_cell(&outcome.detail()),
                md_cell(&sites)
            );
        }
        out.push('\n');
    }
    out
}

/// `cargo xtask docs-links --external [--json]`.
pub(crate) fn run(root: &Path, json_output: bool) -> Result<()> {
    let tracked = Tracked::from_git(root)?;
    let files = markdown_files(&tracked);
    let urls = collect(root, &files)?;
    // Without curl every probe would fail as a network error and be retried: fail now instead.
    let curl_runs = Command::new("curl")
        .arg("--version")
        .output()
        .is_ok_and(|out| out.status.success());
    if !curl_runs {
        bail!("docs-links --external needs `curl` on PATH");
    }
    let allowlist = match fs::read_to_string(root.join(ALLOWLIST)) {
        Ok(text) => parse_allowlist(&text)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(error).context(format!("reading {ALLOWLIST}")),
    };
    eprintln!(
        "docs-links --external: {} URLs on {} hosts in {} Markdown files",
        urls.len(),
        urls.keys()
            .map(|url| host(url))
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        files.len()
    );
    let progress = |url: &str, outcome: &Outcome| {
        if !json_output {
            println!("{:<7} {url} ({})", outcome.label(), outcome.detail());
        }
    };
    let results = check_all(&urls, &allowlist, Pacing::NIGHTLY, &curl, &thread::sleep, &progress);

    let report_dir = root.join(REPORT_DIR);
    fs::create_dir_all(&report_dir).with_context(|| format!("creating {REPORT_DIR}"))?;
    let items: Vec<_> = results
        .iter()
        .map(|(url, outcome)| {
            json!({
                "url": url,
                "result": outcome.label(),
                "detail": outcome.detail(),
                "linked_from": urls.get(url).map(|sites| sites
                    .iter()
                    .map(|s| format!("{}:{}", s.file, s.line))
                    .collect::<Vec<_>>()),
            })
        })
        .collect();
    let report = serde_json::to_string_pretty(&items)?;
    fs::write(report_dir.join("report.json"), &report)?;
    let markdown = markdown_report(&urls, &results);
    fs::write(report_dir.join("report.md"), &markdown)?;
    summary(&markdown)?;
    if json_output {
        println!("{report}");
    }

    for prefix in unused(&allowlist, &urls) {
        eprintln!("warning: {ALLOWLIST}: `{prefix}` covers no linked URL; remove it");
    }
    let dead: Vec<_> = results.iter().filter(|(_, o)| o.is_dead()).collect();
    for (url, outcome) in &dead {
        let sites = urls.get(*url).map(Vec::as_slice).unwrap_or_default();
        for site in sites {
            eprintln!("{}:{}: dead link {url} ({})", site.file, site.line, outcome.detail());
        }
    }
    let flaky = results.values().filter(|o| o.is_flaky()).count();
    github_output("flaky", if flaky > 0 { "true" } else { "false" })?;
    if !dead.is_empty() {
        bail!(
            "{} dead external link(s) of {} (report in {REPORT_DIR}/report.md)",
            dead.len(),
            results.len()
        );
    }
    eprintln!(
        "docs-links --external: {} URLs, none dead ({flaky} flaky)",
        results.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;
    use std::time::Duration;

    use super::{
        Allow, Method, Outcome, Pacing, Probe, Site, allowed, bare_urls, check_all, check_url, host, is_http,
        markdown_report, parse_allowlist, scan, unused, without_fragment,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    const FAST: Pacing = Pacing {
        per_host: Duration::ZERO,
        retries: 2,
        backoff: Duration::ZERO,
    };

    fn external(text: &str) -> Vec<String> {
        scan(text).external.into_iter().map(|l| l.target).collect()
    }

    /// The methods a scripted probe was asked for, in order.
    type Asked = Rc<RefCell<Vec<Method>>>;

    /// A probe that answers from a script, in order, and records what it was asked.
    fn scripted(answers: Vec<Probe>) -> (impl Fn(&str, Method) -> Probe, Asked) {
        let asked = Asked::default();
        let queue = RefCell::new(answers.into_iter());
        let log = Rc::clone(&asked);
        let probe = move |_: &str, method: Method| {
            log.borrow_mut().push(method);
            queue
                .borrow_mut()
                .next()
                .unwrap_or_else(|| Probe::Network("script ran out".into()))
        };
        (probe, asked)
    }

    fn no_sleep(_: Duration) {}

    #[test]
    fn external_should_include_inline_reference_image_and_autolinks() {
        let text = "[a](https://a.example/x) ![i](http://i.example/i.png) <https://auto.example>\n\n\
                    [r][ref]\n\n[ref]: https://ref.example/\n";
        assert_eq!(
            external(text),
            [
                "https://a.example/x",
                "http://i.example/i.png",
                "https://auto.example",
                "https://ref.example/"
            ]
        );
    }

    #[test]
    fn external_should_skip_relative_mailto_and_code() {
        let text = "[r](docs/x.md) [m](mailto:me@example.com) <me@example.com> `https://code.example`\n\n\
                    ```\nhttps://block.example\n```\n";
        assert_eq!(external(text), Vec::<String>::new());
    }

    #[test]
    fn external_should_include_bare_urls_in_text_and_tables() {
        let text = "See https://pdfa.org/standards/, then (https://w.example/a_(b)).\n\n\
                    | Topic | Link |\n| --- | --- |\n| PDF | https://pypdf.example/x_y?q=1 |\n";
        assert_eq!(
            external(text),
            [
                "https://pdfa.org/standards/",
                "https://w.example/a_(b)",
                "https://pypdf.example/x_y?q=1"
            ]
        );
    }

    #[test]
    fn external_should_count_a_url_used_as_link_text_once() {
        assert_eq!(
            external("[https://a.example](https://a.example)\n"),
            ["https://a.example"]
        );
    }

    #[test]
    fn external_should_include_raw_html_href_and_src() {
        let text = "<p><img src=\"https://img.example/i.png\"></p>\n\nx <a href='https://a.example/'>a</a>\n";
        assert_eq!(external(text), ["https://img.example/i.png", "https://a.example/"]);
    }

    #[test]
    fn bare_urls_should_skip_mid_word_schemes_and_empty_hosts() {
        assert_eq!(
            bare_urls("xhttps://a.example https:// ok “https://” https://…").count(),
            0
        );
    }

    #[test]
    fn autolinks_should_stay_out_of_the_relative_link_list() {
        assert_eq!(scan("<https://auto.example>\n").links, []);
    }

    #[test]
    fn is_http_should_ignore_scheme_case() {
        assert!(is_http("HTTPS://Example.com") && is_http("http://x") && !is_http("ftp://x"));
    }

    #[test]
    fn without_fragment_should_drop_the_anchor() {
        assert_eq!(without_fragment("https://x.example/a#b"), "https://x.example/a");
    }

    #[test]
    fn host_should_be_lowercased_without_path_query_or_credentials() {
        assert_eq!(
            [
                host("https://GitHub.com/a/b"),
                host("https://u:p@x.example:8443?q"),
                host("http://y.example")
            ],
            ["github.com", "x.example:8443", "y.example"]
        );
    }

    #[test]
    fn parse_allowlist_should_read_prefixes_and_reasons() {
        let text = "[[allow]]\nprefix = \"https://claude.ai/\"\nreason = \"login wall\"\n";
        assert_eq!(
            parse_allowlist(text).ok(),
            Some(vec![Allow {
                prefix: "https://claude.ai/".into(),
                reason: "login wall".into()
            }])
        );
    }

    #[test]
    fn parse_allowlist_should_accept_an_empty_file() {
        assert_eq!(parse_allowlist("# nothing yet\n").ok(), Some(Vec::new()));
    }

    #[test]
    fn parse_allowlist_should_require_a_reason() {
        assert!(parse_allowlist("[[allow]]\nprefix = \"https://x.example/\"\n").is_err());
    }

    #[test]
    fn parse_allowlist_should_reject_a_non_http_prefix() {
        assert!(parse_allowlist("[[allow]]\nprefix = \"x.example\"\nreason = \"r\"\n").is_err());
    }

    #[test]
    fn parse_allowlist_should_reject_unknown_keys() {
        assert!(parse_allowlist("[[allowed]]\nprefix = \"https://x/\"\nreason = \"r\"\n").is_err());
    }

    #[test]
    fn allowed_should_ignore_case_and_a_trailing_slash() {
        let list = [Allow {
            prefix: "https://github.com/laudebugs/lamina/".into(),
            reason: "private".into(),
        }];
        assert_eq!(
            [
                allowed(&list, "https://GitHub.com/laudebugs/lamina/pull/1").is_some(),
                allowed(&list, "https://github.com/laudebugs/lamina").is_some(),
                allowed(&list, "https://github.com/laudebugs/laminax").is_some()
            ],
            [true, true, false]
        );
    }

    #[test]
    fn allowed_should_match_by_prefix() {
        let list = [Allow {
            prefix: "https://claude.ai/".into(),
            reason: "r".into(),
        }];
        assert_eq!(
            [
                allowed(&list, "https://claude.ai/share/1").is_some(),
                allowed(&list, "https://claude.aix/").is_some()
            ],
            [true, false]
        );
    }

    #[test]
    fn check_url_should_pass_a_200_in_one_request() {
        let (probe, asked) = scripted(vec![Probe::Status(200)]);
        let outcome = check_url("u", FAST, &probe, &no_sleep);
        assert_eq!(
            (outcome, asked.borrow().len()),
            (
                Outcome::Alive {
                    status: 200,
                    attempts: 1
                },
                1
            )
        );
    }

    #[test]
    fn check_url_should_fail_a_404_without_retrying() {
        let (probe, asked) = scripted(vec![Probe::Status(404), Probe::Status(404)]);
        let outcome = check_url("u", FAST, &probe, &no_sleep);
        assert_eq!(
            (outcome, asked.borrow().clone()),
            (
                Outcome::Dead {
                    last: Probe::Status(404),
                    attempts: 1
                },
                vec![Method::Head, Method::Get]
            )
        );
    }

    #[test]
    fn check_url_should_fall_back_to_get_when_head_is_refused() {
        let (probe, _) = scripted(vec![Probe::Status(405), Probe::Status(206)]);
        assert_eq!(
            check_url("u", FAST, &probe, &no_sleep),
            Outcome::Alive {
                status: 206,
                attempts: 1
            }
        );
    }

    #[test]
    fn check_url_should_report_a_pass_after_a_timeout_as_flaky() {
        let timeout = || Probe::Network("timeout".into());
        let (probe, _) = scripted(vec![timeout(), timeout(), Probe::Status(200)]);
        let outcome = check_url("u", FAST, &probe, &no_sleep);
        assert!(outcome.is_flaky());
    }

    #[test]
    fn check_url_should_try_get_when_head_gets_no_answer() {
        let (probe, asked) = scripted(vec![Probe::Network("reset".into()), Probe::Status(200)]);
        let outcome = check_url("u", FAST, &probe, &no_sleep);
        assert_eq!(
            (outcome.label(), asked.borrow().clone()),
            ("ok", vec![Method::Head, Method::Get])
        );
    }

    #[test]
    fn check_url_should_give_up_after_the_retries() {
        let (probe, asked) = scripted(vec![Probe::Status(503); 5]);
        let outcome = check_url("u", FAST, &probe, &no_sleep);
        assert_eq!(
            (outcome, asked.borrow().len()),
            (
                Outcome::Dead {
                    last: Probe::Status(503),
                    attempts: 3
                },
                3
            )
        );
    }

    #[test]
    fn check_url_should_back_off_longer_on_each_retry() {
        let slept = RefCell::new(Vec::new());
        let (probe, _) = scripted(vec![Probe::Status(429); 3]);
        let pacing = Pacing {
            backoff: Duration::from_secs(5),
            ..FAST
        };
        check_url("u", pacing, &probe, &|d| slept.borrow_mut().push(d));
        assert_eq!(*slept.borrow(), [Duration::from_secs(5), Duration::from_secs(10)]);
    }

    fn urls(list: &[&str]) -> BTreeMap<String, Vec<Site>> {
        list.iter()
            .map(|url| {
                (
                    (*url).to_owned(),
                    vec![Site {
                        file: "README.md".into(),
                        line: 1,
                    }],
                )
            })
            .collect()
    }

    #[test]
    fn check_all_should_skip_allowlisted_urls_and_check_the_rest() {
        let urls = urls(&["https://a.example/", "https://b.example/dead", "https://claude.ai/x"]);
        let list = [Allow {
            prefix: "https://claude.ai/".into(),
            reason: "login wall".into(),
        }];
        let probe = |url: &str, _: Method| Probe::Status(if url.ends_with("dead") { 410 } else { 200 });
        let results = check_all(&urls, &list, FAST, &probe, &no_sleep, &|_: &str, _: &Outcome| {});
        let labels: Vec<_> = results.values().map(Outcome::label).collect();
        assert_eq!(labels, ["ok", "dead", "allowed"]);
    }

    #[test]
    fn check_all_should_stop_fetching_a_host_after_repeated_network_failures() {
        let urls = urls(&[
            "https://s.example/1",
            "https://s.example/2",
            "https://s.example/3",
            "https://s.example/4",
            "https://s.example/5",
        ]);
        let asked = AtomicUsize::new(0);
        let probe = |_: &str, _: Method| {
            asked.fetch_add(1, Ordering::Relaxed);
            Probe::Network("timeout".into())
        };
        let results = check_all(&urls, &[], FAST, &probe, &no_sleep, &|_: &str, _: &Outcome| {});
        let probes_per_url = 2 * (FAST.retries as usize + 1);
        assert_eq!(
            (
                results.values().filter(|o| o.is_dead()).count(),
                asked.load(Ordering::Relaxed)
            ),
            (5, super::HOST_GIVE_UP * probes_per_url)
        );
    }

    #[test]
    fn unused_should_list_prefixes_that_match_nothing() {
        let list = [
            Allow {
                prefix: "https://a.example/".into(),
                reason: "r".into(),
            },
            Allow {
                prefix: "https://gone.example/".into(),
                reason: "r".into(),
            },
        ];
        assert_eq!(
            unused(&list, &urls(&["https://a.example/x"])),
            ["https://gone.example/"]
        );
    }

    #[test]
    fn markdown_report_should_list_dead_links_with_their_sites() {
        let urls = urls(&["https://b.example/dead"]);
        let results = BTreeMap::from([(
            "https://b.example/dead".to_owned(),
            Outcome::Dead {
                last: Probe::Status(404),
                attempts: 1,
            },
        )]);
        assert!(
            markdown_report(&urls, &results)
                .contains("| https://b.example/dead | 404 after 1 attempt(s) | `README.md:1` |")
        );
    }
}
