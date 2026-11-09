//! The external link check's memory between nights (`cargo xtask docs-links --external --cache
//! <file>`): the day each URL last answered OK on its first try. A URL checked within its
//! recheck age (4 to 7 days, spread by the URL so they don't all come due on one night) is not
//! fetched again and reports `cached`. A URL that is new, failed, was refused, passed only on a
//! retry, or is due is fetched as before, so a page that breaks is found within a week. The
//! nightly keeps the file in the Actions cache (`.github/workflows/nightly.yml`).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::external::Outcome;

/// The fewest days before a URL that answered OK is fetched again.
const MIN_AGE_DAYS: u64 = 4;
/// How many more days a URL may wait, by its hash: 0 to 3, so the recheck age is 4 to 7 days.
const AGE_SPREAD_DAYS: u64 = 4;

const USAGE: &str = "usage: cargo xtask docs-links --external [--json] [--cache <file>]";

/// What `cargo xtask docs-links --external` was asked to do.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ExternalArgs {
    pub(crate) json: bool,
    pub(crate) cache: Option<PathBuf>,
}

/// Reads the arguments after `docs-links` when they include `--external`.
pub(crate) fn parse_args(args: &[String]) -> Result<ExternalArgs> {
    let mut parsed = ExternalArgs::default();
    let mut external = false;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--external" if !external => external = true,
            "--json" if !parsed.json => parsed.json = true,
            "--cache" if parsed.cache.is_none() => parsed.cache = Some(rest.next().context(USAGE)?.into()),
            other => bail!("unexpected argument {other:?}; {USAGE}"),
        }
    }
    if !external {
        bail!("{USAGE}");
    }
    Ok(parsed)
}

/// Checks the URLs `cache_path`'s cache doesn't vouch for with `check`, and gives the cached
/// ones `Outcome::Cached` (each also passed to `progress`); then writes the cache back. Without a
/// cache file, every URL is checked. `allowlisted` URLs are always handed to `check`, which
/// reports them as allowed.
pub(crate) fn check_due<V: Clone>(
    urls: &BTreeMap<String, V>,
    cache_path: Option<&Path>,
    allowlisted: impl Fn(&str) -> bool,
    check: impl FnOnce(&BTreeMap<String, V>) -> BTreeMap<String, Outcome>,
    progress: impl Fn(&str, &Outcome),
) -> Result<BTreeMap<String, Outcome>> {
    let Some(path) = cache_path else {
        return Ok(check(urls));
    };
    let today = today();
    let cache = LinkCache::load(path);
    let mut cached = BTreeMap::new();
    let mut due = BTreeMap::new();
    for (url, value) in urls {
        match cache.recent(url, today).filter(|_| !allowlisted(url)) {
            Some(outcome) => {
                cached.insert(url.clone(), outcome);
            }
            None => {
                due.insert(url.clone(), value.clone());
            }
        }
    }
    eprintln!(
        "docs-links --external: {} URLs answered OK in the last few days and are not fetched again",
        cached.len()
    );
    let mut results = check(&due);
    for (url, outcome) in cached {
        progress(&url, &outcome);
        results.insert(url, outcome);
    }
    cache.after(&results, today).save(path)?;
    Ok(results)
}

/// Today, in days since the Unix epoch (UTC).
fn today() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() / 86_400)
}

/// The cache file: for each URL, the day it last answered OK on its first try.
#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LinkCache {
    version: u32,
    ok_on: BTreeMap<String, u64>,
}

impl LinkCache {
    const VERSION: u32 = 1;

    /// The cache at `path`; empty when the file is missing, unreadable or from another version,
    /// since a lost cache only means fetching every URL once.
    fn load(path: &Path) -> Self {
        let Ok(text) = fs::read_to_string(path) else {
            return Self::default();
        };
        match serde_json::from_str::<Self>(&text) {
            Ok(cache) if cache.version == Self::VERSION => cache,
            _ => {
                eprintln!(
                    "docs-links --external: ignoring {}, which isn't a cache this version reads",
                    path.display()
                );
                Self::default()
            }
        }
    }

    /// Writes the cache to `path`.
    fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        fs::write(path, serde_json::to_string_pretty(self)? + "\n")
            .with_context(|| format!("writing {}", path.display()))
    }

    /// `Outcome::Cached` for `url` when it answered OK within its recheck age of `today`.
    fn recent(&self, url: &str, today: u64) -> Option<Outcome> {
        let day = *self.ok_on.get(url)?;
        let age_days = today.checked_sub(day)?;
        (age_days < recheck_age(url)).then_some(Outcome::Cached { age_days })
    }

    /// The cache after a run: `results` for the URLs still linked. A URL that answered OK on its
    /// first try is stamped `today`, a cached one keeps its day, and anything else is dropped so
    /// the next run fetches it.
    fn after(&self, results: &BTreeMap<String, Outcome>, today: u64) -> Self {
        let ok_on = results
            .iter()
            .filter_map(|(url, outcome)| match outcome {
                Outcome::Alive { attempts: 1, .. } => Some((url.clone(), today)),
                Outcome::Cached { .. } => self.ok_on.get(url).map(|day| (url.clone(), *day)),
                _ => None,
            })
            .collect();
        Self {
            version: Self::VERSION,
            ok_on,
        }
    }
}

/// How many days `url` stays cached: [`MIN_AGE_DAYS`] plus 0 to 3 by a hash of the URL (FNV-1a,
/// so it is the same on every machine and every night).
fn recheck_age(url: &str) -> u64 {
    let hash = url.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    });
    MIN_AGE_DAYS + hash % AGE_SPREAD_DAYS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docs_links::external::Probe;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn parse_args_should_read_the_cache_file_with_or_without_json() -> Result<()> {
        assert_eq!(parse_args(&args(&["--external"]))?, ExternalArgs::default());
        let both = parse_args(&args(&["--json", "--external", "--cache", "c.json"]))?;
        assert_eq!(
            both,
            ExternalArgs {
                json: true,
                cache: Some("c.json".into())
            }
        );
        Ok(())
    }

    #[test]
    fn parse_args_should_refuse_a_missing_file_a_repeat_or_no_external() {
        for bad in [
            &["--external", "--cache"][..],
            &["--external", "--json", "--json"],
            &["--json"],
        ] {
            assert!(parse_args(&args(bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn recheck_age_should_be_four_to_seven_days_and_stable() {
        for url in ["https://a.example/", "https://b.example/x", "https://github.com/x/y"] {
            let age = recheck_age(url);
            assert!(
                (MIN_AGE_DAYS..MIN_AGE_DAYS + AGE_SPREAD_DAYS).contains(&age),
                "{url}: {age}"
            );
            assert_eq!(age, recheck_age(url));
        }
    }

    #[test]
    fn a_url_should_be_cached_until_its_recheck_age() {
        let url = "https://a.example/";
        let cache = LinkCache {
            version: LinkCache::VERSION,
            ok_on: BTreeMap::from([(url.to_owned(), 100)]),
        };
        assert_eq!(cache.recent(url, 100), Some(Outcome::Cached { age_days: 0 }));
        assert_eq!(
            cache.recent(url, 100 + recheck_age(url) - 1),
            Some(Outcome::Cached {
                age_days: recheck_age(url) - 1
            })
        );
        assert_eq!(cache.recent(url, 100 + recheck_age(url)), None);
        assert_eq!(cache.recent("https://new.example/", 100), None);
        // A day from the future (a clock moved back) is not trusted.
        assert_eq!(cache.recent(url, 99), None);
    }

    #[test]
    fn after_should_keep_only_first_try_successes_and_cached_days() {
        let before = LinkCache {
            version: LinkCache::VERSION,
            ok_on: BTreeMap::from([("https://cached/".to_owned(), 90), ("https://gone/".to_owned(), 90)]),
        };
        let results = BTreeMap::from([
            (
                "https://ok/".to_owned(),
                Outcome::Alive {
                    status: 200,
                    attempts: 1,
                },
            ),
            (
                "https://flaky/".to_owned(),
                Outcome::Alive {
                    status: 200,
                    attempts: 2,
                },
            ),
            ("https://cached/".to_owned(), Outcome::Cached { age_days: 5 }),
            (
                "https://refused/".to_owned(),
                Outcome::Refused {
                    status: 403,
                    attempts: 1,
                },
            ),
            (
                "https://missing/".to_owned(),
                Outcome::Missing {
                    last: Probe::Status(404),
                    attempts: 1,
                },
            ),
        ]);
        let after = before.after(&results, 95);
        assert_eq!(
            after.ok_on,
            BTreeMap::from([("https://cached/".to_owned(), 90), ("https://ok/".to_owned(), 95)])
        );
    }

    #[test]
    fn load_should_start_empty_without_a_file_and_read_back_what_save_wrote() -> Result<()> {
        let dir = std::env::temp_dir().join(format!("xtask-link-cache-{}", std::process::id()));
        let path = dir.join("links.json");
        assert_eq!(LinkCache::load(&path), LinkCache::default());
        let cache = LinkCache {
            version: LinkCache::VERSION,
            ok_on: BTreeMap::from([("https://a/".to_owned(), 7)]),
        };
        cache.save(&path)?;
        assert_eq!(LinkCache::load(&path), cache);
        fs::write(&path, "{\"version\": 99, \"ok_on\": {}}")?;
        assert_eq!(LinkCache::load(&path), LinkCache::default());
        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn check_due_should_fetch_only_what_the_cache_does_not_vouch_for() -> Result<()> {
        let dir = std::env::temp_dir().join(format!("xtask-link-cache-due-{}", std::process::id()));
        let path = dir.join("links.json");
        let urls: BTreeMap<String, u8> = ["https://ok/", "https://dead/", "https://allowed/"]
            .into_iter()
            .map(|u| (u.to_owned(), 0))
            .collect();
        let check = |due: &BTreeMap<String, u8>| {
            due.keys()
                .map(|url| {
                    let outcome = match url.as_str() {
                        "https://dead/" => Outcome::Missing {
                            last: Probe::Status(404),
                            attempts: 1,
                        },
                        "https://allowed/" => Outcome::Allowed { reason: "r".into() },
                        _ => Outcome::Alive {
                            status: 200,
                            attempts: 1,
                        },
                    };
                    (url.clone(), outcome)
                })
                .collect()
        };
        let allowlisted = |url: &str| url == "https://allowed/";
        let first = check_due(&urls, Some(&path), allowlisted, check, |_, _| {})?;
        assert_eq!(first.len(), 3);
        let fetched = std::cell::RefCell::new(Vec::new());
        let second = check_due(
            &urls,
            Some(&path),
            allowlisted,
            |due| {
                fetched.borrow_mut().extend(due.keys().cloned());
                check(due)
            },
            |_, _| {},
        )?;
        assert_eq!(*fetched.borrow(), ["https://allowed/", "https://dead/"]);
        assert_eq!(second.get("https://ok/"), Some(&Outcome::Cached { age_days: 0 }));
        fs::remove_dir_all(&dir)?;
        Ok(())
    }
}
