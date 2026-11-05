use crate::marker::{Copy, Unpin};
use crate::ops::{Deref, DerefMut};

#[lang = "pin"]
#[fundamental]
#[repr(transparent)]
pub struct Pin<Ptr> { pub pointer: Ptr }

impl<Ptr: crate::ops::LegacyReceiver> crate::ops::LegacyReceiver for Pin<Ptr> {}

impl<P, T: Unpin> Pin<P> where P: Deref<Target = T> {
    pub fn new(pointer: P) -> Self { Self { pointer } }
    pub fn into_inner(self) -> P { self.pointer }
    pub fn get_mut(&mut self) -> &mut P::Target where P: DerefMut { self.pointer.deref_mut() }
}
impl<P> Pin<P> where P: Deref {
    pub unsafe fn new_unchecked(pointer: P) -> Self { Self { pointer } }
    pub fn as_ref(&self) -> Pin<&P::Target> { Pin { pointer: self.pointer.deref() } }
    pub fn get_ref(&self) -> &P::Target { self.pointer.deref() }
}
impl<P> Pin<P> where P: DerefMut {
    pub fn as_mut(&mut self) -> Pin<&mut P::Target> { Pin { pointer: self.pointer.deref_mut() } }
    pub unsafe fn into_inner_unchecked(self) -> P { self.pointer }
    pub unsafe fn get_unchecked_mut(&mut self) -> &mut P::Target { self.pointer.deref_mut() }
}
impl<P: Deref> Deref for Pin<P> {
    type Target = P::Target;
    fn deref(&self) -> &Self::Target { self.pointer.deref() }
}
impl<P: DerefMut, T: Unpin> DerefMut for Pin<P> where P: DerefMut<Target = T> {
    fn deref_mut(&mut self) -> &mut Self::Target { self.pointer.deref_mut() }
}
impl<P: Copy> Copy for Pin<P> {}
impl<P: crate::clone::Clone> crate::clone::Clone for Pin<P> {
    fn clone(&self) -> Self { Self { pointer: self.pointer.clone() } }
}
