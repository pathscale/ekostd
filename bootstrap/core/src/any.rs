use crate::clone::Clone;
use crate::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use crate::marker::Copy;
use crate::option::Option::{self, Some};

#[lang = "type_id"]
// As in Rust: an array of pointers, because the compiler's const evaluator writes the `type_id` intrinsic result with provenance in the first
// element. At runtime the pointers hold the bits of a 128-bit hash of the type.
pub struct TypeId { data: [*const (); 16 / crate::mem::size_of::<*const ()>()] }
unsafe impl crate::marker::Send for TypeId {}
unsafe impl crate::marker::Sync for TypeId {}
impl TypeId {
    fn as_u128(&self) -> u128 { unsafe { crate::mem::transmute::<TypeId, u128>(*self) } }
}
impl TypeId {
    pub const fn of<T: ?Sized + 'static>() -> TypeId {
        const { crate::intrinsics::type_id::<T>() }
    }
}
impl Copy for TypeId {}
impl Clone for TypeId { fn clone(&self) -> Self { *self } }
impl PartialEq for TypeId { fn eq(&self, other: &Self) -> bool { self.as_u128() == other.as_u128() } }
impl Eq for TypeId {}
impl PartialOrd for TypeId { fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) } }
impl Ord for TypeId { fn cmp(&self, other: &Self) -> Ordering { self.as_u128().cmp(&other.as_u128()) } }
impl crate::hash::Hash for TypeId {
    fn hash<H: crate::hash::Hasher>(&self, state: &mut H) {
        (self.as_u128() as u64).hash(state);
    }
}
impl crate::fmt::Debug for TypeId {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.debug_struct("TypeId").field("t", &self.as_u128()).finish()
    }
}

pub trait Any: 'static {
    fn type_id(&self) -> TypeId;
}
impl<T: 'static + ?Sized> Any for T {
    fn type_id(&self) -> TypeId { TypeId::of::<T>() }
}
pub const fn type_name<T: ?Sized>() -> &'static str { "<type>" }
