#![no_std]
#![no_main]

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use eko::worker_runtime::{
    Barrier, CancelResult, Cancellation, Completion, Runtime, Shutdown, Status, SubmitError,
    Task, WaitError, WorkResult, Worker,
};

#[global_allocator]
static ALLOCATOR: eko::heap::Malloc = eko::heap::Malloc;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    eko::eprintln!("worker-runtime-witness: witnesses/worker-runtime.rs: unexpected panic");
    eko::exit(101)
}

struct Storage {
    values: [AtomicU64; 64],
    identities: [AtomicU64; 4],
    arrived: AtomicUsize,
    release: AtomicBool,
    release_last: AtomicBool,
    final_written: AtomicBool,
}

struct Partition {
    storage: Arc<Storage>,
    index: usize,
}

extern "C" fn partition(
    capture: &Partition,
    _: &Cancellation,
    worker: Worker,
) -> WorkResult {
    let storage = &capture.storage;
    storage.identities[capture.index].store(worker.os_thread, Ordering::Release);
    storage.arrived.fetch_add(1, Ordering::AcqRel);
    while !storage.release.load(Ordering::Acquire) {
        eko::sleep_ms(1);
    }
    if capture.index == 3 {
        while !storage.release_last.load(Ordering::Acquire) {
            eko::sleep_ms(1);
        }
    }
    for index in capture.index * 16..(capture.index + 1) * 16 {
        storage.values[index].store(index as u64 * 3 + 7, Ordering::Relaxed);
    }
    if capture.index == 3 {
        storage.final_written.store(true, Ordering::Release);
    }
    WorkResult::Complete
}

struct Control {
    entered: AtomicBool,
    release: AtomicBool,
    ran: AtomicUsize,
}

impl Control {
    fn new() -> Self {
        Self {
            entered: AtomicBool::new(false),
            release: AtomicBool::new(false),
            ran: AtomicUsize::new(0),
        }
    }
}

extern "C" fn blocked(control: &Control, _: &Cancellation, _: Worker) -> WorkResult {
    control.entered.store(true, Ordering::Release);
    while !control.release.load(Ordering::Acquire) {
        eko::sleep_ms(1);
    }
    WorkResult::Complete
}

extern "C" fn counted(control: &Control, _: &Cancellation, _: Worker) -> WorkResult {
    control.ran.fetch_add(1, Ordering::Relaxed);
    WorkResult::Complete
}

extern "C" fn aborted(_: &u8, _: &Cancellation, _: Worker) -> WorkResult {
    WorkResult::Abort
}

struct Reentrant {
    runtime: Arc<Runtime>,
    finished: Task,
}

extern "C" fn reentrant(capture: &Reentrant, _: &Cancellation, _: Worker) -> WorkResult {
    if capture.finished.wait() == Err(WaitError::ReentrantWait)
        && Barrier::new().wait() == Err(WaitError::ReentrantWait)
        && capture.runtime.shutdown() == Err(WaitError::ReentrantWait)
    {
        WorkResult::Complete
    } else {
        WorkResult::Abort
    }
}

fn fail(message: &str) -> i32 {
    eko::eprintln!(
        "worker-runtime-witness: witnesses/worker-runtime.rs: {}; barrier=src/worker_runtime.rs::Barrier::wait",
        message
    );
    1
}

#[no_mangle]
pub extern "C" fn main() -> i32 {
    let runtime = Runtime::new(4, 4).expect("four OS workers");
    let storage = Arc::new(Storage {
        values: core::array::from_fn(|_| AtomicU64::new(0)),
        identities: core::array::from_fn(|_| AtomicU64::new(0)),
        arrived: AtomicUsize::new(0),
        release: AtomicBool::new(false),
        release_last: AtomicBool::new(false),
        final_written: AtomicBool::new(false),
    });
    let mut barrier = Barrier::new();
    let mut tasks = Vec::new();
    for index in 0..4 {
        tasks.push(
            barrier
                .submit(
                    &runtime,
                    Arc::new(Partition { storage: storage.clone(), index }),
                    partition,
                )
                .expect("partition admission"),
        );
    }
    while storage.arrived.load(Ordering::Acquire) != 4 {
        // A dropped runnable must produce disagreement rather than hanging at the rendezvous.
        if tasks.iter().any(|task| matches!(task.status(), Status::Finished(_))) {
            break;
        }
        eko::sleep_ms(1);
    }
    let delayed = storage.clone();
    let delay = eko::thread::spawn(move || {
        eko::sleep_ms(200);
        delayed.release_last.store(true, Ordering::Release);
    })
    .expect("delay controller");
    storage.release.store(true, Ordering::Release);

    let result = barrier.wait().expect("caller may wait");
    // Snapshot before joining the delay controller or shutting down the runtime.
    let values: [u64; 64] =
        core::array::from_fn(|index| storage.values[index].load(Ordering::Relaxed));
    let final_written = storage.final_written.load(Ordering::Acquire);
    let identities: [u64; 4] =
        core::array::from_fn(|index| storage.identities[index].load(Ordering::Acquire));

    delay.join().expect("delay controller completion");
    runtime.shutdown().expect("partition shutdown");
    for value in values {
        eko::print!("{} ", value);
    }
    eko::println!();
    eko::println!(
        "batch completed={} cancelled={} aborted={} refused={}",
        result.completed, result.cancelled, result.aborted, result.refused
    );
    eko::println!("barrier_final={}", u8::from(final_written));

    let caller = eko::thread::current_id().as_u64();
    eko::eprintln!(
        "identities caller={} workers={},{},{},{}",
        caller, identities[0], identities[1], identities[2], identities[3]
    );
    if !result.succeeded() || result.completed != 4 || !final_written {
        return fail("completion barrier returned an incomplete partition batch");
    }
    for index in 0..4 {
        if identities[index] == 0
            || identities[index] == caller
            || identities[..index].contains(&identities[index])
        {
            return fail("partitions did not execute on four distinct OS workers");
        }
    }

    let bounded = Runtime::new(1, 2).expect("bounded worker");
    let blocker = Arc::new(Control::new());
    let pending = Arc::new(Control::new());
    let mut bounded_barrier = Barrier::new();
    bounded_barrier
        .submit(&bounded, blocker.clone(), blocked)
        .expect("first admission");
    while !blocker.entered.load(Ordering::Acquire) {
        eko::sleep_ms(1);
    }
    let cancelled = bounded_barrier
        .submit(&bounded, pending.clone(), counted)
        .expect("queued admission");
    if cancelled.cancel() != CancelResult::Requested {
        return fail("queued cancellation was not requested");
    }
    if !matches!(
        bounded_barrier.submit(&bounded, pending.clone(), counted),
        Err(SubmitError::Full)
    ) {
        return fail("capacity did not refuse a third outstanding job");
    }
    blocker.release.store(true, Ordering::Release);
    let bounded_result = bounded_barrier.wait().expect("bounded completion");
    let ran = pending.ran.load(Ordering::Relaxed);
    eko::println!(
        "bounded completed={} cancelled={} aborted={} refused={} ran={}",
        bounded_result.completed,
        bounded_result.cancelled,
        bounded_result.aborted,
        bounded_result.refused,
        ran
    );
    if bounded_result.accepted != 2
        || bounded_result.completed != 1
        || bounded_result.cancelled != 1
        || bounded_result.aborted != 0
        || bounded_result.refused != 1
        || bounded_result.succeeded()
        || ran != 0
        || cancelled.cancel() != CancelResult::AlreadyFinished(Completion::Cancelled)
    {
        return fail("cancellation or failed submission was reported as success");
    }
    let first_shutdown = bounded.shutdown().expect("bounded shutdown");
    let second_shutdown = bounded.shutdown().expect("repeated shutdown");
    let closed = matches!(bounded.submit(pending, counted), Err(SubmitError::Closed));
    let shutdown = first_shutdown == Shutdown::Drained && second_shutdown == Shutdown::AlreadyClosed;
    eko::println!("closed={} shutdown={}", u8::from(closed), u8::from(shutdown));
    if !closed || !shutdown {
        return fail("shutdown did not close admission and drain workers");
    }

    let abort_runtime = Runtime::new(1, 1).expect("abort worker");
    let mut abort_barrier = Barrier::new();
    let finished = abort_barrier
        .submit(&abort_runtime, Arc::new(0u8), aborted)
        .expect("abort admission");
    let abort_result = abort_barrier.wait().expect("abort completion");
    eko::println!(
        "aborted completed={} cancelled={} aborted={} refused={}",
        abort_result.completed,
        abort_result.cancelled,
        abort_result.aborted,
        abort_result.refused
    );
    abort_runtime.shutdown().expect("abort shutdown");
    if abort_result.aborted != 1 || abort_result.succeeded() {
        return fail("explicit abort was reported as success");
    }

    let reentrant_runtime = Arc::new(Runtime::new(1, 1).expect("reentrant worker"));
    let mut reentrant_barrier = Barrier::new();
    reentrant_barrier
        .submit(
            &reentrant_runtime,
            Arc::new(Reentrant { runtime: reentrant_runtime.clone(), finished }),
            reentrant,
        )
        .expect("reentrant admission");
    let refused = reentrant_barrier.wait().expect("external completion").succeeded();
    reentrant_runtime.shutdown().expect("external shutdown");
    eko::println!("reentrant={}", u8::from(refused));
    if !refused {
        return fail("a worker blocking operation was not refused");
    }
    0
}
