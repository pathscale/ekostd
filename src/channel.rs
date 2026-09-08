//! A bounded channel that can say the other end is gone.
//!
//! `std::sync::mpsc::sync_channel`, over the [`Mutex`](crate::thread::Mutex) and
//! [`Condvar`](crate::thread::Condvar) next door. The note at the end of `thread.rs` says why
//! this is here rather than deferred to `ps-spsc`: a ring buffer can report *empty*, and the
//! caller that needs this needs *the peer is gone*, which is a different answer.
//!
//! The caller is `proc_macro`'s cross-thread executor. The compiler runs a proc macro on its own
//! thread and pumps requests back over a pair of these; the server's loop is
//! `while let Some(b) = server.recv()`, so a `recv` that could not distinguish a dead client from
//! an idle one would either spin forever or drop the last reply.
//!
//! # Not poisoned, and not multi-consumer
//!
//! There is no poisoning, for the same reason [`Mutex`](crate::thread::Mutex) has none. The
//! receiver is not `Clone`: one consumer, any number of producers, which is what "mpsc" says and
//! what the disconnect accounting below assumes.

use alloc::collections::VecDeque;
use alloc::sync::Arc;

use crate::thread::{Condvar, Mutex};

/// The queue both ends share.
struct Shared<T> {
    state: Mutex<State<T>>,
    /// Woken when the queue gains an item, or when the last sender goes.
    on_send: Condvar,
    /// Woken when the queue loses an item, or when the receiver goes.
    on_recv: Condvar,
    capacity: usize,
}

struct State<T> {
    queue: VecDeque<T>,
    /// How many `SyncSender`s exist. Zero with an empty queue is what makes `recv` return `None`.
    senders: usize,
    /// Whether the `Receiver` still exists. `false` is what makes `send` fail rather than block
    /// forever against a queue nobody will drain.
    receiver: bool,
}

/// A bounded channel holding at most `capacity` items.
///
/// `capacity` is a buffer, not a rendezvous: a send returns as soon as the item is queued, and
/// only blocks once `capacity` items are waiting. That is `std::sync::mpsc::sync_channel`'s
/// meaning of the word, and the caller relies on it - with a rendezvous, the client's `send`
/// would not return until the server had already come back round to `recv`.
pub fn sync_channel<T>(capacity: usize) -> (SyncSender<T>, Receiver<T>) {
    let shared = Arc::new(Shared {
        state: Mutex::new(State { queue: VecDeque::new(), senders: 1, receiver: true }),
        on_send: Condvar::new(),
        on_recv: Condvar::new(),
        capacity,
    });
    (SyncSender { shared: Arc::clone(&shared) }, Receiver { shared })
}

/// The value a send could not deliver, handed back rather than dropped.
#[derive(Debug, PartialEq, Eq)]
pub struct SendError<T>(pub T);

/// Returned by `recv` when every sender is gone and nothing is left queued.
#[derive(Debug, PartialEq, Eq)]
pub struct RecvError;

/// The sending half. Cloneable: many producers, one consumer.
pub struct SyncSender<T> {
    shared: Arc<Shared<T>>,
}

// The channel is the synchronisation, so the ends travel wherever `T` may.
unsafe impl<T: Send> Send for SyncSender<T> {}
unsafe impl<T: Send> Sync for SyncSender<T> {}

impl<T> Clone for SyncSender<T> {
    fn clone(&self) -> SyncSender<T> {
        self.shared.state.lock().senders += 1;
        SyncSender { shared: Arc::clone(&self.shared) }
    }
}

impl<T> SyncSender<T> {
    /// Queue a value, blocking while the channel is full.
    ///
    /// `Err` means the receiver is gone, and carries the value back so a caller that wants to
    /// retry elsewhere still has it.
    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        let mut state = self.shared.state.lock();
        loop {
            if !state.receiver {
                return Err(SendError(value));
            }
            if state.queue.len() < self.shared.capacity {
                state.queue.push_back(value);
                drop(state);
                self.shared.on_send.notify_one();
                return Ok(());
            }
            // Spurious wakes are permitted, which is why this is a loop and not an `if`.
            state = self.shared.on_recv.wait(state);
        }
    }
}

impl<T> Drop for SyncSender<T> {
    fn drop(&mut self) {
        let mut state = self.shared.state.lock();
        state.senders -= 1;
        let last = state.senders == 0;
        drop(state);
        if last {
            // A receiver blocked in `recv` is waiting for exactly this news.
            self.shared.on_send.notify_all();
        }
    }
}

/// The receiving half. Not `Clone`: one consumer.
pub struct Receiver<T> {
    shared: Arc<Shared<T>>,
}

unsafe impl<T: Send> Send for Receiver<T> {}

impl<T> Receiver<T> {
    /// Take the next value, blocking until there is one.
    ///
    /// `Err(RecvError)` means every sender has been dropped *and* the queue is drained. Items
    /// already queued are delivered first, so a producer that sends and then exits does not lose
    /// its last message.
    pub fn recv(&self) -> Result<T, RecvError> {
        let mut state = self.shared.state.lock();
        loop {
            if let Some(value) = state.queue.pop_front() {
                drop(state);
                self.shared.on_recv.notify_one();
                return Ok(value);
            }
            if state.senders == 0 {
                return Err(RecvError);
            }
            state = self.shared.on_send.wait(state);
        }
    }

    /// Take a value if one is already queued, without blocking.
    ///
    /// `None` covers both "nothing yet" and "nobody left"; a caller that needs to tell those
    /// apart wants `recv`.
    pub fn try_recv(&self) -> Option<T> {
        let mut state = self.shared.state.lock();
        let value = state.queue.pop_front();
        drop(state);
        if value.is_some() {
            self.shared.on_recv.notify_one();
        }
        value
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        let mut state = self.shared.state.lock();
        state.receiver = false;
        drop(state);
        // A sender blocked on a full queue would otherwise wait for a drain that cannot come.
        self.shared.on_recv.notify_all();
    }
}
