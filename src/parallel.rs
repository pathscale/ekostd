//! Joined execution of independent, unit-stride bands.
//!
//! Workers persist for the process lifetime. Each worker has a bounded queue, so one launch
//! assigns exactly one contiguous chunk to each participating worker.

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ffi::c_void;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{Condvar, Mutex, OnceLock, spawn_detached};

const GRAIN: i128 = 1024;
const MAX_WORKERS: usize = 16;

pub type Callback = unsafe extern "C" fn(*mut c_void, i64, i64);

struct Batch {
    callback: Callback,
    context: usize,
    remaining: Mutex<usize>,
    done: Condvar,
}

struct Task {
    batch: Arc<Batch>,
    lower: i64,
    upper: i64,
}

struct Lane {
    queue: Mutex<VecDeque<Task>>,
    ready: Condvar,
    space: Condvar,
}

struct Pool {
    lanes: Vec<Arc<Lane>>,
}

static POOL: OnceLock<Arc<Pool>> = OnceLock::new();
static COMPLETED: AtomicU64 = AtomicU64::new(0);
static PARTICIPANTS: AtomicU64 = AtomicU64::new(0);

/// Diagnostic counters for completed launches.
///
/// The second value describes the most recently completed launch. Concurrent callers can
/// complete between these two reads; this is telemetry, not a synchronization interface.
pub fn snapshot() -> (u64, u64) {
    (
        COMPLETED.load(Ordering::SeqCst),
        PARTICIPANTS.load(Ordering::SeqCst),
    )
}

fn completed(participants: u64) -> u64 {
    PARTICIPANTS.store(participants, Ordering::SeqCst);
    COMPLETED.fetch_add(1, Ordering::SeqCst);
    participants
}

fn processors() -> usize {
    let count = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) };
    if count < 1 {
        1
    } else {
        (count as usize).min(MAX_WORKERS)
    }
}

fn worker(lane: Arc<Lane>) -> ! {
    loop {
        let task = {
            let mut queue = lane.queue.lock();
            while queue.is_empty() {
                queue = lane.ready.wait(queue);
            }
            let task = queue.pop_front().expect("a notified lane has a queued task");
            lane.space.notify_one();
            task
        };
        // The submitting thread keeps the context alive until every task has completed.
        // Generated callbacks cannot call another dispatcher or unwind across this ABI.
        unsafe {
            (task.batch.callback)(
                task.batch.context as *mut c_void,
                task.lower,
                task.upper,
            );
        }
        let mut remaining = task.batch.remaining.lock();
        *remaining -= 1;
        if *remaining == 0 {
            task.batch.done.notify_all();
        }
    }
}

fn pool() -> &'static Arc<Pool> {
    POOL.get_or_init(|| {
        let mut lanes = Vec::new();
        for _ in 0..processors() {
            let lane = Arc::new(Lane {
                queue: Mutex::new(VecDeque::new()),
                ready: Condvar::new(),
                space: Condvar::new(),
            });
            let child = Arc::clone(&lane);
            if spawn_detached(2 * 1024 * 1024, move || worker(child)).is_ok() {
                lanes.push(lane);
            }
        }
        Arc::new(Pool { lanes })
    })
}

/// Execute `[lower, upper)` and return the number of participating threads.
///
/// Small bands and unavailable workers execute on the calling thread. Empty bands invoke no
/// callback. All worker writes happen before this function returns.
///
/// # Safety
///
/// `callback` must implement the C ABI above, execute only the supplied half-open range, and
/// neither unwind nor recursively dispatch work. Its captures and accessed storage must stay
/// valid until this call returns. Different iterations must be independent, including writes
/// through aliased pointers. The compiler supplies that proof before selecting this service.
#[no_mangle]
pub unsafe extern "C" fn ekostd_parallel_for(
    callback: Callback,
    context: *mut c_void,
    lower: i64,
    upper: i64,
) -> u64 {
    let count = i128::from(upper) - i128::from(lower);
    if count <= 0 {
        return completed(0);
    }
    let available = processors();
    if available < 2 || count < GRAIN * available as i128 {
        unsafe { callback(context, lower, upper) };
        return completed(1);
    }
    let pool = pool();
    let workers = pool.lanes.len();
    if workers < 2 || count < GRAIN * workers as i128 {
        unsafe { callback(context, lower, upper) };
        return completed(1);
    }
    let batch = Arc::new(Batch {
        callback,
        context: context as usize,
        remaining: Mutex::new(workers),
        done: Condvar::new(),
    });
    for (index, lane) in pool.lanes.iter().enumerate() {
        // Widen before subtraction and multiplication. Even MIN..MAX is representable here.
        // Adjacent chunks share one boundary and the final boundary is exactly upper.
        let begin = i128::from(lower) + count * index as i128 / workers as i128;
        let end = i128::from(lower) + count * (index + 1) as i128 / workers as i128;
        let mut queue = lane.queue.lock();
        while !queue.is_empty() {
            queue = lane.space.wait(queue);
        }
        queue.push_back(Task {
            batch: Arc::clone(&batch),
            lower: begin as i64,
            upper: end as i64,
        });
        lane.ready.notify_one();
    }
    let mut remaining = batch.remaining.lock();
    while *remaining != 0 {
        remaining = batch.done.wait(remaining);
    }
    completed(workers as u64)
}

