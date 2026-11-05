use crate::marker::{PointeeSized, Tuple};

mod try_trait;
pub use self::try_trait::{ControlFlow, FromResidual, Try};

#[lang = "drop"]
pub const trait Drop {
    fn drop(&mut self);
}

#[lang = "legacy_receiver"]
pub trait LegacyReceiver: PointeeSized {}
impl<T: PointeeSized> LegacyReceiver for &T {}
impl<T: PointeeSized> LegacyReceiver for &mut T {}

#[lang = "deref"]
pub const trait Deref: PointeeSized {
    #[lang = "deref_target"]
    type Target: ?Sized;
    fn deref(&self) -> &Self::Target;
}

#[lang = "deref_mut"]
pub const trait DerefMut: [const] Deref + PointeeSized {
    fn deref_mut(&mut self) -> &mut Self::Target;
}

const impl<T: ?Sized> Deref for &T {
    type Target = T;
    fn deref(&self) -> &T { self }
}
impl<T: ?Sized> !DerefMut for &T {}
const impl<T: ?Sized> Deref for &mut T {
    type Target = T;
    fn deref(&self) -> &T { self }
}
const impl<T: ?Sized> DerefMut for &mut T {
    fn deref_mut(&mut self) -> &mut T { self }
}

#[lang = "fn_once"]
#[rustc_paren_sugar]
#[fundamental]
pub const trait FnOnce<Args: Tuple> {
    #[lang = "fn_once_output"]
    type Output;
    extern "rust-call" fn call_once(self, args: Args) -> Self::Output;
}

#[lang = "fn_mut"]
#[rustc_paren_sugar]
#[fundamental]
pub const trait FnMut<Args: Tuple>: FnOnce<Args> {
    extern "rust-call" fn call_mut(&mut self, args: Args) -> Self::Output;
}

#[lang = "fn"]
#[rustc_paren_sugar]
#[fundamental]
pub const trait Fn<Args: Tuple>: [const] FnMut<Args> {
    extern "rust-call" fn call(&self, args: Args) -> Self::Output;
}
