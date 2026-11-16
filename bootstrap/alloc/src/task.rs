//! Waker construction from reference-counted wakers.
use crate::sync::Arc;
use core::mem::{self, ManuallyDrop};
use core::task::{RawWaker, RawWakerVTable, Waker};

/// The implementation of waking a task on an executor.
pub trait Wake {
    /// Wakes the task, consuming this reference.
    fn wake(self: Arc<Self>);
    /// Wakes the task without consuming this reference.
    fn wake_by_ref(self: &Arc<Self>) {
        self.clone().wake();
    }
}

impl<W: Wake + Send + Sync + 'static> From<Arc<W>> for Waker {
    fn from(waker: Arc<W>) -> Waker {
        // SAFETY: the vtable functions below uphold the contract of `RawWaker`.
        unsafe { Waker::from_raw(raw_waker(waker)) }
    }
}

impl<W: Wake + Send + Sync + 'static> From<Arc<W>> for RawWaker {
    fn from(waker: Arc<W>) -> RawWaker {
        raw_waker(waker)
    }
}

struct VTable<W>(core::marker::PhantomData<W>);
impl<W: Wake + Send + Sync + 'static> VTable<W> {
    const VTABLE: RawWakerVTable = RawWakerVTable::new(clone_waker::<W>, wake::<W>, wake_by_ref::<W>, drop_waker::<W>);
}

fn raw_waker<W: Wake + Send + Sync + 'static>(waker: Arc<W>) -> RawWaker {
    RawWaker::new(Arc::into_raw(waker) as *const (), &VTable::<W>::VTABLE)
}

    // Increment the reference count of the arc to clone it.
    unsafe fn clone_waker<W: Wake + Send + Sync + 'static>(waker: *const ()) -> RawWaker {
        let current = ManuallyDrop::new(unsafe { Arc::from_raw(waker as *const W) });
        mem::forget(Arc::clone(&current));
        RawWaker::new(
            waker,
            &VTable::<W>::VTABLE,
        )
    }
    // Wake by value, moving the Arc into the Wake::wake function.
    unsafe fn wake<W: Wake + Send + Sync + 'static>(waker: *const ()) {
        let waker = unsafe { Arc::from_raw(waker as *const W) };
        <W as Wake>::wake(waker);
    }
    // Wake by reference, wrap the waker in ManuallyDrop to avoid dropping it.
    unsafe fn wake_by_ref<W: Wake + Send + Sync + 'static>(waker: *const ()) {
        let waker = ManuallyDrop::new(unsafe { Arc::from_raw(waker as *const W) });
        <W as Wake>::wake_by_ref(&waker);
    }
    // Decrement the reference count of the Arc on drop.
    unsafe fn drop_waker<W: Wake + Send + Sync + 'static>(waker: *const ()) {
        unsafe { drop(Arc::from_raw(waker as *const W)) };
    }
