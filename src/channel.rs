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

use alloc::boxed::Box;

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

struct Node<T> {
    value: T,
    next: *mut Node<T>,
}

struct State<T> {
    head: *mut Node<T>,
    tail: *mut Node<T>,
    len: usize,
    /// How many `SyncSender`s exist. Zero with an empty queue is what makes `recv` return `None`.
    senders: usize,
    /// Whether the `Receiver` still exists. `false` is what makes `send` fail rather than block
    /// forever against a queue nobody will drain.
    receiver: bool,
}

impl<T> State<T> {
    fn push_back(&mut self, value: T) {
        let node = Box::into_raw(Box::new(Node { value, next: core::ptr::null_mut() }));
        if self.tail.is_null() {
            self.head = node;
        } else {
            unsafe { (*self.tail).next = node };
        }
        self.tail = node;
        self.len += 1;
    }

    fn pop_front(&mut self) -> Option<T> {
        let node = self.head;
        if node.is_null() {
            return None;
        }
        let boxed = unsafe { Box::from_raw(node) };
        self.head = boxed.next;
        if self.head.is_null() {
            self.tail = core::ptr::null_mut();
        }
        self.len -= 1;
        Some(boxed.value)
    }
}

/// A bounded channel holding at most `capacity` items.
///
/// `capacity` is a buffer, not a rendezvous: a send returns as soon as the item is queued, and
/// only blocks once `capacity` items are waiting. That is `std::sync::mpsc::sync_channel`'s
/// meaning of the word, and the caller relies on it - with a rendezvous, the client's `send`
/// would not return until the server had already come back round to `recv`.
pub fn sync_channel<T>(capacity: usize) -> (SyncSender<T>, Receiver<T>) {
    let shared = Box::into_raw(Box::new(Shared {
        state: Mutex::new(State {
            head: core::ptr::null_mut(),
            tail: core::ptr::null_mut(),
            len: 0,
            senders: 1,
            receiver: true,
        }),
        on_send: Condvar::new(),
        on_recv: Condvar::new(),
        capacity,
    }));
    (SyncSender { shared }, Receiver { shared })
}

/// The value a send could not deliver, handed back rather than dropped.
#[derive(Debug, PartialEq, Eq)]
pub struct SendError<T>(pub T);

/// Returned by `recv` when every sender is gone and nothing is left queued.
#[derive(Debug, PartialEq, Eq)]
pub struct RecvError;

/// The sending half. Cloneable: many producers, one consumer.
pub struct SyncSender<T> {
    shared: *mut Shared<T>,
}

// The channel is the synchronisation, so the ends travel wherever `T` may.
unsafe impl<T: Send> Send for SyncSender<T> {}
unsafe impl<T: Send> Sync for SyncSender<T> {}

impl<T> Clone for SyncSender<T> {
    fn clone(&self) -> SyncSender<T> {
        let shared = unsafe { &*self.shared };
        shared.state.lock().senders += 1;
        SyncSender { shared: self.shared }
    }
}

impl<T> SyncSender<T> {
    /// Queue a value, blocking while the channel is full.
    ///
    /// `Err` means the receiver is gone, and carries the value back so a caller that wants to
    /// retry elsewhere still has it.
    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        let shared = unsafe { &*self.shared };
        let mut state = shared.state.lock();
        loop {
            if !state.receiver {
                return Err(SendError(value));
            }
            if state.len < shared.capacity {
                state.push_back(value);
                break;
            }
            // Spurious wakes are permitted, which is why this is a loop and not an `if`.
            state = shared.on_recv.wait(state);
        }
        drop(state);
        shared.on_send.notify_one();
        Ok(())
    }
}

impl<T> Drop for SyncSender<T> {
    fn drop(&mut self) {
        let shared = unsafe { &*self.shared };
        let mut state = shared.state.lock();
        state.senders -= 1;
        let last = state.senders == 0;
        let dead = last && !state.receiver;
        drop(state);
        if last {
            // A receiver blocked in `recv` is waiting for exactly this news.
            shared.on_send.notify_all();
        }
        if dead {
            let boxed = unsafe { Box::from_raw(self.shared) };
            let Shared { state, on_send, on_recv, .. } = *boxed;
            drop(state);
            drop(on_send);
            drop(on_recv);
        }
    }
}

/// The receiving half. Not `Clone`: one consumer.
pub struct Receiver<T> {
    shared: *mut Shared<T>,
}

unsafe impl<T: Send> Send for Receiver<T> {}

impl<T> Receiver<T> {
    /// Take the next value, blocking until there is one.
    ///
    /// `Err(RecvError)` means every sender has been dropped *and* the queue is drained. Items
    /// already queued are delivered first, so a producer that sends and then exits does not lose
    /// its last message.
    pub fn recv(&self) -> Result<T, RecvError> {
        let shared = unsafe { &*self.shared };
        let mut state = shared.state.lock();
        let value = loop {
            if let Some(value) = state.pop_front() {
                break value;
            }
            if state.senders == 0 {
                return Err(RecvError);
            }
            state = shared.on_send.wait(state);
        };
        drop(state);
        shared.on_recv.notify_one();
        Ok(value)
    }

    /// Take a value if one is already queued, without blocking.
    ///
    /// `None` covers both "nothing yet" and "nobody left"; a caller that needs to tell those
    /// apart wants `recv`.
    pub fn try_recv(&self) -> Option<T> {
        let shared = unsafe { &*self.shared };
        let mut state = shared.state.lock();
        let value = state.pop_front();
        drop(state);
        if value.is_some() {
            shared.on_recv.notify_one();
        }
        value
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        let shared = unsafe { &*self.shared };
        let mut state = shared.state.lock();
        state.receiver = false;
        let dead = state.senders == 0;
        while state.pop_front().is_some() {}
        drop(state);
        // A sender blocked on a full queue would otherwise wait for a drain that cannot come.
        shared.on_recv.notify_all();
        if dead {
            let boxed = unsafe { Box::from_raw(self.shared) };
            let Shared { state, on_send, on_recv, .. } = *boxed;
            drop(state);
            drop(on_send);
            drop(on_recv);
        }
    }
}
