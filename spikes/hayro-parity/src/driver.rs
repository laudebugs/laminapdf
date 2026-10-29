//! The driver: runs one child process per document (in parallel), enforces the
//! per-page timeout, turns crashes into rows, and writes `results.csv`.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};

use crate::compare;

pub struct Job {
    pub corpus: String,
    pub path: PathBuf,
    /// Path relative to the corpus root (stable id in the CSV).
    pub name: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Engines {
    Both,
    Hayro,
    Pdfium,
}

pub struct Options {
    pub dpi: f32,
    pub jobs: usize,
    pub timeout: Duration,
    pub engines: Engines,
    pub pdfium_lib: PathBuf,
    pub out: PathBuf,
}

/// One page (or, with `page == 0`, a document that no engine could open).
#[derive(Clone, Default)]
pub struct Row {
    pub corpus: String,
    pub file: String,
    pub page: usize,
    pub class: String,
    pub ssim: Option<f64>,
    pub diff_pct: Option<f64>,
    pub hayro_ms: Option<f64>,
    pub pdfium_ms: Option<f64>,
    pub hayro_size: String,
    pub pdfium_size: String,
    pub hayro_err: String,
    pub pdfium_err: String,
    pub encrypted: bool,
    pub warnings: String,
    /// The child's default-threshold class for a compared page (computed on
    /// unrounded values), used as-is when neither engine erred.
    pub compared_class: String,
    /// A problem in the harness itself (e.g. the triptych could not be
    /// written); never changes the page's class.
    pub harness_err: String,
}

/// Peak RSS of one child process (one document).
pub struct Proc {
    pub corpus: String,
    pub file: String,
    pub max_rss: u64,
    /// The child reported `done` and then exited abnormally (a crash while
    /// tearing down); every page has a row, so this is not a page failure.
    pub teardown: Option<String>,
}

impl Row {
    pub fn classify(&mut self, timed_out: bool) {
        let h_err = !self.hayro_err.is_empty();
        let p_err = !self.pdfium_err.is_empty();
        self.class = if timed_out {
            "timeout"
        } else if h_err && p_err {
            "both-error"
        } else if h_err {
            "hayro-error"
        } else if p_err {
            "pdfium-error"
        } else if !self.compared_class.is_empty() {
            self.compared_class.as_str()
        } else if let (Some(s), Some(d)) = (self.ssim, self.diff_pct) {
            compare::DEFAULT.classify(s, d)
        } else if !self.hayro_size.is_empty() && !self.pdfium_size.is_empty() {
            // Both rendered but to different sizes.
            "fail"
        } else {
            // Single-engine run: no comparison.
            "rendered"
        }
        .to_owned();
    }
}

fn opt(s: &str) -> Option<f64> {
    s.parse().ok()
}

/// Wait for `pid` and return (exit description or None if it exited 0, peak RSS bytes).
fn reap(pid: i32) -> (Option<String>, u64) {
    let mut status = 0;
    // SAFETY: `rusage` is plain data, so all-zero is a valid value.
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    // SAFETY: valid pointers to locals; `pid` is our own unreaped child.
    let r = unsafe { libc::wait4(pid, &mut status, 0, &mut usage) };
    // ru_maxrss is bytes on macOS and KiB on Linux.
    let rss = usage.ru_maxrss as u64 * if cfg!(target_os = "macos") { 1 } else { 1024 };
    if r < 0 {
        return (Some("wait4 failed".into()), rss);
    }
    let exit = if libc::WIFSIGNALED(status) {
        Some(format!("crash (signal {})", libc::WTERMSIG(status)))
    } else if libc::WIFEXITED(status) && libc::WEXITSTATUS(status) != 0 {
        Some(format!("exit {}", libc::WEXITSTATUS(status)))
    } else {
        None
    };
    (exit, rss)
}

/// Renders one document, restarting the child after a crash or a hang.
fn run_doc(job: &Job, opts: &Options) -> Result<(Vec<Row>, Proc)> {
    // `encrypted` is filled in from the child's `pages` line (the driver never
    // reads the document itself).
    let mut base = Row {
        corpus: job.corpus.clone(),
        file: job.name.clone(),
        ..Row::default()
    };
    let mut teardown = None;
    let mut rows = Vec::new();
    let mut max_rss = 0;
    let mut start = 1;
    let (mut hayro_on, mut pdfium_on) = (opts.engines != Engines::Pdfium, opts.engines != Engines::Hayro);
    // Errors for an engine disabled after it crashed or hung while loading.
    let (mut hayro_off_err, mut pdfium_off_err) = (String::new(), String::new());
    let fail_dir = opts
        .out
        .join("fail")
        .join(&job.corpus)
        .join(job.name.replace('/', "__"));
    let mut pages: Option<usize> = None;

    loop {
        let mut cmd = Command::new(std::env::current_exe()?);
        cmd.arg("--one")
            .arg(&job.path)
            .args(["--dpi", &opts.dpi.to_string(), "--start", &start.to_string()])
            .arg("--pdfium-lib")
            .arg(&opts.pdfium_lib)
            .arg("--fail-dir")
            .arg(&fail_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        if !hayro_on {
            cmd.arg("--no-hayro");
        }
        if !pdfium_on {
            cmd.arg("--no-pdfium");
        }
        let mut child = cmd.spawn().context("spawn child")?;
        let pid = child.id() as i32;
        let stdout = child.stdout.take().context("child stdout")?;
        let (tx, rx) = channel();
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });

        let mut current: Option<(usize, String)> = None;
        let mut finished = false;
        let mut timed_out = false;
        loop {
            match rx.recv_timeout(opts.timeout) {
                Ok(line) => {
                    let f: Vec<&str> = line.split('\t').collect();
                    match f.first().copied() {
                        Some("pages") if f.len() >= 6 => {
                            base.encrypted = f[5] == "true";
                            let n = |s: &str| s.parse::<i64>().unwrap_or(-1).max(0) as usize;
                            pages = Some(n(f[1]).max(n(f[2])));
                            if pages == Some(0) {
                                let mut r = base.clone();
                                r.hayro_err = non_empty(f[3], &hayro_off_err, "no pages");
                                r.pdfium_err = non_empty(f[4], &pdfium_off_err, "no pages");
                                if !hayro_on && opts.engines == Engines::Pdfium {
                                    r.hayro_err.clear();
                                }
                                if !pdfium_on && opts.engines == Engines::Hayro {
                                    r.pdfium_err.clear();
                                }
                                r.classify(false);
                                rows.push(r);
                            }
                        }
                        Some("begin") if f.len() >= 3 => current = Some((f[1].parse().unwrap_or(0), f[2].to_owned())),
                        Some("page") if f.len() >= 13 => {
                            let mut r = base.clone();
                            r.page = f[1].parse().unwrap_or(0);
                            r.hayro_ms = opt(f[2]);
                            r.pdfium_ms = opt(f[3]);
                            r.hayro_size = f[4].into();
                            r.pdfium_size = f[5].into();
                            r.hayro_err = non_empty(f[6], &hayro_off_err, "");
                            r.pdfium_err = non_empty(f[7], &pdfium_off_err, "");
                            r.ssim = opt(f[8]);
                            r.diff_pct = opt(f[9]);
                            r.compared_class = f[10].into();
                            r.warnings = f[11].into();
                            r.harness_err = f[12].into();
                            r.classify(false);
                            rows.push(r);
                            current = None;
                        }
                        Some("done") => finished = true,
                        _ => {}
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    timed_out = true;
                    let _ = child.kill();
                    break;
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        let (exit, rss) = reap(pid);
        let _ = reader.join();
        max_rss = max_rss.max(rss);
        if finished {
            // Every page was reported; an abnormal exit after `done` is a
            // teardown problem, recorded per document rather than as a page.
            teardown = exit;
            break;
        }
        let why = if timed_out {
            format!("timeout ({} s)", opts.timeout.as_secs())
        } else {
            exit.unwrap_or_else(|| "child exited early".into())
        };
        match current {
            // Crashed or hung while loading: disable that engine and retry.
            Some((0, engine)) => {
                if engine == "hayro" {
                    hayro_on = false;
                    hayro_off_err = format!("load: {why}");
                } else {
                    pdfium_on = false;
                    pdfium_off_err = format!("load: {why}");
                }
                if !hayro_on && !pdfium_on {
                    let mut r = base.clone();
                    r.hayro_err = hayro_off_err.clone();
                    r.pdfium_err = pdfium_off_err.clone();
                    r.classify(timed_out);
                    rows.push(r);
                    break;
                }
            }
            // Crashed or hung on a page: record it and continue after it.
            Some((page, engine)) => {
                let mut r = base.clone();
                r.page = page;
                // Keep the other engine's load failure, if it had one.
                r.hayro_err = hayro_off_err.clone();
                r.pdfium_err = pdfium_off_err.clone();
                if engine == "hayro" {
                    r.hayro_err = why;
                } else {
                    r.pdfium_err = why;
                }
                r.classify(timed_out);
                rows.push(r);
                start = page + 1;
                if pages.is_none_or(|n| start > n) {
                    break;
                }
            }
            None => {
                let mut r = base.clone();
                r.hayro_err = non_empty(&hayro_off_err, "", &why);
                r.pdfium_err = non_empty(&pdfium_off_err, "", &why);
                r.classify(timed_out);
                rows.push(r);
                break;
            }
        }
    }
    let proc = Proc {
        corpus: job.corpus.clone(),
        file: job.name.clone(),
        max_rss,
        teardown,
    };
    Ok((rows, proc))
}

fn non_empty(primary: &str, fallback: &str, default: &str) -> String {
    if !primary.is_empty() {
        primary.into()
    } else if !fallback.is_empty() {
        fallback.into()
    } else {
        default.into()
    }
}

/// Runs every job on `opts.jobs` worker threads; prints progress to stderr.
pub fn run_all(jobs: Vec<Job>, opts: &Options) -> Result<(Vec<Row>, Vec<Proc>)> {
    let total = jobs.len();
    let queue = Arc::new(Mutex::new(jobs.into_iter().collect::<VecDeque<_>>()));
    let results = Arc::new(Mutex::new((Vec::new(), Vec::new(), 0usize)));
    std::thread::scope(|s| {
        for _ in 0..opts.jobs.max(1) {
            let (queue, results) = (Arc::clone(&queue), Arc::clone(&results));
            s.spawn(move || {
                while let Some(job) = queue.lock().ok().and_then(|mut q| q.pop_front()) {
                    let outcome = run_doc(&job, opts);
                    let Ok(mut res) = results.lock() else { break };
                    res.2 += 1;
                    match outcome {
                        Ok((rows, proc)) => {
                            res.0.extend(rows);
                            res.1.push(proc);
                        }
                        Err(e) => eprintln!("driver error on {}: {e:#}", job.name),
                    }
                    if res.2 % 50 == 0 || res.2 == total {
                        eprintln!("[{}/{}] documents", res.2, total);
                    }
                }
            });
        }
    });
    let (mut rows, procs, _) = Arc::try_unwrap(results)
        .ok()
        .and_then(|m| m.into_inner().ok())
        .context("collect results")?;
    rows.sort_by(|a, b| (&a.corpus, &a.file, a.page).cmp(&(&b.corpus, &b.file, b.page)));
    Ok((rows, procs))
}

pub fn write_csv(path: &Path, rows: &[Row]) -> Result<()> {
    use std::io::Write;
    let mut w = std::io::BufWriter::new(std::fs::File::create(path)?);
    writeln!(
        w,
        "corpus,file,page,class,ssim,diff_pct,hayro_ms,pdfium_ms,hayro_size,pdfium_size,encrypted,hayro_error,pdfium_error,hayro_warnings,harness_error"
    )?;
    let f = |v: Option<f64>| v.map(|v| format!("{v}")).unwrap_or_default();
    let q = quote;
    for r in rows {
        writeln!(
            w,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            r.corpus,
            q(&r.file),
            r.page,
            r.class,
            f(r.ssim),
            f(r.diff_pct),
            f(r.hayro_ms),
            f(r.pdfium_ms),
            r.hayro_size,
            r.pdfium_size,
            r.encrypted,
            q(&r.hayro_err),
            q(&r.pdfium_err),
            q(&r.warnings),
            q(&r.harness_err),
        )?;
    }
    Ok(())
}

/// A CSV field in double quotes, with embedded quotes doubled (RFC 4180).
fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

pub fn write_rss_csv(path: &Path, procs: &[Proc]) -> Result<()> {
    use std::io::Write;
    let mut w = std::io::BufWriter::new(std::fs::File::create(path)?);
    writeln!(w, "corpus,file,max_rss_bytes,teardown_error")?;
    for p in procs {
        writeln!(
            w,
            "{},{},{},{}",
            p.corpus,
            quote(&p.file),
            p.max_rss,
            quote(p.teardown.as_deref().unwrap_or(""))
        )?;
    }
    Ok(())
}
