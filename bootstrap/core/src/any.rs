use crate::clone::Clone;
use crate::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use crate::marker::Copy;
use crate::option::Option::{self, Some};

#[lang = "type_id"]
pub struct TypeId { t: u128 }
impl TypeId {
    pub const fn of<T: ?Sized + 'static>() -> TypeId {
        crate::intrinsics::type_id::<T>()
    }
}
impl Copy for TypeId {}
impl Clone for TypeId { fn clone(&self) -> Self { *self } }
impl PartialEq for TypeId { fn eq(&self, other: &Self) -> bool { self.t == other.t } }
impl Eq for TypeId {}
impl PartialOrd for TypeId { fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) } }
impl Ord for TypeId { fn cmp(&self, other: &Self) -> Ordering { self.t.cmp(&other.t) } }
impl crate::hash::Hash for TypeId {
    fn hash<H: crate::hash::Hasher>(&self, state: &mut H) {
        self.t.hash(state);
    }
}
impl crate::fmt::Debug for TypeId {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.debug_struct("TypeId").field("t", &self.t).finish()
    }
}

pub trait Any: 'static {
    fn type_id(&self) -> TypeId;
}
impl<T: 'static + ?Sized> Any for T {
    fn type_id(&self) -> TypeId { TypeId::of::<T>() }
}
pub const fn type_name<T: ?Sized>() -> &'static str { "<type>" }
