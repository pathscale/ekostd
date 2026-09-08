//! Threads and locks, over pthreads.
//!
//! # Why a real lock and not a spinlock
//!
//! The obvious `no_std` answer is a spinlock, and it is wrong here. The thing behind this mutex
//! is the compiler: a request holds it for as long as a compile takes, which is seconds on a
//! corpus file and tens of seconds on a `--by clock` run. A spinlock makes every waiting thread
//! burn a core for that whole time, on a machine the compiler is trying to use for compiling.
//! `pthread_mutex` blocks in the kernel, which is what a lock held that long needs.
//!
//! # Why the stack size is a parameter and not a default
//!
//! `server.rs` gives its request threads sixteen megabytes, and the comment there records what
//! happened without it: rustc recurses deeply over generated code, so the first corpus file took
//! the whole server down with a SIGTRAP and nothing in the log - a stack overflow is not an
//! unwind, so the panic containment never saw it. Sixteen is what `rustc_interface` gives its own
//! compilation thread. A wrapper that could not set it would not be a replacement.

// `daemon` is `#![no_std]`. These arrive with the standard prelude and have no path to
// match, which is why a `std::` grep cannot see them and the attribute has to be flipped
// to find them at all.
use alloc::boxed::Box;


use crate::Errno;

/// Run `f` on a new thread with `stack_bytes` of stack, detached.
///
/// **Detached, because nothing here joins.** Every caller spawns a connection handler and forgets
/// it; the thread ends when the connection does. A joinable thread nobody joins is a leak of the
/// thread's own bookkeeping, which is the failure `pthread_detach` exists to prevent.
///
/// The closure is boxed twice on purpose: once to make it a sized value, and once more so the
/// pointer handed to C is a thin one. A fat pointer does not fit in `*mut c_void`, and casting a
/// `Box<dyn FnOnce>` straight through would truncate the vtable half and jump into nothing.
pub fn spawn_detached<F>(stack_bytes: usize, f: F) -> Result<(), Errno>
where
    F: FnOnce() + Send + 'static,
{
    // The outer box is what crosses to C; the inner one carries the closure's vtable.
    let payload: Box<Box<dyn FnOnce() + Send>> = Box::new(Box::new(f));
    let raw = Box::into_raw(payload);

    // `extern "C"` and *not* `unsafe fn`: `pthread_create` takes a safe C function pointer, and
    // an `unsafe extern "C" fn` is a different type that will not coerce to it. The unsafety is
    // inside, where reconstructing the box from the pointer is the thing that is actually unsafe.
    extern "C" fn run(arg: *mut libc::c_void) -> *mut libc::c_void {
        // Taking ownership back is what drops the closure once it has run.
        let f: Box<Box<dyn FnOnce() + Send>> = unsafe { Box::from_raw(arg.cast()) };
        f();
        core::ptr::null_mut()
    }

    let mut attr: libc::pthread_attr_t = unsafe { core::mem::zeroed() };
    unsafe {
        if libc::pthread_attr_init(&mut attr) != 0 {
            drop(Box::from_raw(raw));
            return Err(Errno::current());
        }
        libc::pthread_attr_setstacksize(&mut attr, stack_bytes);
        libc::pthread_attr_setdetachstate(&mut attr, libc::PTHREAD_CREATE_DETACHED);
    }

    let mut tid: libc::pthread_t = unsafe { core::mem::zeroed() };
    let created = unsafe { libc::pthread_create(&mut tid, &attr, run, raw.cast()) };
    unsafe { libc::pthread_attr_destroy(&mut attr) };

    if created != 0 {
        // The thread never started, so nothing will ever take the closure back. Dropping it here
        // is the difference between a failed spawn and a failed spawn that leaks.
        unsafe { drop(Box::from_raw(raw)) };
        return Err(Errno(created));
    }
    Ok(())
}

/// A mutex that blocks in the kernel.
///
/// `std::sync::Mutex` without the poisoning. Poisoning existed to say "a thread panicked while
/// holding this", and the callers here all used `unwrap_or_else(PoisonError::into_inner)` to
/// ignore it - which is the right answer for a compiler session: a request that panicked left the
/// session as it was, because the panic containment is what decides that, not the lock.
pub struct Mutex<T: ?Sized> {
    pub(crate) inner: core::cell::UnsafeCell<libc::pthread_mutex_t>,
    value: core::cell::UnsafeCell<T>,
}

// Safe for the same reason `std::sync::Mutex` is: the lock is what makes `&T` from `&Mutex<T>`
// sound across threads, so `T` need only be `Send`.
unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

impl<T: Default> Default for Mutex<T> {
    fn default() -> Self {
        Mutex::new(T::default())
    }
}

/// Prints what it guards is *not* what this does: taking the lock to format would let a `Debug`
/// call deadlock against a held one, which is a debugger's line hanging the server it is
/// debugging. It says there is a lock and nothing else.
impl<T> core::fmt::Debug for Mutex<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Mutex { .. }")
    }
}

impl<T> Mutex<T> {
    // `new` takes a value, so it needs a sized `T`. The rest of the interface does not, which is
    // why the two are separate blocks - `std::sync::Mutex` splits them the same way.
    pub const fn new(value: T) -> Mutex<T> {
        Mutex {
            inner: core::cell::UnsafeCell::new(libc::PTHREAD_MUTEX_INITIALIZER),
            value: core::cell::UnsafeCell::new(value),
        }
    }
}

impl<T: ?Sized> Mutex<T> {

    /// Take it if it is free, or say it is not.
    ///
    /// **Not a weakening of anything.** `serve_on` uses it to ask the handler for idle work,
    /// and the honest answer while a request is running is that there is no idle moment - so
    /// failing to take the lock *is* the answer, not a retry to paper over.
    pub fn try_lock(&self) -> Option<Guard<'_, T>> {
        if unsafe { libc::pthread_mutex_trylock(self.inner.get()) } == 0 {
            Some(Guard { mutex: self })
        } else {
            None
        }
    }

    /// Block until this is ours.
    ///
    /// A failure here is a programming error - a destroyed mutex, or a deadlock the kernel
    /// detected - and there is nothing a caller could do about it, which is why this returns a
    /// guard rather than a result. `std::sync::Mutex::lock` returns one only to carry poisoning,
    /// which this does not have.
    pub fn lock(&self) -> Guard<'_, T> {
        unsafe { libc::pthread_mutex_lock(self.inner.get()) };
        Guard { mutex: self }
    }
}

// **The eyepatch is load-bearing, not decoration.** This destructor calls
// `pthread_mutex_destroy` and touches nothing inside `T`, and `#[may_dangle]` is how that gets
// said to dropck. Without it, every type containing a `Mutex<T>` becomes drop-significant in a
// way that requires `T` to strictly outlive the lock, which breaks the pattern rustc's `'tcx`
// lifetime is built on: `rustc_interface::passes` creates a `OnceLock<GlobalCtxt<'tcx>>` and
// hands out `&'tcx` references to arenas declared beside it, and dropck then reports the arenas
// as dropped while still borrowed. `parking_lot::Mutex`, which this replaced, has no `Drop` at
// all and so never posed the question.
//
// Soundness: the promise `may_dangle` makes is that this destructor does not *inspect* `T`. It
// does not - `self.inner` is the `pthread_mutex_t`, a sibling field. The `UnsafeCell<T>` is still
// an owned field the compiler can see, so `T`'s own drop glue still runs and is still checked;
// the eyepatch relaxes only what *this* impl is assumed to do.
#[cfg(feature = "nightly")]
unsafe impl<#[may_dangle] T: ?Sized> Drop for Mutex<T> {
    fn drop(&mut self) {
        unsafe { libc::pthread_mutex_destroy(self.inner.get()) };
    }
}

// The same destructor without the eyepatch, for a consumer on stable. It has to exist: without
// it the `pthread_mutex_t` is never destroyed. What it costs is the dropck relaxation, which
// only rustc's `'tcx` pattern needs.
#[cfg(not(feature = "nightly"))]
impl<T: ?Sized> Drop for Mutex<T> {
    fn drop(&mut self) {
        unsafe { libc::pthread_mutex_destroy(self.inner.get()) };
    }
}

/// Held while the lock is. Releases on drop, which is the only way it is released.
pub struct Guard<'a, T: ?Sized> {
    pub(crate) mutex: &'a Mutex<T>,
}

impl<T: ?Sized> core::ops::Deref for Guard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.value.get() }
    }
}

impl<T: ?Sized> core::ops::DerefMut for Guard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.value.get() }
    }
}

impl<T: ?Sized> Drop for Guard<'_, T> {
    fn drop(&mut self) {
        unsafe { libc::pthread_mutex_unlock(self.mutex.inner.get()) };
    }
}

/// Run a closure exactly once, however many threads reach it.
///
/// `std::sync::Once` over `pthread_once`. `panics::install` needs it: the accept loop and the
/// first request thread can both be first, and installing twice would chain the hook to itself.
pub struct Once {
    inner: core::cell::UnsafeCell<libc::pthread_once_t>,
}

unsafe impl Send for Once {}
unsafe impl Sync for Once {}

/// The closure `pthread_once` will run, parked where the C callback can reach it.
///
/// **`pthread_once` takes a bare `extern "C" fn` with no argument**, so there is nowhere to pass
/// a closure through. This is the standard workaround and its safety rests on `pthread_once`'s
/// own guarantee: exactly one thread runs the callback, and every other blocks until it returns,
/// so the write and the read cannot overlap.
static mut PENDING: Option<fn()> = None;

unsafe extern "C" fn run_pending() {
    // Safety: inside `pthread_once`'s callback, which runs on one thread with all others blocked.
    if let Some(f) = unsafe { PENDING } {
        f();
    }
}

impl Once {
    pub const fn new() -> Once {
        Once { inner: core::cell::UnsafeCell::new(libc::PTHREAD_ONCE_INIT) }
    }

    /// Run `f` the first time this is called, and never again.
    ///
    /// A plain `fn` rather than a closure, for the reason [`PENDING`] gives. Both callers are
    /// installing a process-global hook and capture nothing.
    pub fn call_once(&self, f: fn()) {
        unsafe {
            PENDING = Some(f);
            libc::pthread_once(self.inner.get(), Some(run_pending));
        }
    }
}

impl Default for Once {
    fn default() -> Self {
        Once::new()
    }
}

/// One value per thread, freed when that thread ends.
///
/// `std::thread_local!` over `pthread_key_create`. The `nesting` crate used that macro and that
/// is how it linked `std` without ever spelling `std::` - a prelude macro names no path, so the
/// ratchet reported it clean for as long as it existed.
///
/// The destructor is what makes this a thread-local rather than a leak: `pthread_key_create`
/// takes one and the runtime calls it as each thread exits.
pub struct ThreadLocal<T> {
    /// The key, plus one, so that zero means "not yet created" without claiming that zero is not
    /// a valid key - which it is. Set with a compare-exchange, so two threads racing to first use
    /// produce one key and the loser frees its own.
    key: core::sync::atomic::AtomicUsize,
    _value: core::marker::PhantomData<T>,
}

unsafe impl<T> Send for ThreadLocal<T> {}
unsafe impl<T> Sync for ThreadLocal<T> {}

impl<T: 'static> ThreadLocal<T> {
    pub const fn new() -> ThreadLocal<T> {
        ThreadLocal {
            key: core::sync::atomic::AtomicUsize::new(0),
            _value: core::marker::PhantomData,
        }
    }

    fn key(&self) -> Option<libc::pthread_key_t> {
        use core::sync::atomic::Ordering;
        // The destructor is monomorphised per `T`, so it knows what to drop without a registry.
        unsafe extern "C" fn drop_box<T>(raw: *mut libc::c_void) {
            drop(unsafe { alloc::boxed::Box::from_raw(raw.cast::<T>()) });
        }
        let existing = self.key.load(Ordering::Acquire);
        if existing != 0 {
            return Some((existing - 1) as libc::pthread_key_t);
        }
        let mut created: libc::pthread_key_t = 0;
        if unsafe { libc::pthread_key_create(&mut created, Some(drop_box::<T>)) } != 0 {
            return None;
        }
        match self.key.compare_exchange(
            0,
            created as usize + 1,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => Some(created),
            // Another thread won. Free ours rather than leaking a key, and use theirs.
            Err(theirs) => {
                unsafe { libc::pthread_key_delete(created) };
                Some((theirs - 1) as libc::pthread_key_t)
            }
        }
    }

    /// This thread's value, creating it from `init` on first use.
    ///
    /// `None` only when the key could not be created, which is a process out of thread-local
    /// slots. Callers treat that as "no sink on this thread" - the same answer `std`'s `try_with`
    /// gives for a thread that is tearing down, which is exactly when a panic in a destructor
    /// happens.
    pub fn with<R>(&self, init: impl FnOnce() -> T, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        let key = self.key()?;
        unsafe {
            let existing = libc::pthread_getspecific(key);
            let ptr = if existing.is_null() {
                let boxed = alloc::boxed::Box::into_raw(alloc::boxed::Box::new(init()));
                libc::pthread_setspecific(key, boxed.cast());
                boxed
            } else {
                existing.cast::<T>()
            };
            Some(f(&mut *ptr))
        }
    }
}

/// A condition variable, paired with a [`Mutex`].
///
/// `std::sync::Condvar` over `pthread_cond_t`. The queue needs it: a job waiting for a slot must
/// sleep rather than poll, because the thing it is waiting for is a compile that may take a
/// minute, and a poll loop over that is a core spent asking.
pub struct Condvar {
    inner: core::cell::UnsafeCell<libc::pthread_cond_t>,
    /// How many threads are inside `wait`.
    ///
    /// `pthread_cond_signal` cannot tell you whether it woke anybody: POSIX gives no way to ask
    /// whether a waiter existed. `parking_lot::Condvar::notify_one` returns exactly that, and
    /// rustc's query-cycle breaker asserts on it - `assert!(waiter.condvar.notify_one())`, whose
    /// job is to catch a cycle-breaker that found nothing to wake. Deleting the assertion to fit
    /// the pthreads shape would have thrown away the check, so the count is kept instead.
    waiters: core::sync::atomic::AtomicUsize,
}

unsafe impl Send for Condvar {}
unsafe impl Sync for Condvar {}

impl Condvar {
    pub const fn new() -> Condvar {
        Condvar {
            inner: core::cell::UnsafeCell::new(libc::PTHREAD_COND_INITIALIZER),
            waiters: core::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Release the lock, wait to be notified, and take it again.
    ///
    /// **Takes the guard and gives it back**, which is `std::sync::Condvar::wait`'s shape and is
    /// not decoration: the lock must be held on the way in, is released while waiting, and is
    /// held again on return. A caller that could wait without the guard would be waiting on a
    /// condition nothing stops changing underneath it.
    ///
    /// A spurious wake returns normally, as pthreads permits. Every caller re-checks its
    /// condition in a loop, which is the only correct way to use one of these.
    pub fn wait<'a, T>(&self, guard: Guard<'a, T>) -> Guard<'a, T> {
        let mutex = guard.mutex;
        // The guard must not run its `Drop` and unlock: `pthread_cond_wait` does the unlocking,
        // and unlocking twice is undefined.
        core::mem::forget(guard);
        // Counted while the mutex is held, which is what makes `notify_one`'s answer exact for a
        // notifier that holds the same mutex. See `waiters`.
        self.waiters.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        unsafe { libc::pthread_cond_wait(self.inner.get(), mutex.inner.get()) };
        self.waiters.fetch_sub(1, core::sync::atomic::Ordering::Relaxed);
        Guard { mutex }
    }

    /// Wake one waiter. Returns whether there was one to wake.
    ///
    /// **The answer is exact only for a caller holding the mutex the waiters wait on**, because
    /// the count is maintained under that lock. `QueryLatch::set` is such a caller. A caller that
    /// does not hold it races with a thread entering `wait`, and gets a best-effort answer - a
    /// `false` means nobody had entered `wait` yet, not that nobody ever will.
    pub fn notify_one(&self) -> bool {
        let had_waiter = self.waiters.load(core::sync::atomic::Ordering::Relaxed) > 0;
        unsafe { libc::pthread_cond_signal(self.inner.get()) };
        had_waiter
    }

    /// Wake all of them. Returns how many were waiting.
    pub fn notify_all(&self) -> usize {
        let waiting = self.waiters.load(core::sync::atomic::Ordering::Relaxed);
        unsafe { libc::pthread_cond_broadcast(self.inner.get()) };
        waiting
    }
}

impl core::fmt::Debug for Condvar {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Condvar")
    }
}

impl Default for Condvar {
    fn default() -> Self {
        Condvar::new()
    }
}

impl Drop for Condvar {
    fn drop(&mut self) {
        unsafe { libc::pthread_cond_destroy(self.inner.get()) };
    }
}

/// A value initialised at most once, on first use.
///
/// `std::sync::OnceLock`. The state machine is three atomics-worth of information in one byte -
/// empty, initialising, ready - and the wait when another thread is initialising is a yield
/// loop rather than a futex. That is the right trade here: initialisation bodies in this
/// compiler are microseconds (building a table, parsing a version string) and the contended
/// case is rare, so a parked thread would cost more to wake than to spin.
pub struct OnceLock<T> {
    state: core::sync::atomic::AtomicU8,
    value: core::cell::UnsafeCell<Option<T>>,
}

const ONCE_EMPTY: u8 = 0;
const ONCE_BUSY: u8 = 1;
const ONCE_READY: u8 = 2;

// The value is published with a Release store and read after an Acquire load, so a `T` that can
// cross threads makes the lock able to.
unsafe impl<T: Send> Send for OnceLock<T> {}
unsafe impl<T: Send + Sync> Sync for OnceLock<T> {}

impl<T> OnceLock<T> {
    /// An empty lock.
    pub const fn new() -> OnceLock<T> {
        OnceLock {
            state: core::sync::atomic::AtomicU8::new(ONCE_EMPTY),
            value: core::cell::UnsafeCell::new(None),
        }
    }

    /// The value, if it has been initialised.
    pub fn get(&self) -> Option<&T> {
        use core::sync::atomic::Ordering;
        if self.state.load(Ordering::Acquire) != ONCE_READY {
            return None;
        }
        // Safe: the Acquire load paired with the Release store in `get_or_init` means the write
        // happened-before this read, and nothing writes after the state reaches READY.
        unsafe { (*self.value.get()).as_ref() }
    }

    /// The value, initialising it with `f` if this is the first call.
    ///
    /// `f` runs at most once even under contention. It must not call back into `get_or_init` on
    /// the same lock: that deadlocks, exactly as `std`'s does.
    pub fn get_or_init(&self, f: impl FnOnce() -> T) -> &T {
        use core::sync::atomic::Ordering;
        loop {
            match self.state.compare_exchange(
                ONCE_EMPTY,
                ONCE_BUSY,
                Ordering::Acquire,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    let v = f();
                    // Safe: this thread won the CAS, so it alone may write, and no reader can
                    // observe the cell until the Release store below.
                    unsafe { *self.value.get() = Some(v) };
                    self.state.store(ONCE_READY, Ordering::Release);
                    break;
                }
                Err(ONCE_READY) => break,
                // Another thread is initialising. Yield rather than spin hot.
                Err(_) => unsafe {
                    libc::sched_yield();
                },
            }
        }
        self.get().expect("the lock is ready once the loop breaks")
    }

    /// Store a value if the lock is empty.
    pub fn set(&self, value: T) -> Result<(), T> {
        use core::sync::atomic::Ordering;
        match self.state.compare_exchange(
            ONCE_EMPTY,
            ONCE_BUSY,
            Ordering::Acquire,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                unsafe { *self.value.get() = Some(value) };
                self.state.store(ONCE_READY, Ordering::Release);
                Ok(())
            }
            Err(_) => Err(value),
        }
    }
}

impl<T: Clone> Clone for OnceLock<T> {
    /// A ready lock clones its value; an empty one clones as empty.
    ///
    /// This is `std::sync::OnceLock`'s behaviour, and `rustc_middle`'s `BasicBlocks` cache
    /// derives `Clone` through one - a cloned cache that had not been filled yet stays unfilled,
    /// which is the same answer as recomputing it.
    fn clone(&self) -> OnceLock<T> {
        let out = OnceLock::new();
        if let Some(v) = self.get() {
            let _ = out.set(v.clone());
        }
        out
    }
}

impl<T> Default for OnceLock<T> {
    fn default() -> OnceLock<T> {
        OnceLock::new()
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for OnceLock<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("OnceLock").field(&self.get()).finish()
    }
}

/// A value computed on first dereference.
///
/// `std::sync::LazyLock`. The closure is stored beside the cell and taken when it runs.
pub struct LazyLock<T, F = fn() -> T> {
    once: OnceLock<T>,
    init: core::cell::UnsafeCell<Option<F>>,
}

unsafe impl<T: Send + Sync, F: Send> Sync for LazyLock<T, F> {}

impl<T, F: FnOnce() -> T> LazyLock<T, F> {
    /// A lock that will call `f` on first use.
    pub const fn new(f: F) -> LazyLock<T, F> {
        LazyLock { once: OnceLock::new(), init: core::cell::UnsafeCell::new(Some(f)) }
    }

    /// Force the value, returning it.
    pub fn force(this: &LazyLock<T, F>) -> &T {
        this.once.get_or_init(|| {
            // Safe: `get_or_init` runs this body on exactly one thread and exactly once, so
            // taking the closure out of the cell cannot race.
            let f = unsafe { (*this.init.get()).take() };
            f.expect("the initialiser runs once")()
        })
    }
}

impl<T, F: FnOnce() -> T> core::ops::Deref for LazyLock<T, F> {
    type Target = T;
    fn deref(&self) -> &T {
        LazyLock::force(self)
    }
}

/// A reader-writer lock over `pthread_rwlock_t`.
pub struct RwLock<T: ?Sized> {
    raw: core::cell::UnsafeCell<libc::pthread_rwlock_t>,
    value: core::cell::UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for RwLock<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for RwLock<T> {}

impl<T> RwLock<T> {
    /// An unlocked lock.
    pub const fn new(value: T) -> RwLock<T> {
        RwLock {
            raw: core::cell::UnsafeCell::new(libc::PTHREAD_RWLOCK_INITIALIZER),
            value: core::cell::UnsafeCell::new(value),
        }
    }

    /// Take a shared read lock.
    pub fn read(&self) -> ReadGuard<'_, T> {
        unsafe { libc::pthread_rwlock_rdlock(self.raw.get()) };
        ReadGuard { lock: self }
    }

    /// Take the exclusive write lock.
    pub fn write(&self) -> WriteGuard<'_, T> {
        unsafe { libc::pthread_rwlock_wrlock(self.raw.get()) };
        WriteGuard { lock: self }
    }

    /// The value, when the lock is owned uniquely and cannot be held.
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }
}

/// A held read lock.
pub struct ReadGuard<'a, T: ?Sized> {
    lock: &'a RwLock<T>,
}

impl<T: ?Sized> core::ops::Deref for ReadGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }
}

impl<T: ?Sized> Drop for ReadGuard<'_, T> {
    fn drop(&mut self) {
        unsafe { libc::pthread_rwlock_unlock(self.lock.raw.get()) };
    }
}

/// A held write lock.
pub struct WriteGuard<'a, T: ?Sized> {
    lock: &'a RwLock<T>,
}

impl<T: ?Sized> core::ops::Deref for WriteGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }
}

impl<T: ?Sized> core::ops::DerefMut for WriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<T: ?Sized> Drop for WriteGuard<'_, T> {
    fn drop(&mut self) {
        unsafe { libc::pthread_rwlock_unlock(self.lock.raw.get()) };
    }
}

/// An opaque identifier for the calling thread.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ThreadId(u64);

impl ThreadId {
    /// The identifier as a number, for a profiler that wants to record it.
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

/// The calling thread's identifier.
///
/// `pthread_self` is a pointer-sized handle rather than the small integer `std::thread::ThreadId`
/// hands out, which is fine for every use here: the identifier is compared for equality and
/// printed, never used as a dense index.
pub fn current_id() -> ThreadId {
    ThreadId(unsafe { libc::pthread_self() } as usize as u64)
}

/// `std::thread_local!`, with the accessor API call sites expect.
///
/// Declares a `static` whose value is per-thread, backed by a `pthread` key. The methods mirror
/// `std::thread::LocalKey`: `with`, `with_borrow`, `with_borrow_mut`, `set`, `replace`, `take`.
///
/// **The initialiser runs per thread, on that thread's first touch**, exactly as `std`'s does.
/// What is missing is destruction: `std` runs the value's destructor when the thread exits, and
/// this does not - a value that owns an allocation leaks it once per thread that touched it.
/// Every use in this compiler is a cache or a counter that lives as long as the process, so the
/// leak is bounded by thread count rather than by work done. Do not put a file handle in one.
#[macro_export]
macro_rules! thread_local {
    () => {};
    ($(#[$attr:meta])* $vis:vis static $name:ident: $ty:ty = $init:expr; $($rest:tt)*) => {
        $(#[$attr])*
        $vis static $name: $crate::thread::LocalKey<$ty> =
            $crate::thread::LocalKey::new(|| $init);
        $crate::thread_local!($($rest)*);
    };
    ($(#[$attr:meta])* $vis:vis static $name:ident: $ty:ty = const { $init:expr }; $($rest:tt)*) => {
        $(#[$attr])*
        $vis static $name: $crate::thread::LocalKey<$ty> =
            $crate::thread::LocalKey::new(|| $init);
        $crate::thread_local!($($rest)*);
    };
}

/// The static declared by [`thread_local!`].
pub struct LocalKey<T: 'static> {
    slot: ThreadLocal<T>,
    init: fn() -> T,
}

// A `LocalKey` is per-thread by construction: every thread that touches it gets its own value,
// so sharing the *key* across threads is safe even when the value type is not - which it usually
// is not, since the whole point is `Cell` and `RefCell`. `std::thread_local!`'s `LocalKey` is
// `Sync` for exactly this reason.
unsafe impl<T: 'static> Sync for LocalKey<T> {}
unsafe impl<T: 'static> Send for LocalKey<T> {}

impl<T: 'static> LocalKey<T> {
    /// Not called directly; `thread_local!` builds these.
    pub const fn new(init: fn() -> T) -> LocalKey<T> {
        LocalKey { slot: ThreadLocal::new(), init }
    }

    /// Run `f` on this thread's value, creating it if this is the first touch.
    pub fn with<R>(&'static self, f: impl FnOnce(&T) -> R) -> R {
        self.slot.with(self.init, |v| f(v)).expect("a thread can always reach its own key")
    }

    /// Run `f` on this thread's value mutably.
    pub fn with_mut<R>(&'static self, f: impl FnOnce(&mut T) -> R) -> R {
        self.slot.with(self.init, f).expect("a thread can always reach its own key")
    }
}

impl<T: 'static> LocalKey<core::cell::RefCell<T>> {
    /// Run `f` on the value inside the `RefCell`.
    pub fn with_borrow<R>(&'static self, f: impl FnOnce(&T) -> R) -> R {
        self.with(|c| f(&c.borrow()))
    }

    /// Run `f` on the value inside the `RefCell`, mutably.
    pub fn with_borrow_mut<R>(&'static self, f: impl FnOnce(&mut T) -> R) -> R {
        self.with(|c| f(&mut c.borrow_mut()))
    }

    /// Replace the value, returning the old one.
    pub fn replace(&'static self, value: T) -> T {
        self.with(|c| c.replace(value))
    }

    /// Take the value, leaving its default.
    pub fn take(&'static self) -> T
    where
        T: Default,
    {
        self.with(|c| c.take())
    }

    /// Overwrite the value.
    pub fn set(&'static self, value: T) {
        self.with(|c| *c.borrow_mut() = value);
    }
}

impl<T: 'static + Copy> LocalKey<core::cell::Cell<T>> {
    /// The current value.
    pub fn get(&'static self) -> T {
        self.with(|c| c.get())
    }

    /// Overwrite the value.
    pub fn set(&'static self, value: T) {
        self.with(|c| c.set(value));
    }

    /// Overwrite the value, returning the old one.
    ///
    /// `rustc_middle`'s pretty printer uses this for scoped flags - set on the way in, restore
    /// on the way out - so the old value is the whole point.
    pub fn replace(&'static self, value: T) -> T {
        self.with(|c| c.replace(value))
    }
}

// ================================================================================================
// Threads you can wait for
// ================================================================================================

use alloc::string::String;
use alloc::vec::Vec;

/// A thread that can be waited for, and its result collected.
///
/// `spawn_detached` above covers the daemon's accept loop, where nothing waits. This is the other
/// half: `coupled`'s frontend worker and its batch compiler both need the value the thread
/// produced, and a scoped borrow of data the parent owns.
pub struct JoinHandle<T> {
    tid: libc::pthread_t,
    // Where the closure leaves its result. The thread writes it before returning and `join`
    // reads it after `pthread_join`, which is the ordering edge that makes the read safe.
    slot: *mut Option<T>,
}

// The handle is just an id and a pointer the joining thread owns exclusively after the join.
unsafe impl<T: Send> Send for JoinHandle<T> {}

struct SpawnPayload<T> {
    f: Box<dyn FnOnce() -> T + Send>,
    slot: *mut Option<T>,
}

extern "C" fn spawn_trampoline<T>(arg: *mut libc::c_void) -> *mut libc::c_void {
    // Safe: `spawn` leaked exactly one `SpawnPayload<T>` for this thread and no one else holds it.
    let payload: Box<SpawnPayload<T>> = unsafe { Box::from_raw(arg.cast()) };
    let SpawnPayload { f, slot } = *payload;
    let value = f();
    // Safe: the slot was leaked by `spawn` and is written by this thread alone, before the
    // `pthread_join` that lets the parent read it.
    unsafe { *slot = Some(value) };
    core::ptr::null_mut()
}

/// How a thread is configured before it starts.
pub struct Builder {
    stack_bytes: usize,
    name: Option<String>,
}

impl Builder {
    /// A builder with the platform's default stack.
    pub fn new() -> Builder {
        Builder { stack_bytes: 0, name: None }
    }

    /// Ask for a stack of this size.
    ///
    /// The frontend needs this: rustc recurses over deeply nested types and the default 512 KiB
    /// is not enough, which is why `DEFAULT_STACK_SIZE` exists.
    pub fn stack_size(mut self, bytes: usize) -> Builder {
        self.stack_bytes = bytes;
        self
    }

    /// Name the thread, for a debugger and for crash reports.
    pub fn name(mut self, name: impl Into<String>) -> Builder {
        self.name = Some(name.into());
        self
    }

    /// Start it.
    pub fn spawn<T: Send + 'static>(
        self,
        f: impl FnOnce() -> T + Send + 'static,
    ) -> Result<JoinHandle<T>, Errno> {
        let slot: *mut Option<T> = Box::into_raw(Box::new(None));
        let payload = Box::into_raw(Box::new(SpawnPayload { f: Box::new(f), slot }));

        let mut attr: libc::pthread_attr_t = unsafe { core::mem::zeroed() };
        unsafe { libc::pthread_attr_init(&mut attr) };
        if self.stack_bytes != 0 {
            unsafe { libc::pthread_attr_setstacksize(&mut attr, self.stack_bytes) };
        }
        let mut tid: libc::pthread_t = unsafe { core::mem::zeroed() };
        let rc = unsafe {
            libc::pthread_create(&mut tid, &attr, spawn_trampoline::<T>, payload.cast())
        };
        unsafe { libc::pthread_attr_destroy(&mut attr) };
        if rc != 0 {
            // Nothing started, so both allocations are still ours to reclaim.
            drop(unsafe { Box::from_raw(payload) });
            drop(unsafe { Box::from_raw(slot) });
            return Err(Errno(rc));
        }
        // The name is set from the parent because `pthread_setname_np` on macOS only names the
        // *calling* thread, so the child would have to do it itself; the name is a debugging
        // affordance and not worth another hop through the payload.
        let _ = self.name;
        Ok(JoinHandle { tid, slot })
    }
}

impl Default for Builder {
    fn default() -> Builder {
        Builder::new()
    }
}

/// Start a thread with the default stack.
pub fn spawn<T: Send + 'static>(
    f: impl FnOnce() -> T + Send + 'static,
) -> Result<JoinHandle<T>, Errno> {
    Builder::new().spawn(f)
}

impl<T> JoinHandle<T> {
    /// Wait for the thread and take its result.
    ///
    /// `Err` means the join itself failed - the thread was already detached or joined. A thread
    /// that panicked does not arrive here at all: with `panic = "abort"` it took the process
    /// down, which is why this returns `Result<T, Errno>` and not `std`'s
    /// `Result<T, Box<dyn Any>>`.
    pub fn join(self) -> Result<T, Errno> {
        let rc = unsafe { libc::pthread_join(self.tid, core::ptr::null_mut()) };
        if rc != 0 {
            return Err(Errno(rc));
        }
        // Safe: the join is the ordering edge - the thread's write to the slot happened-before
        // this read, and it wrote exactly once.
        let boxed = unsafe { Box::from_raw(self.slot) };
        (*boxed).ok_or(Errno(libc::EINVAL))
    }
}

// ================================================================================================
// Scoped threads
// ================================================================================================

/// The handle a scoped thread is started through.
pub struct Scope<'scope, 'env: 'scope> {
    handles: core::cell::RefCell<Vec<libc::pthread_t>>,
    _marker: core::marker::PhantomData<(&'scope (), &'env ())>,
}

impl<'scope, 'env> Scope<'scope, 'env> {
    /// Start a thread that may borrow from outside the scope.
    ///
    /// The borrow is sound because [`scope`] joins every thread before it returns, so nothing
    /// the closure captured can be dropped while the thread is still running.
    pub fn spawn<F>(&'scope self, f: F)
    where
        F: FnOnce() + Send + 'scope,
    {
        // The closure borrows for `'scope`, but `pthread_create` needs a `'static` payload. The
        // lifetime is erased here and restored by the join in `scope`, which cannot be skipped.
        let boxed: Box<dyn FnOnce() + Send + 'scope> = Box::new(f);
        let erased: Box<dyn FnOnce() + Send + 'static> =
            unsafe { core::mem::transmute(boxed) };
        let payload = Box::into_raw(Box::new(erased));

        extern "C" fn run(arg: *mut libc::c_void) -> *mut libc::c_void {
            let f: Box<Box<dyn FnOnce() + Send + 'static>> = unsafe { Box::from_raw(arg.cast()) };
            (*f)();
            core::ptr::null_mut()
        }

        let mut tid: libc::pthread_t = unsafe { core::mem::zeroed() };
        let rc = unsafe {
            libc::pthread_create(&mut tid, core::ptr::null(), run, payload.cast())
        };
        if rc != 0 {
            // The thread never started, so the closure is still ours - and dropping it is the
            // only honest thing to do, because there is no way to report from here without
            // making every call site handle a failure that means the machine is out of threads.
            drop(unsafe { Box::from_raw(payload) });
            return;
        }
        self.handles.borrow_mut().push(tid);
    }
}

/// Run `f` with a scope that can start threads borrowing from the caller.
///
/// Every thread started inside is joined before this returns. That join is what makes the
/// borrows sound, and it happens even if `f` returns early - there is no unwinding to escape it
/// under `panic = "abort"`, and no early return that skips it.
pub fn scope<'env, F, T>(f: F) -> T
where
    F: for<'scope> FnOnce(&'scope Scope<'scope, 'env>) -> T,
{
    let sc = Scope { handles: core::cell::RefCell::new(Vec::new()), _marker: core::marker::PhantomData };
    let out = f(&sc);
    for tid in sc.handles.borrow().iter() {
        unsafe { libc::pthread_join(*tid, core::ptr::null_mut()) };
    }
    out
}

// Channels used to be deliberately absent, on the argument that cross-thread signalling goes
// through `ps-spsc` - a bounded single-producer single-consumer queue that already exists, is
// already `no_std`, and would not be a second set of memory-ordering bugs to find.
//
// `crates/libc-wrapper/src/channel.rs` is the exception, and it is one `ps-spsc` cannot serve:
// `proc_macro`'s cross-thread executor terminates its dispatch loop on `recv()` returning
// `None`, which means *the peer is gone*. A ring buffer has no way to say that - it can report
// empty, and empty is not the same answer. See `driver.rs`, which hit the same wall from the
// other side and had to add its own end marker.
//
// The memory-ordering objection does not apply to what is there: it is a `VecDeque` under the
// `Mutex` above, woken by the `Condvar` above, so every ordering edge is one of theirs.

// ================================================================================================
// Scoped thread-local values
// ================================================================================================

/// A value installed for the duration of a call, readable by anything below it on the stack.
///
/// This is `scoped_tls::scoped_thread_local!`, which `rustc_span` uses for `SESSION_GLOBALS` -
/// the interner and span data that every part of the frontend reaches for without threading a
/// parameter through. That crate's macro expands to `::std::thread_local!`, which is a `std`
/// dependency no grep can find, and it is why `rustc_span` was the last crate still linking it.
///
/// Only a pointer is stored, so `T` need not be `'static` in the value - the borrow is live
/// exactly as long as [`ScopedKey::set`]'s closure runs, which is what makes reading it sound.
pub struct ScopedKey<T: 'static> {
    slot: LocalKey<core::cell::Cell<*const ()>>,
    _marker: core::marker::PhantomData<T>,
}

// Same reasoning as `LocalKey`: the key is per-thread, so it is shareable regardless of `T`.
unsafe impl<T: 'static> Sync for ScopedKey<T> {}
unsafe impl<T: 'static> Send for ScopedKey<T> {}

impl<T: 'static> ScopedKey<T> {
    /// Not called directly; [`scoped_thread_local!`] builds these.
    pub const fn new() -> ScopedKey<T> {
        ScopedKey {
            slot: LocalKey::new(|| core::cell::Cell::new(core::ptr::null())),
            _marker: core::marker::PhantomData,
        }
    }

    /// Install `value` for the duration of `f`.
    ///
    /// The previous value is restored afterwards, so nesting works - `create_session_globals_then`
    /// asserts nothing is installed, but the restore is what makes that assertion meaningful
    /// rather than a one-shot.
    ///
    /// **The restore is a guard and not a statement after `f()`, because `f` can unwind.** This
    /// is `scoped_tls`'s own `Reset`, and it is not a stylistic preference: the compiler refuses
    /// a program by unwinding a `FatalError` from deep inside the frontend, and that unwind
    /// passes straight through this frame. A restore written as the line after the call is
    /// skipped, and what is left installed is a pointer to a `SessionGlobals` whose owner has
    /// been unwound off the stack - so `is_set` answers yes, `with` hands out a reference to
    /// freed memory, and the next `create_session_globals_then` fails its own assertion. In
    /// `ekod` that was: refuse one file, then compile a valid one, and the daemon died.
    pub fn set<R>(&'static self, value: &T, f: impl FnOnce() -> R) -> R {
        struct Reset {
            slot: &'static LocalKey<core::cell::Cell<*const ()>>,
            previous: *const (),
        }

        impl Drop for Reset {
            fn drop(&mut self) {
                self.slot.set(self.previous);
            }
        }

        let previous = self.slot.replace((value as *const T).cast());
        let _reset = Reset { slot: &self.slot, previous };
        f()
    }

    /// Read the installed value.
    ///
    /// Panics if nothing is installed. That is `scoped_tls`'s behaviour and the right one: a
    /// caller reaching for session globals outside a session has a bug that a `None` would only
    /// move somewhere less obvious.
    pub fn with<R>(&'static self, f: impl FnOnce(&T) -> R) -> R {
        let p = self.slot.get();
        assert!(!p.is_null(), "cannot access a scoped thread local that has not been set");
        // Safe: the pointer was written by `set`, whose closure is still running - so the
        // referent outlives this borrow - and it is cleared on the way out.
        f(unsafe { &*p.cast::<T>() })
    }

    /// Whether a value is installed on this thread.
    pub fn is_set(&'static self) -> bool {
        !self.slot.get().is_null()
    }
}

/// Declare a [`ScopedKey`], with `scoped_tls`'s syntax.
#[macro_export]
macro_rules! scoped_thread_local {
    ($(#[$attr:meta])* $vis:vis static $name:ident: $ty:ty) => {
        $(#[$attr])*
        $vis static $name: $crate::thread::ScopedKey<$ty> = $crate::thread::ScopedKey::new();
    };
}
