//! Atomic synchronization primitives.
pub mod atomic {
use crate::cell::UnsafeCell;
use crate::clone::Clone;
use crate::cmp::{Eq, PartialEq};
use crate::intrinsics::{self, AtomicOrdering as IOrd};
use crate::marker::{Copy, Send, Sync};
use crate::ops::FnMut;
use crate::option::Option::{self, None, Some};
use crate::result::Result::{self, Err, Ok};

#[repr(u32)]
pub enum Ordering { Relaxed = 0, Release = 1, Acquire = 2, AcqRel = 3, SeqCst = 4 }
impl Copy for Ordering {}
impl Clone for Ordering { fn clone(&self) -> Self { *self } }
impl PartialEq for Ordering {
    fn eq(&self, other: &Self) -> bool { (*self as u32) == (*other as u32) }
}
impl Eq for Ordering {}

macro_rules! load { ($p:expr,$t:ty,$o:ident) => { unsafe { intrinsics::atomic_load::<$t, {IOrd::$o}>($p) } } }
macro_rules! store { ($p:expr,$v:expr,$t:ty,$o:ident) => { unsafe { intrinsics::atomic_store::<$t, {IOrd::$o}>($p,$v) } } }
macro_rules! xchg { ($p:expr,$v:expr,$t:ty,$o:ident) => { unsafe { intrinsics::atomic_xchg::<$t, {IOrd::$o}>($p,$v) } } }
macro_rules! cx { ($intr:ident,$p:expr,$old:expr,$new:expr,$t:ty,$s:expr,$f:expr) => {{
    let (v,ok) = match ($s,$f) {
        (Ordering::Relaxed,Ordering::Relaxed)=>unsafe{intrinsics::$intr::<$t,{IOrd::Relaxed},{IOrd::Relaxed}>($p,$old,$new)},
        (Ordering::Relaxed,Ordering::Acquire)=>unsafe{intrinsics::$intr::<$t,{IOrd::Relaxed},{IOrd::Acquire}>($p,$old,$new)},
        (Ordering::Acquire,Ordering::Relaxed)=>unsafe{intrinsics::$intr::<$t,{IOrd::Acquire},{IOrd::Relaxed}>($p,$old,$new)},
        (Ordering::Acquire,Ordering::Acquire)=>unsafe{intrinsics::$intr::<$t,{IOrd::Acquire},{IOrd::Acquire}>($p,$old,$new)},
        (Ordering::Release,Ordering::Relaxed)=>unsafe{intrinsics::$intr::<$t,{IOrd::Release},{IOrd::Relaxed}>($p,$old,$new)},
        (Ordering::AcqRel,Ordering::Relaxed)=>unsafe{intrinsics::$intr::<$t,{IOrd::AcqRel},{IOrd::Relaxed}>($p,$old,$new)},
        (Ordering::AcqRel,Ordering::Acquire)=>unsafe{intrinsics::$intr::<$t,{IOrd::AcqRel},{IOrd::Acquire}>($p,$old,$new)},
        (Ordering::SeqCst,Ordering::Relaxed)=>unsafe{intrinsics::$intr::<$t,{IOrd::SeqCst},{IOrd::Relaxed}>($p,$old,$new)},
        (Ordering::SeqCst,Ordering::Acquire)=>unsafe{intrinsics::$intr::<$t,{IOrd::SeqCst},{IOrd::Acquire}>($p,$old,$new)},
        (Ordering::SeqCst,Ordering::SeqCst)=>unsafe{intrinsics::$intr::<$t,{IOrd::SeqCst},{IOrd::SeqCst}>($p,$old,$new)},
        _=>panic!("invalid compare_exchange ordering")
    };
    if ok { Ok(v) } else { Err(v) }
} } }

#[repr(transparent)] pub struct AtomicBool(UnsafeCell<u8>);
unsafe impl Sync for AtomicBool {}
impl AtomicBool {
    pub const fn new(v: bool) -> Self { Self(UnsafeCell::new(v as u8)) }
    pub fn load(&self,o:Ordering)->bool { (match o { Ordering::Relaxed=>load!(self.0.get(),u8,Relaxed), Ordering::Acquire=>load!(self.0.get(),u8,Acquire), Ordering::SeqCst=>load!(self.0.get(),u8,SeqCst), _=>panic!("invalid load ordering") }) != 0 }
    pub fn store(&self,v:bool,o:Ordering) { let v = v as u8; match o { Ordering::Relaxed=>store!(self.0.get(),v,u8,Relaxed), Ordering::Release=>store!(self.0.get(),v,u8,Release), Ordering::SeqCst=>store!(self.0.get(),v,u8,SeqCst), _=>panic!("invalid store ordering") } }
    pub fn swap(&self,v:bool,o:Ordering)->bool { let v = v as u8; (match o { Ordering::Relaxed=>xchg!(self.0.get(),v,u8,Relaxed), Ordering::Release=>xchg!(self.0.get(),v,u8,Release), Ordering::Acquire=>xchg!(self.0.get(),v,u8,Acquire), Ordering::AcqRel=>xchg!(self.0.get(),v,u8,AcqRel), Ordering::SeqCst=>xchg!(self.0.get(),v,u8,SeqCst) }) != 0 }
    pub fn compare_exchange(&self,old:bool,new:bool,s:Ordering,f:Ordering)->Result<bool,bool> { match cx!(atomic_cxchg,self.0.get(),old as u8,new as u8,u8,s,f) { Ok(v)=>Ok(v!=0), Err(v)=>Err(v!=0) } }
    pub fn compare_exchange_weak(&self,old:bool,new:bool,s:Ordering,f:Ordering)->Result<bool,bool> { match cx!(atomic_cxchgweak,self.0.get(),old as u8,new as u8,u8,s,f) { Ok(v)=>Ok(v!=0), Err(v)=>Err(v!=0) } }
    pub fn fetch_and(&self,v:bool,o:Ordering)->bool{let v=v as u8;(match o{Ordering::Relaxed=>unsafe{intrinsics::atomic_and::<u8,u8,{IOrd::Relaxed}>(self.0.get(),v)},Ordering::Release=>unsafe{intrinsics::atomic_and::<u8,u8,{IOrd::Release}>(self.0.get(),v)},Ordering::Acquire=>unsafe{intrinsics::atomic_and::<u8,u8,{IOrd::Acquire}>(self.0.get(),v)},Ordering::AcqRel=>unsafe{intrinsics::atomic_and::<u8,u8,{IOrd::AcqRel}>(self.0.get(),v)},Ordering::SeqCst=>unsafe{intrinsics::atomic_and::<u8,u8,{IOrd::SeqCst}>(self.0.get(),v)}})!=0}
    pub fn fetch_or(&self,v:bool,o:Ordering)->bool{let v=v as u8;(match o{Ordering::Relaxed=>unsafe{intrinsics::atomic_or::<u8,u8,{IOrd::Relaxed}>(self.0.get(),v)},Ordering::Release=>unsafe{intrinsics::atomic_or::<u8,u8,{IOrd::Release}>(self.0.get(),v)},Ordering::Acquire=>unsafe{intrinsics::atomic_or::<u8,u8,{IOrd::Acquire}>(self.0.get(),v)},Ordering::AcqRel=>unsafe{intrinsics::atomic_or::<u8,u8,{IOrd::AcqRel}>(self.0.get(),v)},Ordering::SeqCst=>unsafe{intrinsics::atomic_or::<u8,u8,{IOrd::SeqCst}>(self.0.get(),v)}})!=0}
    pub fn fetch_update<F:FnMut(bool)->Option<bool>>(&self,s:Ordering,f:Ordering,mut fun:F)->Result<bool,bool> { let mut cur=self.load(f); loop { match fun(cur) { None=>return Err(cur), Some(n)=>match self.compare_exchange_weak(cur,n,s,f) { Ok(v)=>return Ok(v),Err(v)=>cur=v } } } }
}

macro_rules! atomic_int { ($name:ident,$t:ty) => {
    #[repr(transparent)] pub struct $name(UnsafeCell<$t>); unsafe impl Sync for $name {}
    impl $name {
        pub const fn new(v:$t)->Self{Self(UnsafeCell::new(v))}
        pub fn load(&self,o:Ordering)->$t{match o{Ordering::Relaxed=>load!(self.0.get(),$t,Relaxed),Ordering::Acquire=>load!(self.0.get(),$t,Acquire),Ordering::SeqCst=>load!(self.0.get(),$t,SeqCst),_=>panic!("invalid load ordering")}}
        pub fn store(&self,v:$t,o:Ordering){match o{Ordering::Relaxed=>store!(self.0.get(),v,$t,Relaxed),Ordering::Release=>store!(self.0.get(),v,$t,Release),Ordering::SeqCst=>store!(self.0.get(),v,$t,SeqCst),_=>panic!("invalid store ordering")}}
        pub fn swap(&self,v:$t,o:Ordering)->$t{match o{Ordering::Relaxed=>xchg!(self.0.get(),v,$t,Relaxed),Ordering::Release=>xchg!(self.0.get(),v,$t,Release),Ordering::Acquire=>xchg!(self.0.get(),v,$t,Acquire),Ordering::AcqRel=>xchg!(self.0.get(),v,$t,AcqRel),Ordering::SeqCst=>xchg!(self.0.get(),v,$t,SeqCst)}}
        pub fn compare_exchange(&self,a:$t,b:$t,s:Ordering,f:Ordering)->Result<$t,$t>{cx!(atomic_cxchg,self.0.get(),a,b,$t,s,f)}
        pub fn compare_exchange_weak(&self,a:$t,b:$t,s:Ordering,f:Ordering)->Result<$t,$t>{cx!(atomic_cxchgweak,self.0.get(),a,b,$t,s,f)}
        pub fn fetch_add(&self,v:$t,o:Ordering)->$t{match o{Ordering::Relaxed=>unsafe{intrinsics::atomic_xadd::<$t,$t,{IOrd::Relaxed}>(self.0.get(),v)},Ordering::Release=>unsafe{intrinsics::atomic_xadd::<$t,$t,{IOrd::Release}>(self.0.get(),v)},Ordering::Acquire=>unsafe{intrinsics::atomic_xadd::<$t,$t,{IOrd::Acquire}>(self.0.get(),v)},Ordering::AcqRel=>unsafe{intrinsics::atomic_xadd::<$t,$t,{IOrd::AcqRel}>(self.0.get(),v)},Ordering::SeqCst=>unsafe{intrinsics::atomic_xadd::<$t,$t,{IOrd::SeqCst}>(self.0.get(),v)}}}
        pub fn fetch_sub(&self,v:$t,o:Ordering)->$t{match o{Ordering::Relaxed=>unsafe{intrinsics::atomic_xsub::<$t,$t,{IOrd::Relaxed}>(self.0.get(),v)},Ordering::Release=>unsafe{intrinsics::atomic_xsub::<$t,$t,{IOrd::Release}>(self.0.get(),v)},Ordering::Acquire=>unsafe{intrinsics::atomic_xsub::<$t,$t,{IOrd::Acquire}>(self.0.get(),v)},Ordering::AcqRel=>unsafe{intrinsics::atomic_xsub::<$t,$t,{IOrd::AcqRel}>(self.0.get(),v)},Ordering::SeqCst=>unsafe{intrinsics::atomic_xsub::<$t,$t,{IOrd::SeqCst}>(self.0.get(),v)}}}
        pub fn fetch_and(&self,v:$t,o:Ordering)->$t{match o{Ordering::Relaxed=>unsafe{intrinsics::atomic_and::<$t,$t,{IOrd::Relaxed}>(self.0.get(),v)},Ordering::Release=>unsafe{intrinsics::atomic_and::<$t,$t,{IOrd::Release}>(self.0.get(),v)},Ordering::Acquire=>unsafe{intrinsics::atomic_and::<$t,$t,{IOrd::Acquire}>(self.0.get(),v)},Ordering::AcqRel=>unsafe{intrinsics::atomic_and::<$t,$t,{IOrd::AcqRel}>(self.0.get(),v)},Ordering::SeqCst=>unsafe{intrinsics::atomic_and::<$t,$t,{IOrd::SeqCst}>(self.0.get(),v)}}}
        pub fn fetch_or(&self,v:$t,o:Ordering)->$t{match o{Ordering::Relaxed=>unsafe{intrinsics::atomic_or::<$t,$t,{IOrd::Relaxed}>(self.0.get(),v)},Ordering::Release=>unsafe{intrinsics::atomic_or::<$t,$t,{IOrd::Release}>(self.0.get(),v)},Ordering::Acquire=>unsafe{intrinsics::atomic_or::<$t,$t,{IOrd::Acquire}>(self.0.get(),v)},Ordering::AcqRel=>unsafe{intrinsics::atomic_or::<$t,$t,{IOrd::AcqRel}>(self.0.get(),v)},Ordering::SeqCst=>unsafe{intrinsics::atomic_or::<$t,$t,{IOrd::SeqCst}>(self.0.get(),v)}}}
        pub fn fetch_update<F:FnMut($t)->Option<$t>>(&self,s:Ordering,f:Ordering,mut fun:F)->Result<$t,$t>{let mut cur=self.load(f);loop{match fun(cur){None=>return Err(cur),Some(n)=>match self.compare_exchange_weak(cur,n,s,f){Ok(v)=>return Ok(v),Err(v)=>cur=v}}}}
    }
} }
atomic_int!(AtomicU8,u8); atomic_int!(AtomicU16,u16); atomic_int!(AtomicU32,u32); atomic_int!(AtomicU64,u64);
atomic_int!(AtomicUsize,usize); atomic_int!(AtomicIsize,isize);

#[repr(transparent)] pub struct AtomicPtr<T>(UnsafeCell<*mut T>); unsafe impl<T> Sync for AtomicPtr<T> {} unsafe impl<T> Send for AtomicPtr<T> {}
impl<T> AtomicPtr<T> {
    pub const fn new(p:*mut T)->Self{Self(UnsafeCell::new(p))}
    pub fn load(&self,o:Ordering)->*mut T{match o{Ordering::Relaxed=>load!(self.0.get(),*mut T,Relaxed),Ordering::Acquire=>load!(self.0.get(),*mut T,Acquire),Ordering::SeqCst=>load!(self.0.get(),*mut T,SeqCst),_=>panic!("invalid load ordering")}}
    pub fn store(&self,p:*mut T,o:Ordering){match o{Ordering::Relaxed=>store!(self.0.get(),p,*mut T,Relaxed),Ordering::Release=>store!(self.0.get(),p,*mut T,Release),Ordering::SeqCst=>store!(self.0.get(),p,*mut T,SeqCst),_=>panic!("invalid store ordering")}}
    pub fn swap(&self,p:*mut T,o:Ordering)->*mut T{match o{Ordering::Relaxed=>xchg!(self.0.get(),p,*mut T,Relaxed),Ordering::Release=>xchg!(self.0.get(),p,*mut T,Release),Ordering::Acquire=>xchg!(self.0.get(),p,*mut T,Acquire),Ordering::AcqRel=>xchg!(self.0.get(),p,*mut T,AcqRel),Ordering::SeqCst=>xchg!(self.0.get(),p,*mut T,SeqCst)}}
    pub fn compare_exchange(&self,a:*mut T,b:*mut T,s:Ordering,f:Ordering)->Result<*mut T,*mut T>{cx!(atomic_cxchg,self.0.get(),a,b,*mut T,s,f)}
    pub fn compare_exchange_weak(&self,a:*mut T,b:*mut T,s:Ordering,f:Ordering)->Result<*mut T,*mut T>{cx!(atomic_cxchgweak,self.0.get(),a,b,*mut T,s,f)}
    pub fn fetch_update<F:FnMut(*mut T)->Option<*mut T>>(&self,s:Ordering,f:Ordering,mut fun:F)->Result<*mut T,*mut T>{let mut cur=self.load(f);loop{match fun(cur){None=>return Err(cur),Some(n)=>match self.compare_exchange_weak(cur,n,s,f){Ok(v)=>return Ok(v),Err(v)=>cur=v}}}}
}

pub fn fence(o:Ordering){match o{Ordering::Relaxed=>panic!("fence ordering cannot be Relaxed"),Ordering::Release=>unsafe{intrinsics::atomic_fence::<{IOrd::Release}>()},Ordering::Acquire=>unsafe{intrinsics::atomic_fence::<{IOrd::Acquire}>()},Ordering::AcqRel=>unsafe{intrinsics::atomic_fence::<{IOrd::AcqRel}>()},Ordering::SeqCst=>unsafe{intrinsics::atomic_fence::<{IOrd::SeqCst}>()}}}
pub fn compiler_fence(o:Ordering){match o{Ordering::Relaxed=>panic!("fence ordering cannot be Relaxed"),Ordering::Release=>unsafe{intrinsics::atomic_singlethreadfence::<{IOrd::Release}>()},Ordering::Acquire=>unsafe{intrinsics::atomic_singlethreadfence::<{IOrd::Acquire}>()},Ordering::AcqRel=>unsafe{intrinsics::atomic_singlethreadfence::<{IOrd::AcqRel}>()},Ordering::SeqCst=>unsafe{intrinsics::atomic_singlethreadfence::<{IOrd::SeqCst}>()}}}
}
