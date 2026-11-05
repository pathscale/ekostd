use crate::default::Default;
use crate::marker::PointeeSized;
use crate::marker::PhantomData;

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
}
pub struct BuildHasherDefault<H>(PhantomData<fn() -> H>);
impl<H> BuildHasherDefault<H> {
    pub const fn new() -> Self { Self(PhantomData) }
}
impl<H: Hasher + Default> BuildHasher for BuildHasherDefault<H> {
    type Hasher = H;
    fn build_hasher(&self) -> H { H::default() }
}
