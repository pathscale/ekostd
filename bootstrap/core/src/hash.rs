use crate::default::Default;
use crate::marker::PointeeSized;
use crate::marker::PhantomData;
pub use crate::macros::builtin::Hash;

pub trait Hasher {
    fn finish(&self) -> u64;
    fn write(&mut self, bytes: &[u8]);
    fn write_u8(&mut self, i: u8) { self.write(&[i]); }
    fn write_u16(&mut self, i: u16) { self.write(&i.to_ne_bytes()); }
    fn write_u32(&mut self, i: u32) { self.write(&i.to_ne_bytes()); }
    fn write_u64(&mut self, i: u64) { self.write(&i.to_ne_bytes()); }
    fn write_u128(&mut self, i: u128) { self.write(&i.to_ne_bytes()); }
    fn write_usize(&mut self, i: usize) { self.write(&i.to_ne_bytes()); }
    fn write_i8(&mut self, i: i8) { self.write(&[i as u8]); }
    fn write_i16(&mut self, i: i16) { self.write(&i.to_ne_bytes()); }
    fn write_i32(&mut self, i: i32) { self.write(&i.to_ne_bytes()); }
    fn write_i64(&mut self, i: i64) { self.write(&i.to_ne_bytes()); }
    fn write_i128(&mut self, i: i128) { self.write(&i.to_ne_bytes()); }
    fn write_isize(&mut self, i: isize) { self.write(&i.to_ne_bytes()); }
}

pub trait Hash: PointeeSized {
    fn hash<H: Hasher>(&self, state: &mut H);
    fn hash_slice<H: Hasher>(data: &[Self], state: &mut H) where Self: Sized {
        state.write_usize(data.len());
        for value in data { value.hash(state); }
    }
}

macro_rules! hash_integer {
    ($($ty:ty => $method:ident),* $(,)?) => { $(
        impl Hash for $ty {
            fn hash<H: Hasher>(&self, state: &mut H) { state.$method(*self); }
        }
    )* };
}
hash_integer!(u8 => write_u8, u16 => write_u16, u32 => write_u32,
    u64 => write_u64, u128 => write_u128, usize => write_usize,
    i8 => write_i8, i16 => write_i16, i32 => write_i32, i64 => write_i64,
    i128 => write_i128, isize => write_isize);

impl Hash for bool {
    fn hash<H: Hasher>(&self, state: &mut H) { state.write_u8(*self as u8); }
}
impl Hash for char {
    fn hash<H: Hasher>(&self, state: &mut H) { state.write_u32(*self as u32); }
}
impl Hash for () {
    fn hash<H: Hasher>(&self, _: &mut H) {}
}
impl Hash for str {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(self.as_bytes());
        state.write_u8(0xff);
    }
}
impl<T: Hash> Hash for [T] {
    fn hash<H: Hasher>(&self, state: &mut H) { <T as Hash>::hash_slice(self, state); }
}
impl<T: Hash, const N: usize> Hash for [T; N] {
    fn hash<H: Hasher>(&self, state: &mut H) { Hash::hash(&self[..], state); }
}
impl<T: Hash + ?Sized> Hash for &T {
    fn hash<H: Hasher>(&self, state: &mut H) { (**self).hash(state); }
}
impl<T: Hash + ?Sized> Hash for &mut T {
    fn hash<H: Hasher>(&self, state: &mut H) { (**self).hash(state); }
}
impl<T: PointeeSized> Hash for *const T {
    fn hash<H: Hasher>(&self, state: &mut H) { state.write_usize(*self as *const () as usize); }
}
impl<T: PointeeSized> Hash for *mut T {
    fn hash<H: Hasher>(&self, state: &mut H) { state.write_usize(*self as *const () as usize); }
}
impl<T: Hash> Hash for crate::option::Option<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            crate::option::Option::None => state.write_u8(0),
            crate::option::Option::Some(value) => { state.write_u8(1); value.hash(state); }
        }
    }
}
impl<T: Hash, E: Hash> Hash for crate::result::Result<T, E> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            crate::result::Result::Ok(value) => { state.write_u8(0); value.hash(state); }
            crate::result::Result::Err(error) => { state.write_u8(1); error.hash(state); }
        }
    }
}

pub trait BuildHasher {
    type Hasher: Hasher;
    fn build_hasher(&self) -> Self::Hasher;
    /// Hashes one value with a fresh hasher.
    fn hash_one<T: Hash>(&self, value: T) -> u64 where Self: Sized {
        let mut hasher = self.build_hasher();
        value.hash(&mut hasher);
        hasher.finish()
    }
}
pub struct BuildHasherDefault<H>(PhantomData<fn() -> H>);
impl<H> BuildHasherDefault<H> {
    pub const fn new() -> Self { Self(PhantomData) }
}
impl<H: Hasher + Default> BuildHasher for BuildHasherDefault<H> {
    type Hasher = H;
    fn build_hasher(&self) -> H { H::default() }
}

impl<A: Hash, B: Hash> Hash for (A, B) {
    fn hash<H: Hasher>(&self, state: &mut H) { self.0.hash(state); self.1.hash(state); }
}

macro_rules! fnptr_hash_impls {
    (@impl $ty:ty, $($arg:ident),*) => {
        impl<Ret, $($arg),*> Hash for $ty {
            fn hash<HS: Hasher>(&self, state: &mut HS) {
                (*self as *const () as usize).hash(state);
            }
        }
    };
    (@one $($arg:ident),*) => {
        fnptr_hash_impls!(@impl fn($($arg),*) -> Ret, $($arg),*);
        fnptr_hash_impls!(@impl unsafe fn($($arg),*) -> Ret, $($arg),*);
        fnptr_hash_impls!(@impl extern "C" fn($($arg),*) -> Ret, $($arg),*);
        fnptr_hash_impls!(@impl unsafe extern "C" fn($($arg),*) -> Ret, $($arg),*);
    };
    () => {
        fnptr_hash_impls!(@one);
    };
    ($first:ident $(, $rest:ident)*) => {
        fnptr_hash_impls!(@one $first $(, $rest)*);
        fnptr_hash_impls!($($rest),*);
    };
}
fnptr_hash_impls!(A, B, C, D, E, F, G, H, I, J, K, L);

impl<T: ?Sized> Hash for PhantomData<T> {
    fn hash<H: Hasher>(&self, _: &mut H) {}
}
impl<H> Default for BuildHasherDefault<H> {
    fn default() -> Self { Self(PhantomData) }
}
macro_rules! tuple_hash {
    ($(($($T:ident $i:tt),+))*) => {$(
        impl<$($T: Hash),+> Hash for ($($T,)+) {
            fn hash<HS: Hasher>(&self, state: &mut HS) { $(self.$i.hash(state);)+ }
        }
    )*};
}
tuple_hash!((A 0) (A 0, B 1, C 2) (A 0, B 1, C 2, D 3) (A 0, B 1, C 2, D 3, E 4) (A 0, B 1, C 2, D 3, E 4, F 5)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6) (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10, L 11));

/// Hashes the `i8` discriminant: -1 for `Less`, 0 for `Equal`, 1 for `Greater`.
impl Hash for crate::cmp::Ordering {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let discr: i8 = match self {
            crate::cmp::Ordering::Less => -1,
            crate::cmp::Ordering::Equal => 0,
            crate::cmp::Ordering::Greater => 1,
        };
        discr.hash(state);
    }
}

/// Hashes the variant index as an `isize`, then the bound value if there is one.
impl<T: Hash> Hash for crate::ops::Bound<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            crate::ops::Bound::Included(x) => { (0isize).hash(state); x.hash(state); }
            crate::ops::Bound::Excluded(x) => { (1isize).hash(state); x.hash(state); }
            crate::ops::Bound::Unbounded => (2isize).hash(state),
        }
    }
}

/// Hashes `start`, then `end`.
impl<Idx: Hash> Hash for crate::ops::Range<Idx> {
    fn hash<H: Hasher>(&self, state: &mut H) { self.start.hash(state); self.end.hash(state); }
}
/// Hashes `start`.
impl<Idx: Hash> Hash for crate::ops::RangeFrom<Idx> {
    fn hash<H: Hasher>(&self, state: &mut H) { self.start.hash(state); }
}
/// Hashes `end`.
impl<Idx: Hash> Hash for crate::ops::RangeTo<Idx> {
    fn hash<H: Hasher>(&self, state: &mut H) { self.end.hash(state); }
}
/// Hashes `end`.
impl<Idx: Hash> Hash for crate::ops::RangeToInclusive<Idx> {
    fn hash<H: Hasher>(&self, state: &mut H) { self.end.hash(state); }
}
/// A unit struct: hashes nothing.
impl Hash for crate::ops::RangeFull {
    fn hash<H: Hasher>(&self, _: &mut H) {}
}

/// Hashes the wrapped value.
impl<T: Hash> Hash for crate::num::Wrapping<T> {
    fn hash<H: Hasher>(&self, state: &mut H) { self.0.hash(state); }
}
/// Hashes the inner value.
impl<T: ?Sized + Hash> Hash for crate::mem::ManuallyDrop<T> {
    fn hash<H: Hasher>(&self, state: &mut H) { (**self).hash(state); }
}
/// Hashes the value the pinned pointer points to.
impl<Ptr: crate::ops::Deref> Hash for crate::pin::Pin<Ptr> where Ptr::Target: Hash {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Ptr::Target::hash(crate::ops::Deref::deref(&self.pointer), state);
    }
}
/// A unit struct: hashes nothing.
impl Hash for crate::fmt::Error {
    fn hash<H: Hasher>(&self, _: &mut H) {}
}
/// Has no values, so there is nothing to hash.
impl Hash for crate::convert::Infallible {
    fn hash<H: Hasher>(&self, _: &mut H) { match *self {} }
}
/// Hashes the size, then the alignment, each as a `usize`.
impl Hash for crate::alloc::Layout {
    fn hash<H: Hasher>(&self, state: &mut H) { self.size().hash(state); self.align().hash(state); }
}
