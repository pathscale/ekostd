//! Bounded synchronous work on host-owned OS workers.
//!
//! This first callback contract supports abort-on-panic artifacts only. An unwind build is
//! refused before starting workers. Callbacks use a non-unwinding C ABI and report an explicit
//! abort rather than throwing. A process abort cannot be recovered as a completion result.
//!
//! Admission counts queued and running jobs together. Cancellation never releases admission
//! early and never rolls back writes. Shutdown closes admission, drains every accepted job,
//! then stops the pool and waits for every worker to leave it.
//!
//! A barrier retains all its captures, including completed ones. Dropping handles detaches
//! observation; queued and running jobs still own their captures. No borrowed stack storage
//! crosses submission. Waiting from any runtime worker is refused, including cross-pool waits.

use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use nagoya::{Executor, WorkerContext};
use st3::fanout::{AtomicHost, Pool};

use crate::thread::{Condvar, Mutex, OnceLock, ScopedKey};
use crate::{thread, time::Instant, Errno};

/// Version of the callback and completion contract.
pub const CONTRACT_VERSION: u32 = 1;

/// Identity of the worker actually executing a callback.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Worker {
    pub index: usize,
    pub os_thread: u64,
}

/// A callback's explicit outcome. This is not panic recovery.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkResult {
    Complete,
    Abort,
}

/// Terminal outcome after a job has stopped accessing its capture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Completion {
    Completed,
    Cancelled,
    Aborted,
}

/// Observable job progress. Cancellation requests do not imply terminal completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Queued,
    Running,
    Finished(Completion),
}

/// Construction refuses unsupported contracts before accepting work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartError {
    UnsupportedUnwind,
    InvalidWorkerCount,
    ZeroCapacity,
    WorkerStart(Errno),
}

/// A refusal means no job was accepted and no callback will run for that submission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmitError {
    Full,
    Closed,
}

/// A blocking operation attempted from a worker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaitError {
    ReentrantWait,
}

/// Cancellation is cooperative once the callback has started.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancelResult {
    Requested,
    AlreadyFinished(Completion),
}

/// Both outcomes mean all accepted jobs and OS workers have finished.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shutdown {
    Drained,
    AlreadyClosed,
}

/// Borrowed by the callback for its duration; a request does not roll back writes.
pub struct Cancellation {
    requested: AtomicBool,
}

impl Cancellation {
    pub fn is_cancelled(&self) -> bool {
        self.requested.load(Ordering::Acquire)
    }
}

struct Identity {
    pool: Arc<Pool>,
    index: usize,
}

static CURRENT: ScopedKey<Identity> = ScopedKey::new();
static CLOCK_ORIGIN: OnceLock<Instant> = OnceLock::new();

fn clock_ns() -> u64 {
    CLOCK_ORIGIN
        .get_or_init(Instant::now)
        .elapsed()
        .as_nanos()
        .min(u64::MAX as u128) as u64
}

struct Context;

impl WorkerContext for Context {
    fn current_worker(&self, pool: &Pool) -> Option<usize> {
        if !CURRENT.is_set() {
            return None;
        }
        CURRENT.with(|identity| {
            core::ptr::eq(identity.pool.as_ref(), pool).then_some(identity.index)
        })
    }

    fn with_current_worker(&self, visit: &mut dyn FnMut(&Pool, usize)) {
        if CURRENT.is_set() {
            CURRENT.with(|identity| visit(identity.pool.as_ref(), identity.index));
        }
    }
}

fn refuse_worker_wait() -> Result<(), WaitError> {
    if CURRENT.is_set() {
        Err(WaitError::ReentrantWait)
    } else {
        Ok(())
    }
}

struct Ledger {
    closing: bool,
    active: usize,
    started: usize,
    exited: usize,
}

struct Shared {
    ledger: Mutex<Ledger>,
    changed: Condvar,
    pool: Weak<Pool>,
    capacity: usize,
}

impl Shared {
    fn close(&self) -> bool {
        let mut ledger = self.ledger.lock();
        let newly_closed = !ledger.closing;
        ledger.closing = true;
        if ledger.active == 0 {
            if let Some(pool) = self.pool.upgrade() {
                pool.shut_down();
            }
        }
        newly_closed
    }

    fn finish_job(&self) {
        let mut ledger = self.ledger.lock();
        ledger.active -= 1;
        if ledger.closing && ledger.active == 0 {
            if let Some(pool) = self.pool.upgrade() {
                pool.shut_down();
            }
        }
        self.changed.notify_all();
    }

    fn drain(&self) {
        let mut ledger = self.ledger.lock();
        while ledger.active != 0 || ledger.exited != ledger.started {
            ledger = self.changed.wait(ledger);
        }
    }
}

/// Owns a private pool. The executor and pool are intentionally not exposed.
pub struct Runtime {
    executor: Executor,
    shared: Arc<Shared>,
}

impl Runtime {
    pub fn new(workers: usize, capacity: usize) -> Result<Self, StartError> {
        if cfg!(panic = "unwind") {
            return Err(StartError::UnsupportedUnwind);
        }
        if workers == 0 || workers > usize::BITS as usize {
            return Err(StartError::InvalidWorkerCount);
        }
        if capacity == 0 {
            return Err(StartError::ZeroCapacity);
        }

        let host = Arc::new(AtomicHost::new(workers, clock_ns));
        let pool = Pool::new(workers, 1024, host);
        let shared = Arc::new(Shared {
            ledger: Mutex::new(Ledger {
                closing: false,
                active: 0,
                started: 0,
                exited: 0,
            }),
            changed: Condvar::new(),
            pool: Arc::downgrade(&pool),
            capacity,
        });
        let runtime = Self {
            executor: Executor::with_worker_context(pool.clone(), Arc::new(Context)),
            shared,
        };

        for index in 0..workers {
            let runner = pool.runner(index);
            let worker_pool = pool.clone();
            let worker_shared = runtime.shared.clone();
            runtime.shared.ledger.lock().started += 1;
            let started = thread::spawn_detached(2 * 1024 * 1024, move || {
                let identity = Identity { pool: worker_pool.clone(), index };
                CURRENT.set(&identity, || {
                    worker_pool.run(runner);
                });
                let mut ledger = worker_shared.ledger.lock();
                ledger.exited += 1;
                worker_shared.changed.notify_all();
            });
            if let Err(error) = started {
                runtime.shared.ledger.lock().started -= 1;
                runtime.shared.close();
                runtime.shared.drain();
                return Err(StartError::WorkerStart(error));
            }
        }
        Ok(runtime)
    }

    /// Accept an Arc-owned capture and a callback that cannot unwind across its ABI.
    ///
    /// Full and Closed are refusals, not completed jobs. The caller may retain another Arc.
    pub fn submit<T: Send + Sync + 'static>(
        &self,
        capture: Arc<T>,
        callback: extern "C" fn(&T, &Cancellation, Worker) -> WorkResult,
    ) -> Result<Task, SubmitError> {
        {
            let mut ledger = self.shared.ledger.lock();
            if ledger.closing {
                return Err(SubmitError::Closed);
            }
            if ledger.active == self.shared.capacity {
                return Err(SubmitError::Full);
            }
            ledger.active += 1;
        }

        let state = Arc::new(TaskState {
            status: Mutex::new(Status::Queued),
            ready: Condvar::new(),
            cancellation: Cancellation { requested: AtomicBool::new(false) },
            _capture: capture.clone(),
        });
        let task = Task { state: state.clone() };
        let work = Job {
            capture: Some(capture),
            callback,
            state,
            shared: self.shared.clone(),
            finished: false,
        };
        self.executor.submit(move || work.run());
        Ok(task)
    }

    /// Drain without abandoning queued callbacks. A worker refusal leaves admission open.
    pub fn shutdown(&self) -> Result<Shutdown, WaitError> {
        refuse_worker_wait()?;
        let newly_closed = self.shared.close();
        self.shared.drain();
        Ok(if newly_closed { Shutdown::Drained } else { Shutdown::AlreadyClosed })
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.shared.close();
        // A worker cannot join its own pool. Its Arc leases remain live, and the last accepted
        // job stops the pool. Off a worker, destruction has the same drain boundary as shutdown.
        if !CURRENT.is_set() {
            self.shared.drain();
        }
    }
}

struct TaskState {
    status: Mutex<Status>,
    ready: Condvar,
    cancellation: Cancellation,
    // Barrier handles retain every capture until the whole barrier can be dropped.
    _capture: Arc<dyn Send + Sync>,
}

/// Cloneable observation and cancellation of one accepted job.
#[derive(Clone)]
pub struct Task {
    state: Arc<TaskState>,
}

impl Task {
    pub fn status(&self) -> Status {
        *self.state.status.lock()
    }

    pub fn cancel(&self) -> CancelResult {
        let status = self.state.status.lock();
        if let Status::Finished(result) = *status {
            return CancelResult::AlreadyFinished(result);
        }
        self.state.cancellation.requested.store(true, Ordering::Release);
        CancelResult::Requested
    }

    /// The mutex acquisition publishes all callback writes before returning a terminal result.
    pub fn wait(&self) -> Result<Completion, WaitError> {
        refuse_worker_wait()?;
        let mut status = self.state.status.lock();
        loop {
            if let Status::Finished(result) = *status {
                return Ok(result);
            }
            status = self.state.ready.wait(status);
        }
    }
}

struct Job<T: Send + Sync + 'static> {
    capture: Option<Arc<T>>,
    callback: extern "C" fn(&T, &Cancellation, Worker) -> WorkResult,
    state: Arc<TaskState>,
    shared: Arc<Shared>,
    finished: bool,
}

impl<T: Send + Sync + 'static> Job<T> {
    fn run(mut self) {
        let cancelled = {
            let mut status = self.state.status.lock();
            let cancelled = self.state.cancellation.is_cancelled();
            if !cancelled {
                *status = Status::Running;
            }
            cancelled
        };
        if cancelled {
            self.finish(Completion::Cancelled);
            return;
        }
        let Some(worker) = CURRENT.is_set().then(|| {
            CURRENT.with(|identity| Worker {
                index: identity.index,
                os_thread: thread::current_id().as_u64(),
            })
        }) else {
            self.finish(Completion::Aborted);
            return;
        };
        let result = (self.callback)(
            self.capture.as_deref().expect("an unfinished job owns its capture"),
            &self.state.cancellation,
            worker,
        );
        let completion = match result {
            WorkResult::Abort => Completion::Aborted,
            WorkResult::Complete if self.state.cancellation.is_cancelled() => Completion::Cancelled,
            WorkResult::Complete => Completion::Completed,
        };
        self.finish(completion);
    }

    fn finish(&mut self, result: Completion) {
        if self.finished {
            return;
        }
        self.finished = true;
        self.capture.take();
        {
            let mut status = self.state.status.lock();
            *status = Status::Finished(result);
            self.state.ready.notify_all();
        }
        self.shared.finish_job();
    }
}

impl<T: Send + Sync + 'static> Drop for Job<T> {
    fn drop(&mut self) {
        // Queue disposal is an abort, never successful completion or an admission refund alone.
        self.finish(Completion::Aborted);
    }
}

/// Aggregate outcomes retain the distinction between refusal and accepted-job completion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BatchResult {
    pub accepted: usize,
    pub completed: usize,
    pub cancelled: usize,
    pub aborted: usize,
    pub refused: usize,
}

impl BatchResult {
    pub fn succeeded(self) -> bool {
        self.completed == self.accepted
            && self.cancelled == 0
            && self.aborted == 0
            && self.refused == 0
    }
}

/// A completion barrier and owner of the captures for all accepted submissions.
#[derive(Default)]
pub struct Barrier {
    tasks: Vec<Task>,
    refused: usize,
}

impl Barrier {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record both admission success and refusal; successful handles may be used to cancel.
    pub fn submit<T: Send + Sync + 'static>(
        &mut self,
        runtime: &Runtime,
        capture: Arc<T>,
        callback: extern "C" fn(&T, &Cancellation, Worker) -> WorkResult,
    ) -> Result<Task, SubmitError> {
        match runtime.submit(capture, callback) {
            Ok(task) => {
                self.tasks.push(task.clone());
                Ok(task)
            }
            Err(error) => {
                self.refused += 1;
                Err(error)
            }
        }
    }

    /// Wait for every accepted job, including cancelled and aborted jobs.
    ///
    /// A failed submission does not skip the drain of earlier accepted jobs. Repeated waits
    /// observe the same terminal results. The barrier continues retaining captures afterwards.
    pub fn wait(&self) -> Result<BatchResult, WaitError> {
        refuse_worker_wait()?;
        let mut result = BatchResult {
            accepted: self.tasks.len(),
            refused: self.refused,
            ..BatchResult::default()
        };
        for task in &self.tasks {
            match task.wait()? {
                Completion::Completed => result.completed += 1,
                Completion::Cancelled => result.cancelled += 1,
                Completion::Aborted => result.aborted += 1,
            }
        }
        Ok(result)
    }
}
