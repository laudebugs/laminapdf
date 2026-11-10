//! The worker's job queue: document requests first, then visible tiles, then prefetched ones,
//! each in arrival order. The router pushes and cancels; the worker pops.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};

use laminapdf_desktop_protocol::{DocId, Password, Priority, ReqId, TileRequest};
use laminapdf_render::CancelToken;

/// One request for the worker.
pub(crate) enum Job {
    Open { req: ReqId, doc: DocId, len: u64 },
    Unlock { req: ReqId, doc: DocId, password: Password },
    PageInfo { req: ReqId, doc: DocId },
    Render(TileRequest),
    Close { doc: DocId },
}

impl Job {
    /// The request the job answers; `Close` answers none.
    pub(crate) const fn req(&self) -> Option<ReqId> {
        match self {
            Self::Open { req, .. } | Self::Unlock { req, .. } | Self::PageInfo { req, .. } => Some(*req),
            Self::Render(tile) => Some(tile.req),
            Self::Close { .. } => None,
        }
    }

    /// Higher goes first: a document request blocks everything the user waits for after it.
    const fn class(&self) -> u8 {
        match self {
            Self::Render(TileRequest {
                priority: Priority::Prefetch,
                ..
            }) => 0,
            Self::Render(_) => 1,
            _ => 2,
        }
    }
}

/// A job and its place in line.
struct Queued {
    class: u8,
    seq: u64,
    job: Job,
}

impl PartialEq for Queued {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Queued {}

impl PartialOrd for Queued {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Queued {
    /// The heap pops the greatest: the higher class, then the earlier arrival.
    fn cmp(&self, other: &Self) -> Ordering {
        self.class.cmp(&other.class).then_with(|| other.seq.cmp(&self.seq))
    }
}

#[derive(Default)]
struct State {
    jobs: BinaryHeap<Queued>,
    seq: u64,
    closed: bool,
    /// The request the worker is running, and the token that cancels it.
    running: Option<(ReqId, CancelToken)>,
}

/// What [`Queue::cancel`] did.
pub(crate) enum Cancelled {
    /// The job was waiting and is gone: the caller answers it.
    Dequeued,
    /// The worker is running it; it answers once the engine call returns.
    Running,
    /// Nothing waits under that id (already answered).
    Unknown,
}

#[derive(Default)]
pub(crate) struct Queue {
    state: Mutex<State>,
    ready: Condvar,
}

impl Queue {
    pub(crate) fn push(&self, job: Job) {
        let mut state = self.lock();
        state.seq += 1;
        let seq = state.seq;
        state.jobs.push(Queued {
            class: job.class(),
            seq,
            job,
        });
        drop(state);
        self.ready.notify_one();
    }

    /// The next job, waiting for one; `None` once the queue is closed.
    pub(crate) fn pop(&self) -> Option<Job> {
        let mut state = self.lock();
        loop {
            if state.closed {
                return None;
            }
            if let Some(queued) = state.jobs.pop() {
                return Some(queued.job);
            }
            state = self.ready.wait(state).unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// Marks `req` as running: [`Queue::cancel`] then reaches it through `token`.
    pub(crate) fn start(&self, req: ReqId, token: CancelToken) {
        self.lock().running = Some((req, token));
    }

    /// Clears the running request.
    pub(crate) fn finish(&self) {
        self.lock().running = None;
    }

    pub(crate) fn cancel(&self, req: ReqId) -> Cancelled {
        let mut state = self.lock();
        let before = state.jobs.len();
        state.jobs.retain(|queued| queued.job.req() != Some(req));
        if state.jobs.len() < before {
            return Cancelled::Dequeued;
        }
        match &state.running {
            Some((running, token)) if *running == req => {
                token.cancel();
                Cancelled::Running
            }
            _ => Cancelled::Unknown,
        }
    }

    /// Stops the worker: [`Queue::pop`] answers `None` from now on.
    pub(crate) fn close(&self) {
        let mut state = self.lock();
        state.closed = true;
        if let Some((_, token)) = &state.running {
            token.cancel();
        }
        drop(state);
        self.ready.notify_all();
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use laminapdf_desktop_protocol::TileFormat;

    use super::*;

    fn tile(req: u64, priority: Priority) -> Job {
        Job::Render(TileRequest {
            req: ReqId(req),
            doc: DocId(1),
            page: 0,
            scale_milli: 1000,
            x: 0,
            y: 0,
            w: 8,
            h: 8,
            format: TileFormat::Rgba8,
            priority,
        })
    }

    fn order(queue: &Queue, n: usize) -> Vec<Option<ReqId>> {
        (0..n).map(|_| queue.pop().and_then(|job| job.req())).collect()
    }

    #[test]
    fn jobs_should_pop_documents_then_visible_then_prefetched_tiles() {
        let queue = Queue::default();
        queue.push(tile(1, Priority::Prefetch));
        queue.push(tile(2, Priority::Visible));
        queue.push(Job::PageInfo {
            req: ReqId(3),
            doc: DocId(1),
        });
        queue.push(tile(4, Priority::Visible));
        assert_eq!(
            order(&queue, 4),
            [Some(ReqId(3)), Some(ReqId(2)), Some(ReqId(4)), Some(ReqId(1))]
        );
    }

    #[test]
    fn cancelling_a_queued_job_should_remove_it() {
        let queue = Queue::default();
        queue.push(tile(1, Priority::Visible));
        queue.push(tile(2, Priority::Visible));
        assert!(matches!(queue.cancel(ReqId(1)), Cancelled::Dequeued));
        assert_eq!(order(&queue, 1), [Some(ReqId(2))]);
    }

    #[test]
    fn cancelling_the_running_job_should_cancel_its_token() {
        let queue = Queue::default();
        let token = CancelToken::new();
        queue.start(ReqId(5), token.clone());
        assert!(matches!(queue.cancel(ReqId(5)), Cancelled::Running) && token.is_cancelled());
    }

    #[test]
    fn a_closed_queue_should_pop_nothing() {
        let queue = Queue::default();
        queue.push(tile(1, Priority::Visible));
        queue.close();
        assert!(queue.pop().is_none());
    }
}
