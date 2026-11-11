//! An in-process fake engine for tests (feature `test-support`): two pipes and a thread that
//! speaks the protocol, and that can crash, hang or stay silent on cue.
//!
//! A document whose length is even is encrypted, with the password `secret`; every document has
//! 3 pages of 612 × 792 pt. Tiles answer with the size asked for and no pixels.

use std::io::{self, PipeReader, PipeWriter};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::{Process, Spawned, Spawner, lock};
use crate::{
    DocId, FailureKind, FromEngine, MAX_FRAME, PROTOCOL_VERSION, PageDimensions, SandboxLevel, TileFormat, ToEngine,
    read_frame, write_frame,
};

/// How a fake engine behaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Answers requests and beats.
    Healthy,
    /// Exits shortly after `Ready` (an engine that dies on its first document).
    CrashAfterReady,
    /// Answers `Hello`, then never beats or answers again.
    Silent,
    /// Exits as it says `Ready`.
    DieAtOnce,
    /// The first engine is healthy; later ones refuse every `OpenDocument` as invalid.
    RefuseReopen,
    /// The first engine is healthy; later ones exit when asked to open `DocId(1)`.
    DieReopeningFirst,
}

/// The password of the fake's encrypted documents.
pub const PASSWORD: &str = "secret";

/// Starts fake engines in one [`Mode`] and counts them.
#[derive(Debug, Clone)]
pub struct FakeSpawner {
    mode: Mode,
    spawns: Arc<AtomicU32>,
}

impl FakeSpawner {
    /// Fakes in `mode`.
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            spawns: Arc::new(AtomicU32::new(0)),
        }
    }

    /// How many engines it has started; shared by its clones.
    pub fn spawns(&self) -> u32 {
        self.spawns.load(Ordering::SeqCst)
    }
}

/// The fake's stdout, which `kill` closes: the supervisor then reads the end of the stream.
type SharedOut = Arc<Mutex<Option<PipeWriter>>>;

struct FakeProcess {
    pid: u32,
    out: SharedOut,
}

impl Process for FakeProcess {
    fn id(&self) -> u32 {
        self.pid
    }

    fn kill(&mut self) {
        lock(&self.out).take();
    }

    fn wait(&mut self) -> String {
        "killed".into()
    }
}

impl Spawner for FakeSpawner {
    fn spawn(&self) -> io::Result<Spawned> {
        let pid = 1000 + self.spawns.fetch_add(1, Ordering::SeqCst);
        let (stdin_r, stdin_w) = io::pipe()?;
        let (stdout_r, stdout_w) = io::pipe()?;
        let out: SharedOut = Arc::new(Mutex::new(Some(stdout_w)));
        let mode = self.mode;
        let engine_out = Arc::clone(&out);
        thread::spawn(move || fake_engine(mode, pid, stdin_r, &engine_out));
        Ok(Spawned {
            stdin: Box::new(stdin_w),
            stdout: Box::new(stdout_r),
            process: Box::new(FakeProcess { pid, out }),
        })
    }
}

fn send(out: &SharedOut, msg: &FromEngine) -> bool {
    lock(out).as_mut().is_some_and(|w| write_frame(w, msg, &[]).is_ok())
}

/// Speaks the protocol: `Ready`, then one answer per request.
fn fake_engine(mode: Mode, pid: u32, mut stdin: PipeReader, out: &SharedOut) {
    if !matches!(
        read_frame::<_, ToEngine>(&mut stdin, MAX_FRAME),
        Ok((ToEngine::Hello { .. }, _))
    ) {
        return;
    }
    send(
        out,
        &FromEngine::Ready {
            version: PROTOCOL_VERSION,
            pid,
            sandboxed: SandboxLevel::default(),
        },
    );
    match mode {
        Mode::DieAtOnce => {
            lock(out).take();
            return;
        }
        Mode::CrashAfterReady => {
            thread::sleep(Duration::from_millis(30));
            lock(out).take();
            return;
        }
        Mode::Silent => {
            // Keep the pipes open without a word until the supervisor kills it.
            while read_frame::<_, ToEngine>(&mut stdin, MAX_FRAME).is_ok() {}
            return;
        }
        Mode::Healthy | Mode::RefuseReopen | Mode::DieReopeningFirst => {}
    }
    // Later engines (pid past the first) misbehave in the reopen modes.
    let restarted = pid > 1000;
    let beats = Arc::clone(out);
    thread::spawn(move || {
        while send(&beats, &FromEngine::Heartbeat) {
            thread::sleep(Duration::from_millis(20));
        }
    });
    let mut locked: Vec<DocId> = Vec::new();
    while let Ok((msg, _)) = read_frame::<_, ToEngine>(&mut stdin, MAX_FRAME) {
        if let (true, ToEngine::OpenDocument { req, doc, .. }) = (restarted, &msg) {
            match mode {
                Mode::DieReopeningFirst if *doc == DocId(1) => {
                    lock(out).take();
                    return;
                }
                Mode::RefuseReopen => {
                    let refused = FromEngine::Error {
                        req: Some(*req),
                        kind: FailureKind::Invalid,
                        message: "invalid".into(),
                    };
                    if !send(out, &refused) {
                        return;
                    }
                    continue;
                }
                _ => {}
            }
        }
        let Some(reply) = answer(msg, &mut locked) else {
            continue;
        };
        if !send(out, &reply) {
            return;
        }
    }
}

/// The fake's answer to `msg`; `None` for messages without one.
fn answer(msg: ToEngine, locked: &mut Vec<DocId>) -> Option<FromEngine> {
    Some(match msg {
        ToEngine::OpenDocument { req, doc, len } if len % 2 == 0 => {
            locked.push(doc);
            FromEngine::Opened {
                req,
                doc,
                page_count: 0,
                encrypted: true,
                needs_password: true,
                repaired: false,
            }
        }
        ToEngine::OpenDocument { req, doc, .. } => FromEngine::Opened {
            req,
            doc,
            page_count: 3,
            encrypted: false,
            needs_password: false,
            repaired: false,
        },
        ToEngine::UnlockDocument { req, doc, password } if password.expose() == PASSWORD => {
            locked.retain(|d| *d != doc);
            FromEngine::Unlocked {
                req,
                doc,
                page_count: 3,
                repaired: false,
            }
        }
        ToEngine::UnlockDocument { req, .. } => FromEngine::Error {
            req: Some(req),
            kind: FailureKind::WrongPassword,
            message: "wrong-password".into(),
        },
        ToEngine::GetPageInfo { req, doc } if locked.contains(&doc) => FromEngine::NeedsPassword { req, doc },
        ToEngine::GetPageInfo { req, doc } => FromEngine::PageInfo {
            req,
            doc,
            sizes: vec![
                PageDimensions {
                    width: 612.0,
                    height: 792.0,
                    rotate: 0,
                };
                3
            ],
        },
        ToEngine::RenderTile(tile) => FromEngine::Tile {
            req: tile.req,
            w: tile.w,
            h: tile.h,
            format: TileFormat::Rgba8,
        },
        ToEngine::CloseDocument { doc } => {
            locked.retain(|d| *d != doc);
            return None;
        }
        _ => return None,
    })
}
