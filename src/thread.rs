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

use crate::libc_lite as libc;
use crate::Errno;

/// Run `f` on a new thread with `stack_bytes` of stack, detached.
///
/// Detached execution owns and drops the concrete closure payload on the new pthread.
pub fn spawn_detached<F>(stack_bytes: usize, f: F) -> Result<(), Errno>
where
    F: FnOnce() + Send + 'static,
{
    let raw = Box::into_raw(Box::new(f));

    #[cfg(target_os = "none")]
    {
        match unsafe {
            ::libc_rs::thread::spawn_thread(
                stack_bytes,
                detached_trampoline_rs::<F>,
                raw.cast(),
                true,
            )
        } {
            Ok(_) => Ok(()),
            Err(e) => {
                unsafe { drop(Box::from_raw(raw)) };
                Err(Errno(e.0))
            }
        }
    }

    #[cfg(not(target_os = "none"))]
    {
        let mut attr: libc::pthread_attr_t = unsafe { core::mem::zeroed() };
        let init_rc = unsafe { libc::pthread_attr_init(&mut attr) };
        if init_rc != 0 {
            unsafe { drop(Box::from_raw(raw)) };
            return Err(Errno(init_rc));
        }
        let stack_rc = unsafe { libc::pthread_attr_setstacksize(&mut attr, stack_bytes) };
        if stack_rc != 0 {
            unsafe {
                libc::pthread_attr_destroy(&mut attr);
                drop(Box::from_raw(raw));
            }
            return Err(Errno(stack_rc));
        }
        let detach_rc =
            unsafe { libc::pthread_attr_setdetachstate(&mut attr, libc::PTHREAD_CREATE_DETACHED) };
        if detach_rc != 0 {
            unsafe {
                libc::pthread_attr_destroy(&mut attr);
                drop(Box::from_raw(raw));
            }
            return Err(Errno(detach_rc));
        }

        let mut tid: libc::pthread_t = unsafe { core::mem::zeroed() };
        let create_rc =
            unsafe { libc::pthread_create(&mut tid, &attr, detached_trampoline::<F>, raw.cast()) };
        unsafe { libc::pthread_attr_destroy(&mut attr) };

        if create_rc != 0 {
            // The thread never started, so nothing will ever take the closure back. Dropping it here
            // is the difference between a failed spawn and a failed spawn that leaks.
            unsafe { drop(Box::from_raw(raw)) };
            return Err(Errno(create_rc));
        }
        Ok(())
    }
}

#[cfg(target_os = "none")]
extern "C" fn detached_trampoline_rs<F: FnOnce() + Send + 'static>(arg: *mut u8) {
    let f: Box<F> = unsafe { Box::from_raw(arg.cast()) };
    (*f)();
}

#[cfg(not(target_os = "none"))]
extern "C" fn detached_trampoline<F: FnOnce() + Send + 'static>(
    arg: *mut libc::c_void,
) -> *mut libc::c_void {
    let f: Box<F> = unsafe { Box::from_raw(arg.cast()) };
    (*f)();
    core::ptr::null_mut()
}

#[cfg(not(target_os = "none"))]
pub(crate) struct RawMutex(core::sync::atomic::AtomicPtr<libc::pthread_mutex_t>);

#[cfg(not(target_os = "none"))]
impl RawMutex {
    const fn new() -> Self {
        Self(core::sync::atomic::AtomicPtr::new(core::ptr::null_mut()))
    }

    pub(crate) fn get(&self) -> *mut libc::pthread_mutex_t {
        let ptr = self.0.load(core::sync::atomic::Ordering::Acquire);
        if !ptr.is_null() {
            return ptr;
        }
        self.init_slow()
    }

    #[cold]
    fn init_slow(&self) -> *mut libc::pthread_mutex_t {
        let boxed = Box::into_raw(Box::new(libc::PTHREAD_MUTEX_INITIALIZER));
        match self.0.compare_exchange(
            core::ptr::null_mut(),
            boxed,
            core::sync::atomic::Ordering::Release,
            core::sync::atomic::Ordering::Acquire,
        ) {
            Ok(_) => boxed,
            Err(existing) => {
                unsafe { drop(Box::from_raw(boxed)) };
                existing
            }
        }
    }
}

#[cfg(not(target_os = "none"))]
impl Drop for RawMutex {
    fn drop(&mut self) {
        let ptr = *self.0.get_mut();
        if !ptr.is_null() {
            unsafe {
                libc::pthread_mutex_destroy(ptr);
                drop(Box::from_raw(ptr));
            }
        }
    }
}

/// A mutex that blocks in the kernel.
///
/// `std::sync::Mutex` without the poisoning. Poisoning existed to say "a thread panicked while
/// holding this", and the callers here all used `unwrap_or_else(PoisonError::into_inner)` to
/// ignore it - which is the right answer for a compiler session: a request that panicked left the
/// session as it was, because the panic containment is what decides that, not the lock.
pub struct Mutex<T: ?Sized> {
    #[cfg(target_os = "none")]
    pub(crate) inner: ::libc_rs::sync::FutexMutex<()>,
    #[cfg(not(target_os = "none"))]
    pub(crate) inner: RawMutex,
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
            #[cfg(target_os = "none")]
            inner: ::libc_rs::sync::FutexMutex::new(()),
            #[cfg(not(target_os = "none"))]
            inner: RawMutex::new(),
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
        #[cfg(target_os = "none")]
        {
            if let Some(g) = self.inner.try_lock() {
                core::mem::forget(g);
                Some(Guard { mutex: self })
            } else {
                None
            }
        }
        #[cfg(not(target_os = "none"))]
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
        #[cfg(target_os = "none")]
        {
            let g = self.inner.lock();
            core::mem::forget(g);
        }
        #[cfg(not(target_os = "none"))]
        unsafe {
            libc::pthread_mutex_lock(self.inner.get());
        }
        Guard { mutex: self }
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
        #[cfg(target_os = "none")]
        unsafe {
            self.mutex.inner.raw_unlock();
        }
        #[cfg(not(target_os = "none"))]
        unsafe {
            libc::pthread_mutex_unlock(self.mutex.inner.get());
        }
    }
}

/// Run a closure exactly once, however many threads reach it.
///
/// `std::sync::Once` over `pthread_once`. `panics::install` needs it: the accept loop and the
/// first request thread can both be first, and installing twice would chain the hook to itself.
pub struct Once {
    inner: OnceLock<()>,
}

impl Once {
    pub const fn new() -> Once {
        Once { inner: OnceLock::new() }
    }

    /// Run `f` the first time this is called, and never again.
    pub fn call_once(&self, f: impl FnOnce()) {
        self.inner.get_or_init(f);
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

impl<T: 'static> Default for ThreadLocal<T> {
    fn default() -> ThreadLocal<T> {
        ThreadLocal::new()
    }
}

impl<T: 'static> ThreadLocal<T> {
    pub const fn new() -> ThreadLocal<T> {
        ThreadLocal {
            key: core::sync::atomic::AtomicUsize::new(0),
            _value: core::marker::PhantomData,
        }
    }

    #[cfg(target_os = "none")]
    fn key(&self) -> Option<usize> {
        use core::sync::atomic::Ordering;
        unsafe extern "C" fn drop_box_rs<T>(raw: *mut u8) {
            if !raw.is_null() {
                unsafe {
                    let ptr = raw.cast::<T>();
                    if core::mem::needs_drop::<T>() {
                        core::ptr::drop_in_place(ptr);
                    }
                    let layout = core::alloc::Layout::new::<T>();
                    if layout.size() != 0 {
                        alloc::alloc::dealloc(ptr.cast::<u8>(), layout);
                    }
                }
            }
        }
        let existing = self.key.fetch_add(0, Ordering::Acquire);
        if existing != 0 {
            return Some(existing - 1);
        }
        let created = ::libc_rs::thread::tls_key_create(Some(drop_box_rs::<T>))?;
        match self.key.compare_exchange(0, created + 1, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => Some(created),
            Err(theirs) => {
                let _ = ::libc_rs::thread::tls_key_delete(created);
                Some(theirs - 1)
            }
        }
    }

    #[cfg(not(target_os = "none"))]
    fn key(&self) -> Option<libc::pthread_key_t> {
        use core::sync::atomic::Ordering;
        // The destructor is monomorphised per `T`, so it knows what to drop without a registry.
        unsafe extern "C" fn drop_box<T>(raw: *mut libc::c_void) {
            if !raw.is_null() {
                unsafe {
                    let ptr = raw.cast::<T>();
                    if core::mem::needs_drop::<T>() {
                        core::ptr::drop_in_place(ptr);
                    }
                    let layout = core::alloc::Layout::new::<T>();
                    if layout.size() != 0 {
                        alloc::alloc::dealloc(ptr.cast::<u8>(), layout);
                    }
                }
            }
        }
        let existing = self.key.fetch_add(0, Ordering::Acquire);
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

    unsafe fn alloc_tls_box(value: T) -> *mut T {
        let layout = core::alloc::Layout::new::<T>();
        if layout.size() == 0 {
            return core::ptr::NonNull::<T>::dangling().as_ptr();
        }
        let raw = unsafe { alloc::alloc::alloc(layout).cast::<T>() };
        if raw.is_null() {
            alloc::alloc::handle_alloc_error(layout);
        }
        unsafe { core::ptr::write(raw, value) };
        raw
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
            #[cfg(target_os = "none")]
            let ptr = {
                let existing = ::libc_rs::thread::tls_get(key);
                if existing.is_null() {
                    let boxed = Self::alloc_tls_box(init());
                    let _ = ::libc_rs::thread::tls_set(key, boxed.cast());
                    boxed
                } else {
                    existing.cast::<T>()
                }
            };
            #[cfg(not(target_os = "none"))]
            let ptr = {
                let existing = libc::pthread_getspecific(key);
                if existing.is_null() {
                    let boxed = Self::alloc_tls_box(init());
                    libc::pthread_setspecific(key, boxed.cast());
                    boxed
                } else {
                    existing.cast::<T>()
                }
            };
            Some(f(&mut *ptr))
        }
    }

    /// This thread's value by shared reference, creating it from `init` on first use.
    ///
    /// The form to use whenever `f` can reach the same thread-local again, directly or through
    /// anything it calls. [`ThreadLocal::with`] hands out `&mut T`; a second `with` on the same
    /// thread while the first is still running makes two exclusive references to one value. That
    /// is undefined behaviour, and it is not theoretical: the optimiser may treat a store made
    /// through the outer reference as unobservable by the calls inside it and remove the store.
    /// A value reached this way is changed through `Cell` or `RefCell`, as with `std`'s keys.
    pub fn with_ref<R>(&self, init: impl FnOnce() -> T, f: impl FnOnce(&T) -> R) -> Option<R> {
        let key = self.key()?;
        unsafe {
            #[cfg(target_os = "none")]
            let ptr = {
                let existing = ::libc_rs::thread::tls_get(key);
                if existing.is_null() {
                    let boxed = Self::alloc_tls_box(init());
                    let _ = ::libc_rs::thread::tls_set(key, boxed.cast());
                    boxed
                } else {
                    existing.cast::<T>()
                }
            };
            #[cfg(not(target_os = "none"))]
            let ptr = {
                let existing = libc::pthread_getspecific(key);
                if existing.is_null() {
                    let boxed = Self::alloc_tls_box(init());
                    libc::pthread_setspecific(key, boxed.cast());
                    boxed
                } else {
                    existing.cast::<T>()
                }
            };
            Some(f(&*ptr))
        }
    }
}

/// A condition variable, paired with a [`Mutex`].
///
/// `std::sync::Condvar` over `pthread_cond_t`. The queue needs it: a job waiting for a slot must
/// sleep rather than poll, because the thing it is waiting for is a compile that may take a
/// minute, and a poll loop over that is a core spent asking.
pub struct Condvar {
    #[cfg(target_os = "none")]
    seq: core::sync::atomic::AtomicU32,
    #[cfg(not(target_os = "none"))]
    inner: core::sync::atomic::AtomicPtr<libc::pthread_cond_t>,
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
            #[cfg(target_os = "none")]
            seq: core::sync::atomic::AtomicU32::new(0),
            #[cfg(not(target_os = "none"))]
            inner: core::sync::atomic::AtomicPtr::new(core::ptr::null_mut()),
            waiters: core::sync::atomic::AtomicUsize::new(0),
        }
    }

    #[cfg(not(target_os = "none"))]
    fn raw(&self) -> *mut libc::pthread_cond_t {
        let ptr = self.inner.load(core::sync::atomic::Ordering::Acquire);
        if !ptr.is_null() {
            return ptr;
        }
        self.init_slow()
    }

    #[cfg(not(target_os = "none"))]
    #[cold]
    fn init_slow(&self) -> *mut libc::pthread_cond_t {
        let boxed = Box::into_raw(Box::new(libc::PTHREAD_COND_INITIALIZER));
        #[cfg(not(any(target_os = "macos", target_os = "android", target_os = "espidf")))]
        unsafe {
            let mut attr = core::mem::MaybeUninit::<libc::pthread_condattr_t>::uninit();
            if libc::pthread_condattr_init(attr.as_mut_ptr()) == 0 {
                let _ = libc::pthread_condattr_setclock(
                    attr.as_mut_ptr(),
                    libc::CLOCK_MONOTONIC as core::ffi::c_int,
                );
                let _ = libc::pthread_cond_init(boxed, attr.as_ptr().cast());
                let _ = libc::pthread_condattr_destroy(attr.as_mut_ptr());
            }
        }
        match self.inner.compare_exchange(
            core::ptr::null_mut(),
            boxed,
            core::sync::atomic::Ordering::Release,
            core::sync::atomic::Ordering::Acquire,
        ) {
            Ok(_) => boxed,
            Err(existing) => {
                unsafe {
                    libc::pthread_cond_destroy(boxed);
                    drop(Box::from_raw(boxed));
                }
                existing
            }
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
        #[cfg(target_os = "none")]
        {
            let seq = self.seq.load(core::sync::atomic::Ordering::Acquire);
            self.waiters.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            unsafe { mutex.inner.raw_unlock() };
            let _ = ::libc_rs::sync::futex_wait(&self.seq, seq, None);
            self.waiters.fetch_sub(1, core::sync::atomic::Ordering::Relaxed);
            mutex.lock()
        }
        #[cfg(not(target_os = "none"))]
        {
            // Counted while the mutex is held, which is what makes `notify_one`'s answer exact for a
            // notifier that holds the same mutex. See `waiters`.
            self.waiters.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            unsafe { libc::pthread_cond_wait(self.raw(), mutex.inner.get()) };
            self.waiters.fetch_sub(1, core::sync::atomic::Ordering::Relaxed);
            Guard { mutex }
        }
    }

    /// Release the lock, wait to be notified or until `dur` elapses, and take it again.
    ///
    /// Returns `(guard, timed_out)`.
    pub fn wait_timeout<'a, T>(
        &self,
        guard: Guard<'a, T>,
        dur: core::time::Duration,
    ) -> (Guard<'a, T>, bool) {
        if dur.is_zero() {
            return (guard, true);
        }
        let mutex = guard.mutex;
        core::mem::forget(guard);
        #[cfg(target_os = "none")]
        {
            let start = crate::time::Instant::now();
            let seq = self.seq.load(core::sync::atomic::Ordering::Acquire);
            self.waiters.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            unsafe { mutex.inner.raw_unlock() };
            let ts = ::libc_rs::time::Timespec {
                tv_sec: dur.as_secs().min(i64::MAX as u64) as i64,
                tv_nsec: i64::from(dur.subsec_nanos()),
            };
            ::libc_rs::sync::futex_wait(&self.seq, seq, Some(&ts));
            self.waiters.fetch_sub(1, core::sync::atomic::Ordering::Relaxed);
            let timed_out = start.elapsed() >= dur;
            (mutex.lock(), timed_out)
        }
        #[cfg(not(target_os = "none"))]
        {
            let abstime = cond_abstime(dur);
            self.waiters.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            let rc = match abstime {
                Some(ts) => unsafe {
                    libc::pthread_cond_timedwait(self.raw(), mutex.inner.get(), &ts)
                },
                None => unsafe { libc::pthread_cond_wait(self.raw(), mutex.inner.get()) },
            };
            self.waiters.fetch_sub(1, core::sync::atomic::Ordering::Relaxed);
            (Guard { mutex }, rc == libc::ETIMEDOUT)
        }
    }

    /// Wake one waiter. Returns whether there was one to wake.
    ///
    /// **The answer is exact only for a caller holding the mutex the waiters wait on**, because
    /// the count is maintained under that lock. `QueryLatch::set` is such a caller. A caller that
    /// does not hold it races with a thread entering `wait`, and gets a best-effort answer - a
    /// `false` means nobody had entered `wait` yet, not that nobody ever will.
    pub fn notify_one(&self) -> bool {
        let had_waiter = self.waiters.fetch_add(0, core::sync::atomic::Ordering::Relaxed) > 0;
        #[cfg(target_os = "none")]
        {
            self.seq.fetch_add(1, core::sync::atomic::Ordering::Release);
            let _ = ::libc_rs::sync::futex_wake_one(&self.seq);
        }
        #[cfg(not(target_os = "none"))]
        {
            let ptr = self.inner.load(core::sync::atomic::Ordering::Acquire);
            if !ptr.is_null() {
                unsafe { libc::pthread_cond_signal(ptr) };
            }
        }
        had_waiter
    }

    /// Wake all of them. Returns how many were waiting.
    pub fn notify_all(&self) -> usize {
        let waiting = self.waiters.fetch_add(0, core::sync::atomic::Ordering::Relaxed);
        #[cfg(target_os = "none")]
        {
            self.seq.fetch_add(1, core::sync::atomic::Ordering::Release);
            let _ = ::libc_rs::sync::futex_wake_all(&self.seq);
        }
        #[cfg(not(target_os = "none"))]
        {
            let ptr = self.inner.load(core::sync::atomic::Ordering::Acquire);
            if !ptr.is_null() {
                unsafe { libc::pthread_cond_broadcast(ptr) };
            }
        }
        waiting
    }
}

#[cfg(not(target_os = "none"))]
fn cond_abstime(dur: core::time::Duration) -> Option<libc::timespec> {
    if dur.as_secs() > core::ffi::c_long::MAX as u64 {
        return None;
    }
    #[cfg(target_os = "macos")]
    let now = {
        let mut tv = libc::timeval { tv_sec: 0, tv_usec: 0 };
        unsafe { libc::gettimeofday(&mut tv, core::ptr::null_mut()) };
        libc::timespec {
            tv_sec: tv.tv_sec,
            tv_nsec: core::ffi::c_long::from(tv.tv_usec) * 1000,
        }
    };
    #[cfg(not(target_os = "macos"))]
    let now = {
        let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
        let clk = if cfg!(target_os = "android") {
            libc::CLOCK_REALTIME
        } else {
            libc::CLOCK_MONOTONIC
        };
        unsafe { libc::clock_gettime(clk, &mut ts) };
        ts
    };
    let mut nsec = now.tv_nsec + dur.subsec_nanos() as core::ffi::c_long;
    let mut sec = now.tv_sec.checked_add(dur.as_secs() as _);
    if nsec >= 1_000_000_000 {
        nsec -= 1_000_000_000;
        sec = sec.and_then(|s| s.checked_add(1));
    }
    sec.map(|tv_sec| libc::timespec { tv_sec, tv_nsec: nsec })
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
        #[cfg(not(target_os = "none"))]
        {
            let ptr = *self.inner.get_mut();
            if !ptr.is_null() {
                unsafe {
                    libc::pthread_cond_destroy(ptr);
                    drop(Box::from_raw(ptr));
                }
            }
        }
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
        if self.state.fetch_add(0, Ordering::Acquire) != ONCE_READY {
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
                    // If `f` unwinds, the lock goes back to empty and the panic stays the
                    // caller's, as with `std`'s. Left busy, every later caller would yield
                    // forever waiting for a value that is never coming.
                    struct Unfinished<'a>(&'a core::sync::atomic::AtomicU8);
                    impl Drop for Unfinished<'_> {
                        fn drop(&mut self) {
                            self.0.store(ONCE_EMPTY, Ordering::Release);
                        }
                    }
                    let unfinished = Unfinished(&self.state);
                    let v = f();
                    core::mem::forget(unfinished);
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

const RW_WRITER_BIT: u32 = 1 << 31;
const RW_WAITERS_BIT: u32 = 1 << 30;
const RW_READER_MASK: u32 = 0x3fff_ffff;

#[cfg(all(target_os = "macos", not(target_os = "none")))]
mod ulock {
    use core::ffi::{c_int, c_void};
    use core::sync::atomic::AtomicU32;

    const UL_COMPARE_AND_WAIT: u32 = 1;
    const ULF_WAKE_ALL: u32 = 0x0000_0100;
    const ULF_NO_ERRNO: u32 = 0x0100_0000;

    unsafe extern "C" {
        fn __ulock_wait(operation: u32, addr: *mut c_void, value: u64, timeout: u32) -> c_int;
        fn __ulock_wake(operation: u32, addr: *mut c_void, wake_value: u64) -> c_int;
    }

    pub(super) fn wait(addr: &AtomicU32, expected: u32, timeout_us: u32) {
        unsafe {
            __ulock_wait(
                UL_COMPARE_AND_WAIT | ULF_NO_ERRNO,
                core::ptr::from_ref(addr).cast_mut().cast(),
                expected as u64,
                timeout_us,
            );
        }
    }

    pub(super) fn wake_all(addr: &AtomicU32) {
        unsafe {
            __ulock_wake(
                UL_COMPARE_AND_WAIT | ULF_WAKE_ALL | ULF_NO_ERRNO,
                core::ptr::from_ref(addr).cast_mut().cast(),
                0,
            );
        }
    }
}

/// A move-safe, pure-atomic reader-writer lock.
///
/// Reader-preferring on the fast path: recursive `.read()` calls on the same thread never deadlock
/// against a waiting writer, and the lock has no address-sensitive OS state so moving it by value
/// before or after use is always sound.
pub struct RwLock<T: ?Sized> {
    state: core::sync::atomic::AtomicU32,
    value: core::cell::UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for RwLock<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for RwLock<T> {}

impl<T: Default> Default for RwLock<T> {
    fn default() -> Self {
        RwLock::new(T::default())
    }
}

impl<T: ?Sized> core::fmt::Debug for RwLock<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("RwLock { .. }")
    }
}

impl<T> RwLock<T> {
    /// An unlocked lock.
    pub const fn new(value: T) -> RwLock<T> {
        RwLock {
            state: core::sync::atomic::AtomicU32::new(0),
            value: core::cell::UnsafeCell::new(value),
        }
    }

    /// Consume the lock and return the inner value.
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
}

impl<T: ?Sized> RwLock<T> {
    /// Try to take a shared read lock without blocking.
    pub fn try_read(&self) -> Option<ReadGuard<'_, T>> {
        use core::sync::atomic::Ordering;
        let mut s = self.state.load(Ordering::Relaxed);
        loop {
            if (s & RW_WRITER_BIT) != 0 {
                return None;
            }
            let readers = s & RW_READER_MASK;
            assert!(readers < RW_READER_MASK, "RwLock reader count overflow");
            match self.state.compare_exchange_weak(s, s + 1, Ordering::Acquire, Ordering::Relaxed) {
                Ok(_) => return Some(ReadGuard { lock: self }),
                Err(next) => s = next,
            }
        }
    }

    /// Take a shared read lock.
    pub fn read(&self) -> ReadGuard<'_, T> {
        use core::sync::atomic::Ordering;
        let mut s = self.state.load(Ordering::Relaxed);
        let mut spins = 0u32;
        loop {
            if (s & RW_WRITER_BIT) == 0 {
                let readers = s & RW_READER_MASK;
                assert!(readers < RW_READER_MASK, "RwLock reader count overflow");
                match self.state.compare_exchange_weak(
                    s,
                    s + 1,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return ReadGuard { lock: self },
                    Err(next) => {
                        s = next;
                        continue;
                    }
                }
            }
            if spins < 64 {
                spins += 1;
                core::hint::spin_loop();
                s = self.state.load(Ordering::Relaxed);
                continue;
            }
            #[cfg(target_os = "none")]
            {
                if (s & RW_WAITERS_BIT) == 0 {
                    match self.state.compare_exchange_weak(
                        s,
                        s | RW_WAITERS_BIT,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => s |= RW_WAITERS_BIT,
                        Err(next) => {
                            s = next;
                            continue;
                        }
                    }
                }
                let _ = ::libc_rs::sync::futex_wait(&self.state, s, None);
            }
            #[cfg(all(target_os = "macos", not(target_os = "none")))]
            {
                if (s & RW_WAITERS_BIT) == 0 {
                    match self.state.compare_exchange_weak(
                        s,
                        s | RW_WAITERS_BIT,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => s |= RW_WAITERS_BIT,
                        Err(next) => {
                            s = next;
                            continue;
                        }
                    }
                }
                ulock::wait(&self.state, s, 1000);
            }
            #[cfg(all(not(target_os = "macos"), not(target_os = "none")))]
            unsafe {
                libc::sched_yield();
            }
            s = self.state.load(Ordering::Relaxed);
        }
    }

    /// Try to take the exclusive write lock without blocking.
    pub fn try_write(&self) -> Option<WriteGuard<'_, T>> {
        use core::sync::atomic::Ordering;
        let mut s = self.state.load(Ordering::Relaxed);
        loop {
            if (s & (RW_WRITER_BIT | RW_READER_MASK)) != 0 {
                return None;
            }
            match self.state.compare_exchange_weak(
                s,
                s | RW_WRITER_BIT,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Some(WriteGuard { lock: self }),
                Err(next) => s = next,
            }
        }
    }

    /// Take the exclusive write lock.
    pub fn write(&self) -> WriteGuard<'_, T> {
        use core::sync::atomic::Ordering;
        let mut s = self.state.load(Ordering::Relaxed);
        let mut spins = 0u32;
        loop {
            if (s & (RW_WRITER_BIT | RW_READER_MASK)) == 0 {
                match self.state.compare_exchange_weak(
                    s,
                    s | RW_WRITER_BIT,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return WriteGuard { lock: self },
                    Err(next) => {
                        s = next;
                        continue;
                    }
                }
            }
            if spins < 64 {
                spins += 1;
                core::hint::spin_loop();
                s = self.state.load(Ordering::Relaxed);
                continue;
            }
            #[cfg(target_os = "none")]
            {
                if (s & RW_WAITERS_BIT) == 0 {
                    match self.state.compare_exchange_weak(
                        s,
                        s | RW_WAITERS_BIT,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => s |= RW_WAITERS_BIT,
                        Err(next) => {
                            s = next;
                            continue;
                        }
                    }
                }
                let _ = ::libc_rs::sync::futex_wait(&self.state, s, None);
            }
            #[cfg(all(target_os = "macos", not(target_os = "none")))]
            {
                if (s & RW_WAITERS_BIT) == 0 {
                    match self.state.compare_exchange_weak(
                        s,
                        s | RW_WAITERS_BIT,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => s |= RW_WAITERS_BIT,
                        Err(next) => {
                            s = next;
                            continue;
                        }
                    }
                }
                ulock::wait(&self.state, s, 1000);
            }
            #[cfg(all(not(target_os = "macos"), not(target_os = "none")))]
            unsafe {
                libc::sched_yield();
            }
            s = self.state.load(Ordering::Relaxed);
        }
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
        use core::sync::atomic::Ordering;
        let prev = self.lock.state.fetch_sub(1, Ordering::Release);
        if (prev & RW_READER_MASK) == 1 && (prev & RW_WAITERS_BIT) != 0 {
            self.lock.state.fetch_and(!RW_WAITERS_BIT, Ordering::Relaxed);
            #[cfg(target_os = "none")]
            {
                let _ = ::libc_rs::sync::futex_wake_all(&self.lock.state);
            }
            #[cfg(all(target_os = "macos", not(target_os = "none")))]
            ulock::wake_all(&self.lock.state);
        }
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
        use core::sync::atomic::Ordering;
        let prev = self.lock.state.fetch_and(!(RW_WRITER_BIT | RW_WAITERS_BIT), Ordering::Release);
        if (prev & RW_WAITERS_BIT) != 0 {
            #[cfg(target_os = "none")]
            {
                let _ = ::libc_rs::sync::futex_wake_all(&self.lock.state);
            }
            #[cfg(all(target_os = "macos", not(target_os = "none")))]
            ulock::wake_all(&self.lock.state);
        }
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
    #[cfg(target_os = "none")]
    {
        ThreadId(::libc_rs::thread::current_thread_id())
    }
    #[cfg(not(target_os = "none"))]
    {
        ThreadId(unsafe { libc::pthread_self() } as usize as u64)
    }
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
    #[cfg(target_os = "none")]
    handle: Option<::libc_rs::thread::ThreadHandle>,
    #[cfg(not(target_os = "none"))]
    tid: libc::pthread_t,
    // Some until a successful join or detach. The worker owns a separate Arc.
    slot: Option<alloc::sync::Arc<ThreadResult<T>>>,
}

struct ThreadResult<T> {
    value: core::cell::UnsafeCell<Option<T>>,
}

// Only the worker writes; only a successful native join permits the handle to read.
// Arc destruction cannot overlap either access because each accessor owns an Arc.
unsafe impl<T: Send> Sync for ThreadResult<T> {}

#[cfg(not(target_os = "none"))]
unsafe extern "C" {
    fn pthread_detach(thread: libc::pthread_t) -> core::ffi::c_int;
    fn pthread_equal(left: libc::pthread_t, right: libc::pthread_t) -> core::ffi::c_int;
}

// Native handle operations require exclusive access; the result is shared with the worker.
unsafe impl<T: Send> Send for JoinHandle<T> {}

struct SpawnPayload<F, T> {
    f: F,
    slot: alloc::sync::Arc<ThreadResult<T>>,
}

#[cfg(target_os = "none")]
extern "C" fn spawn_trampoline_rs<F: FnOnce() -> T + Send + 'static, T>(arg: *mut u8) {
    let payload: Box<SpawnPayload<F, T>> = unsafe { Box::from_raw(arg.cast()) };
    let SpawnPayload { f, slot } = *payload;
    let value = f();
    unsafe { *slot.value.get() = Some(value) };
}

#[cfg(not(target_os = "none"))]
extern "C" fn spawn_trampoline<F: FnOnce() -> T + Send + 'static, T>(
    arg: *mut libc::c_void,
) -> *mut libc::c_void {
    let payload: Box<SpawnPayload<F, T>> = unsafe { Box::from_raw(arg.cast()) };
    let SpawnPayload { f, slot } = *payload;
    let value = f();
    // This worker is the sole writer. Join reads only after pthread_join succeeds.
    // Detach drops only the handle's Arc; this Arc protects the write and result.
    unsafe { *slot.value.get() = Some(value) };
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
        self.spawn_inner(f)
    }

    fn spawn_inner<T: Send + 'static, F: FnOnce() -> T + Send + 'static>(
        self,
        f: F,
    ) -> Result<JoinHandle<T>, Errno> {
        let slot = alloc::sync::Arc::new(ThreadResult { value: core::cell::UnsafeCell::new(None) });
        let payload =
            Box::into_raw(Box::new(SpawnPayload { f, slot: alloc::sync::Arc::clone(&slot) }));

        #[cfg(target_os = "none")]
        {
            let stack = if self.stack_bytes == 0 {
                ::libc_rs::thread::DEFAULT_STACK_BYTES
            } else {
                self.stack_bytes
            };
            let _ = self.name;
            match unsafe {
                ::libc_rs::thread::spawn_thread(
                    stack,
                    spawn_trampoline_rs::<F, T>,
                    payload.cast(),
                    false,
                )
            } {
                Ok(handle) => Ok(JoinHandle { handle, slot: Some(slot) }),
                Err(e) => {
                    drop(unsafe { Box::from_raw(payload) });
                    Err(Errno(e.0))
                }
            }
        }

        #[cfg(not(target_os = "none"))]
        {
            let mut attr: libc::pthread_attr_t = unsafe { core::mem::zeroed() };
            let init_rc = unsafe { libc::pthread_attr_init(&mut attr) };
            if init_rc != 0 {
                drop(unsafe { Box::from_raw(payload) });
                return Err(Errno(init_rc));
            }
            if self.stack_bytes != 0 {
                let stack_rc =
                    unsafe { libc::pthread_attr_setstacksize(&mut attr, self.stack_bytes) };
                if stack_rc != 0 {
                    unsafe { libc::pthread_attr_destroy(&mut attr) };
                    drop(unsafe { Box::from_raw(payload) });
                    return Err(Errno(stack_rc));
                }
            }
            let mut tid: libc::pthread_t = unsafe { core::mem::zeroed() };
            let rc = unsafe {
                libc::pthread_create(&mut tid, &attr, spawn_trampoline::<F, T>, payload.cast())
            };
            unsafe { libc::pthread_attr_destroy(&mut attr) };
            if rc != 0 {
                // Nothing started, so both allocations are still ours to reclaim.
                drop(unsafe { Box::from_raw(payload) });
                return Err(Errno(rc));
            }
            // The name is set from the parent because `pthread_setname_np` on macOS only names the
            // *calling* thread, so the child would have to do it itself; the name is a debugging
            // affordance and not worth another hop through the payload.
            let _ = self.name;
            Ok(JoinHandle { tid, slot: Some(slot) })
        }
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
    /// Whether this still-owned native thread is the caller.
    /// Returns false after successful join or detach, without using a stale native ID.
    pub fn is_current_thread(&self) -> bool {
        #[cfg(target_os = "none")]
        {
            self.slot.is_some() && self.handle.as_ref().is_some_and(|h| h.is_current())
        }
        #[cfg(not(target_os = "none"))]
        {
            self.slot.is_some() && unsafe { pthread_equal(self.tid, libc::pthread_self()) != 0 }
        }
    }

    /// Wait for completion and take the result, retaining ownership on native failure.
    /// Despite its name this blocks, just like pthread_join. A successful call consumes
    /// the native ownership; subsequent calls and self-join return EINVAL.
    /// Panic aborts the process rather than producing a recoverable result.
    pub fn try_join(&mut self) -> Result<T, Errno> {
        if self.slot.is_none() {
            return Err(Errno(libc::EINVAL));
        }
        if self.is_current_thread() {
            return Err(Errno(libc::EINVAL));
        }
        #[cfg(target_os = "none")]
        {
            if let Some(handle) = self.handle.take() {
                handle.join().map_err(|e| Errno(e.0))?;
            }
        }
        #[cfg(not(target_os = "none"))]
        {
            let rc = unsafe { libc::pthread_join(self.tid, core::ptr::null_mut()) };
            if rc != 0 {
                return Err(Errno(rc));
            }
        }
        // Native join orders the worker's sole write and all thread teardown before us.
        let slot = self.slot.take().unwrap();
        unsafe { (&mut *slot.value.get()).take() }.ok_or(Errno(libc::EINVAL))
    }

    /// Wait for the thread and take its result. On native failure, Drop detaches it.
    /// Use try_join when the caller needs to retain ownership after an error.
    pub fn join(mut self) -> Result<T, Errno> {
        self.try_join()
    }

    /// Relinquish native ownership without waiting, including from this thread itself.
    /// On failure this handle stays live and can be retried or joined. On success the
    /// result is discarded when the last Arc goes away, on this caller if the worker
    /// already finished, or on the worker otherwise. No completion guarantee is made.
    /// Repeated calls after successful join/detach are harmless.
    pub fn try_detach(&mut self) -> Result<(), Errno> {
        if self.slot.is_none() {
            return Ok(());
        }
        #[cfg(target_os = "none")]
        {
            if let Some(handle) = self.handle.take() {
                handle.detach().map_err(|e| Errno(e.0))?;
            }
        }
        #[cfg(not(target_os = "none"))]
        {
            let rc = unsafe { pthread_detach(self.tid) };
            if rc != 0 {
                return Err(Errno(rc));
            }
        }
        self.slot.take();
        Ok(())
    }
}

impl<T> Drop for JoinHandle<T> {
    /// Never waits or starts a reaper. Failure to release native ownership is fatal;
    /// use try_detach explicitly to handle a native error while retaining the handle.
    fn drop(&mut self) {
        if self.try_detach().is_err() {
            #[cfg(target_os = "none")]
            panic!("JoinHandle::try_detach failed");
            #[cfg(not(target_os = "none"))]
            unsafe {
                libc::abort()
            }
        }
    }
}

// ================================================================================================
// Scoped threads
// ================================================================================================

/// The handle a scoped thread is started through.
pub struct Scope<'scope, 'env: 'scope> {
    #[cfg(target_os = "none")]
    handles: *mut Vec<::libc_rs::thread::ThreadHandle>,
    #[cfg(not(target_os = "none"))]
    handles: *mut Vec<libc::pthread_t>,
    _marker: core::marker::PhantomData<(&'scope (), &'env ())>,
}

struct ScopedPayload<F> {
    f: F,
}

#[cfg(target_os = "none")]
extern "C" fn scoped_trampoline_rs<F: FnOnce() + Send>(arg: *mut u8) {
    let payload: Box<ScopedPayload<F>> = unsafe { Box::from_raw(arg.cast()) };
    let ScopedPayload { f } = *payload;
    f();
}

#[cfg(not(target_os = "none"))]
extern "C" fn scoped_trampoline<F: FnOnce() + Send>(arg: *mut libc::c_void) -> *mut libc::c_void {
    let payload: Box<ScopedPayload<F>> = unsafe { Box::from_raw(arg.cast()) };
    let ScopedPayload { f } = *payload;
    f();
    core::ptr::null_mut()
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
        // The concrete payload retains 'scope in its type. scope joins it before borrowed data expires.
        let payload = Box::into_raw(Box::new(ScopedPayload { f }));

        #[cfg(target_os = "none")]
        {
            match unsafe {
                ::libc_rs::thread::spawn_thread(
                    ::libc_rs::thread::DEFAULT_STACK_BYTES,
                    scoped_trampoline_rs::<F>,
                    payload.cast(),
                    false,
                )
            } {
                Ok(Some(h)) => {
                    let handles = self.handles;
                    unsafe { (*handles).push(h) };
                }
                Ok(None) => {}
                Err(_) => {
                    let payload = unsafe { Box::from_raw(payload) };
                    (payload.f)();
                }
            }
        }

        #[cfg(not(target_os = "none"))]
        {
            let mut tid: libc::pthread_t = unsafe { core::mem::zeroed() };
            let rc = unsafe {
                libc::pthread_create(
                    &mut tid,
                    core::ptr::null(),
                    scoped_trampoline::<F>,
                    payload.cast(),
                )
            };
            if rc != 0 {
                // The thread never started, so the closure is still ours - and dropping it is the
                // only honest thing to do, because there is no way to report from here without
                // making every call site handle a failure that means the machine is out of threads.
                drop(unsafe { Box::from_raw(payload) });
                return;
            }
            let handles = self.handles;
            unsafe { (*handles).push(tid) };
        }
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
    #[cfg(target_os = "none")]
    {
        let mut handles: Vec<::libc_rs::thread::ThreadHandle> = Vec::new();
        let sc = Scope {
            handles: core::ptr::from_mut(&mut handles),
            _marker: core::marker::PhantomData,
        };
        let out = f(&sc);
        for h in handles {
            let _ = h.join();
        }
        out
    }
    #[cfg(not(target_os = "none"))]
    {
        let mut handles: Vec<libc::pthread_t> = Vec::new();
        let sc = Scope {
            handles: core::ptr::from_mut(&mut handles),
            _marker: core::marker::PhantomData,
        };
        let out = f(&sc);
        let slice = handles.as_slice();
        let len = slice.len();
        let mut i = 0usize;
        while i < len {
            unsafe { libc::pthread_join(slice[i], core::ptr::null_mut()) };
            i += 1;
        }
        out
    }
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
fn init_scoped_cell() -> core::cell::Cell<*const ()> {
    core::cell::Cell::new(core::ptr::null())
}

pub struct ScopedKey<T: 'static> {
    slot: ThreadLocal<core::cell::Cell<*const ()>>,
    _marker: core::marker::PhantomData<T>,
}

// Same reasoning as `LocalKey`: the key is per-thread, so it is shareable regardless of `T`.
unsafe impl<T: 'static> Sync for ScopedKey<T> {}
unsafe impl<T: 'static> Send for ScopedKey<T> {}

impl<T: 'static> Default for ScopedKey<T> {
    fn default() -> ScopedKey<T> {
        ScopedKey::new()
    }
}

impl<T: 'static> ScopedKey<T> {
    /// Not called directly; [`scoped_thread_local!`] builds these.
    pub const fn new() -> ScopedKey<T> {
        ScopedKey { slot: ThreadLocal::new(), _marker: core::marker::PhantomData }
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
            slot: &'static ThreadLocal<core::cell::Cell<*const ()>>,
            previous: *const (),
        }

        impl Drop for Reset {
            fn drop(&mut self) {
                self.slot
                    .with(|| init_scoped_cell(), |c| c.set(self.previous))
                    .expect("a thread can always reach its own key");
            }
        }

        let previous = self
            .slot
            .with(|| init_scoped_cell(), |c| c.replace((value as *const T).cast()))
            .expect("a thread can always reach its own key");
        let _reset = Reset { slot: &self.slot, previous };
        f()
    }

    /// Read the installed value.
    ///
    /// Panics if nothing is installed. That is `scoped_tls`'s behaviour and the right one: a
    /// caller reaching for session globals outside a session has a bug that a `None` would only
    /// move somewhere less obvious.
    pub fn with<R>(&'static self, f: impl FnOnce(&T) -> R) -> R {
        let p = self
            .slot
            .with(|| init_scoped_cell(), |c| c.get())
            .expect("a thread can always reach its own key");
        assert!(!p.is_null(), "cannot access a scoped thread local that has not been set");
        // Safe: the pointer was written by `set`, whose closure is still running - so the
        // referent outlives this borrow - and it is cleared on the way out.
        f(unsafe { &*p.cast::<T>() })
    }

    /// Whether a value is installed on this thread.
    pub fn is_set(&'static self) -> bool {
        !self
            .slot
            .with(|| init_scoped_cell(), |c| c.get())
            .expect("a thread can always reach its own key")
            .is_null()
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::sync::Arc;
    use alloc::vec::Vec;
    use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    #[test]
    fn rwlock_basic_and_move_after_use() {
        let lock = RwLock::new(10u32);
        assert_eq!(*lock.read(), 10);
        assert!(lock.try_read().is_some());
        {
            let r1 = lock.read();
            let r2 = lock.read();
            assert_eq!(*r1, 10);
            assert_eq!(*r2, 10);
            assert!(lock.try_write().is_none());
        }
        {
            let mut w = lock.write();
            *w = 20;
            assert!(lock.try_read().is_none());
            assert!(lock.try_write().is_none());
        }
        // Move by value after lock has been used.
        let mut vec = Vec::new();
        vec.push(lock);
        vec.reserve(64);
        let moved = vec.pop().unwrap();
        assert_eq!(*moved.read(), 20);
        let mut moved = moved;
        *moved.get_mut() = 30;
        assert_eq!(moved.into_inner(), 30);
    }

    #[cfg(not(target_os = "none"))]
    #[test]
    fn rwlock_recursive_read_with_waiting_writer_does_not_deadlock() {
        let lock = Arc::new(RwLock::new(42u32));
        let writer_waiting = Arc::new(AtomicBool::new(false));
        let writer_done = Arc::new(AtomicBool::new(false));

        let r1 = lock.read();
        assert_eq!(*r1, 42);

        let lock2 = Arc::clone(&lock);
        let ww = Arc::clone(&writer_waiting);
        let wd = Arc::clone(&writer_done);
        let handle = spawn(move || {
            ww.store(true, Ordering::Release);
            let mut w = lock2.write();
            *w = 99;
            wd.store(true, Ordering::Release);
        })
        .unwrap();

        while !writer_waiting.load(Ordering::Acquire) {
            unsafe { libc::sched_yield() };
        }
        // Give writer a moment to enter wait state on the lock.
        for _ in 0..200 {
            unsafe { libc::sched_yield() };
        }

        // Recursive read on the thread already holding r1 must succeed even with a waiting writer.
        let r2 = lock.read();
        assert_eq!(*r2, 42);
        drop(r2);
        drop(r1);

        handle.join().unwrap();
        assert!(writer_done.load(Ordering::Acquire));
        assert_eq!(*lock.read(), 99);
    }

    #[cfg(not(target_os = "none"))]
    #[test]
    fn rwlock_multithreaded_contention() {
        let lock = Arc::new(RwLock::new(0usize));
        let iters = 200usize;
        let threads = 4usize;

        let mut handles = Vec::new();
        for _ in 0..threads {
            let l = Arc::clone(&lock);
            handles.push(
                spawn(move || {
                    for i in 0..iters {
                        if i % 4 == 0 {
                            let mut w = l.write();
                            *w += 1;
                        } else {
                            let r = l.read();
                            let _ = *r;
                        }
                    }
                })
                .unwrap(),
            );
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(*lock.read(), threads * (iters / 4));
    }

    #[cfg(not(target_os = "none"))]
    #[test]
    fn mutex_and_condvar_move_after_use() {
        let m = Mutex::new(0u32);
        *m.lock() = 1;
        let cv = Condvar::new();
        assert!(!cv.notify_one());

        // Move both after use.
        let mut pair = Some((m, cv));
        let (m2, cv2) = pair.take().unwrap();
        let shared = Arc::new((m2, cv2, AtomicUsize::new(0)));

        let s2 = Arc::clone(&shared);
        let h = spawn(move || {
            let mut g = s2.0.lock();
            while *g == 1 {
                g = s2.1.wait(g);
            }
            assert_eq!(*g, 2);
            s2.2.store(1, Ordering::Release);
        })
        .unwrap();

        while shared.2.load(Ordering::Acquire) == 0 {
            let mut g = shared.0.lock();
            *g = 2;
            shared.1.notify_all();
            drop(g);
            unsafe { libc::sched_yield() };
        }
        h.join().unwrap();
    }

    #[cfg(not(target_os = "none"))]
    #[test]
    fn condvar_wait_timeout_interruptible_and_expires() {
        use core::time::Duration;
        let pair = Arc::new((Mutex::new(false), Condvar::new()));

        // Expiry path.
        let t0 = crate::time::Instant::now();
        let g = pair.0.lock();
        let (g, timed_out) = pair.1.wait_timeout(g, Duration::from_millis(10));
        drop(g);
        assert!(timed_out);
        assert!(t0.elapsed() >= Duration::from_millis(8));

        // Early wake path: notify during a 500ms wait must return promptly.
        let p2 = Arc::clone(&pair);
        let h = spawn(move || {
            crate::sleep_ms(5);
            let mut g = p2.0.lock();
            *g = true;
            p2.1.notify_one();
        })
        .unwrap();

        let t1 = crate::time::Instant::now();
        let mut g = pair.0.lock();
        while !*g {
            let (ng, _) = pair.1.wait_timeout(g, Duration::from_millis(500));
            g = ng;
        }
        assert!(t1.elapsed() < Duration::from_millis(200));
        drop(g);
        h.join().unwrap();
    }

    #[cfg(target_os = "none")]
    #[test]
    fn libc_rs_sync_tls_and_scope_primitives() {
        let m = Mutex::new(10u32);
        {
            let mut g = m.try_lock().expect("try_lock");
            assert!(m.try_lock().is_none());
            *g += 5;
        }
        assert_eq!(*m.lock(), 15);

        let cv = Condvar::new();
        assert!(!cv.notify_one());
        assert_eq!(cv.notify_all(), 0);

        let once = Once::new();
        let count = AtomicUsize::new(0);
        once.call_once(|| {
            count.fetch_add(1, Ordering::Relaxed);
        });
        once.call_once(|| {
            count.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(count.load(Ordering::Relaxed), 1);

        let ol = OnceLock::new();
        assert_eq!(*ol.get_or_init(|| 77u32), 77);
        assert_eq!(*ol.get_or_init(|| 88u32), 77);

        let ll = LazyLock::new(|| 123u32);
        assert_eq!(*ll, 123);

        let tl = ThreadLocal::new();
        let v = tl.with(
            || 41u32,
            |x| {
                *x += 1;
                *x
            },
        );
        assert_eq!(v, Some(42));
        assert_eq!(tl.with_ref(|| 0u32, |x| *x), Some(42));

        crate::thread_local! {
            static LOCAL_COUNTER: core::cell::Cell<u32> = core::cell::Cell::new(5);
        }
        assert_eq!(LOCAL_COUNTER.with(|c| c.get()), 5);
        LOCAL_COUNTER.with(|c| c.set(9));
        assert_eq!(LOCAL_COUNTER.with(|c| c.get()), 9);

        crate::scoped_thread_local!(static SCOPED_FLAG: u32);
        assert!(!SCOPED_FLAG.is_set());
        let val = 33u32;
        let out = SCOPED_FLAG.set(&val, || {
            assert!(SCOPED_FLAG.is_set());
            SCOPED_FLAG.with(|x| *x)
        });
        assert_eq!(out, 33);
        assert!(!SCOPED_FLAG.is_set());

        let _tid = current_id().as_u64();

        let _ = spawn_detached(64 * 1024, || {});
        let _ = spawn(|| 1u32);

        let mut x = 0u32;
        scope(|s| {
            s.spawn(|| {
                x += 1;
            });
        });
        assert!(x <= 1);
    }
}
