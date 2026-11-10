//! [`RemoteSource`]: a document's bytes, read from the main process one block at a time.
//!
//! The parser and PDFium call [`ByteSource::read_at`] synchronously. A block that isn't cached
//! becomes a `ReadRange` request; the calling thread waits for the `RangeData` that the router
//! thread hands over through [`Ranges::deliver`]. The router never parses or renders, so it is
//! always free to deliver, which is what keeps a parse waiting on its bytes from deadlocking.

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use laminapdf_render::{ByteSource, SourceError};
use laminapdf_desktop_protocol::{DocId, FromEngine, ReqId};

use crate::outbox::Outbox;

/// The bytes asked for at once: reads are rounded out to whole blocks.
pub(crate) const BLOCK: u64 = 256 * 1024;

/// Blocks kept per document (16 MiB): the parser revisits the xref, the page tree and fonts.
const CACHED_BLOCKS: usize = 64;

/// How long a read waits for the main process before failing the call.
const TIMEOUT: Duration = Duration::from_secs(30);

/// The answer to one `ReadRange`: the bytes, or why the main process couldn't read them.
type Answer = Result<Vec<u8>, String>;

/// The engine's outstanding `ReadRange` requests, shared by every document's source and the
/// router that answers them.
pub(crate) struct Ranges {
    out: Outbox,
    next: AtomicU64,
    waiting: Mutex<BTreeMap<ReqId, mpsc::Sender<Answer>>>,
    closed: AtomicBool,
}

impl Ranges {
    pub(crate) fn new(out: Outbox) -> Arc<Self> {
        Arc::new(Self {
            out,
            next: AtomicU64::new(1),
            waiting: Mutex::new(BTreeMap::new()),
            closed: AtomicBool::new(false),
        })
    }

    /// Hands an answer to the read waiting for it. An answer nobody waits for (its read timed
    /// out) is dropped.
    pub(crate) fn deliver(&self, req: ReqId, answer: Answer) {
        let sender = self.lock().remove(&req);
        if let Some(sender) = sender {
            // The receiver is gone only if its read gave up; nothing to do.
            let _ = sender.send(answer);
        }
    }

    /// Fails every waiting read and every later one: the main process is gone.
    pub(crate) fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.lock().clear();
    }

    /// Asks the main process for `len` bytes of `doc` from `offset` and waits for them.
    fn fetch(&self, doc: DocId, offset: u64, len: u32) -> Answer {
        if self.closed.load(Ordering::SeqCst) {
            return Err("the main process is gone".into());
        }
        let req = ReqId(self.next.fetch_add(1, Ordering::Relaxed));
        let (tx, rx) = mpsc::channel();
        self.lock().insert(req, tx);
        let asked = FromEngine::ReadRange { req, doc, offset, len };
        if let Err(error) = self.out.send(&asked, &[]) {
            self.lock().remove(&req);
            return Err(format!("could not ask for bytes: {error}"));
        }
        match rx.recv_timeout(TIMEOUT) {
            Ok(answer) => answer,
            Err(RecvTimeoutError::Timeout) => {
                self.lock().remove(&req);
                Err(format!("no answer within {} s", TIMEOUT.as_secs()))
            }
            Err(RecvTimeoutError::Disconnected) => Err("the main process is gone".into()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeMap<ReqId, mpsc::Sender<Answer>>> {
        self.waiting.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// One document's bytes, `len` long, read from the main process with a small block cache.
pub(crate) struct RemoteSource {
    doc: DocId,
    len: u64,
    ranges: Arc<Ranges>,
    cache: Mutex<VecDeque<(u64, Arc<[u8]>)>>,
}

impl RemoteSource {
    pub(crate) fn new(doc: DocId, len: u64, ranges: Arc<Ranges>) -> Self {
        Self {
            doc,
            len,
            ranges,
            cache: Mutex::new(VecDeque::with_capacity(CACHED_BLOCKS)),
        }
    }

    /// Block `index`, from the cache or the main process.
    fn block(&self, index: u64) -> Result<Arc<[u8]>, SourceError> {
        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(pos) = cache.iter().position(|(i, _)| *i == index) {
            // Most recently used goes to the front.
            if let Some(entry) = cache.remove(pos) {
                let bytes = Arc::clone(&entry.1);
                cache.push_front(entry);
                return Ok(bytes);
            }
        }
        // The lock is dropped while waiting, so other threads read cached blocks meanwhile.
        drop(cache);
        let start = index * BLOCK;
        let want = BLOCK.min(self.len.saturating_sub(start));
        let len = u32::try_from(want).unwrap_or(u32::MAX);
        let bytes: Arc<[u8]> = match self.ranges.fetch(self.doc, start, len) {
            Ok(bytes) if bytes.len() as u64 == want => bytes.into(),
            Ok(bytes) => {
                crate::log(&format!(
                    "document {}: asked for {want} bytes at {start}, got {}",
                    self.doc.0,
                    bytes.len()
                ));
                return Err(SourceError::Failed { offset: start, len: want });
            }
            Err(why) => {
                crate::log(&format!("document {}: bytes at {start} failed: {why}", self.doc.0));
                return Err(SourceError::Failed { offset: start, len: want });
            }
        };
        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        cache.push_front((index, Arc::clone(&bytes)));
        cache.truncate(CACHED_BLOCKS);
        Ok(bytes)
    }
}

impl ByteSource for RemoteSource {
    fn len(&self) -> u64 {
        self.len
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
        let mut filled = 0;
        while filled < buf.len() {
            let at = offset + filled as u64;
            if at >= self.len {
                break;
            }
            let block = self.block(at / BLOCK)?;
            let within = usize::try_from(at % BLOCK).unwrap_or(usize::MAX);
            let Some(src) = block.get(within..) else { break };
            let Some(dst) = buf.get_mut(filled..) else { break };
            let n = src.len().min(dst.len());
            if let (Some(dst), Some(src)) = (dst.get_mut(..n), src.get(..n)) {
                dst.copy_from_slice(src);
            }
            if n == 0 {
                break;
            }
            filled += n;
        }
        Ok(filled)
    }
}
