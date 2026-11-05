pub use core::borrow::{Borrow, BorrowMut};

pub trait ToOwned {
    type Owned: Borrow<Self>;
    fn to_owned(&self) -> Self::Owned;
}

impl<T: Clone> ToOwned for T {
    type Owned = T;
    fn to_owned(&self) -> T { self.clone() }
}

impl<T: Clone> ToOwned for [T] {
    type Owned = crate::vec::Vec<T>;
    fn to_owned(&self) -> Self::Owned {
        let mut result = crate::vec::Vec::with_capacity(self.len());
        result.extend_from_slice(self);
        result
    }
}

impl ToOwned for str {
    type Owned = crate::string::String;
    fn to_owned(&self) -> Self::Owned { crate::string::String::from(self) }
}

pub enum Cow<'a, B: ?Sized + 'a> where B: ToOwned {
    Borrowed(&'a B),
    Owned(<B as ToOwned>::Owned),
}

impl<'a, B: ?Sized + ToOwned> Cow<'a, B> {
    pub fn into_owned(self) -> B::Owned {
        match self { Self::Borrowed(value) => value.to_owned(), Self::Owned(value) => value }
    }
    pub fn as_ref(&self) -> &B {
        match self { Self::Borrowed(value) => value, Self::Owned(value) => value.borrow() }
    }
}

impl<B: ?Sized + ToOwned> core::ops::Deref for Cow<'_, B> {
    type Target = B;
    fn deref(&self) -> &B { self.as_ref() }
}

impl<B> Clone for Cow<'_, B>
where B: ?Sized + ToOwned, B::Owned: Clone {
    fn clone(&self) -> Self {
        match self { Self::Borrowed(value) => Self::Borrowed(value), Self::Owned(value) => Self::Owned(value.clone()) }
    }
}

impl<B: ?Sized + ToOwned + PartialEq> PartialEq for Cow<'_, B> {
    fn eq(&self, other: &Self) -> bool { self.as_ref() == other.as_ref() }
}
impl<B: ?Sized + ToOwned + Eq> Eq for Cow<'_, B> {}
impl<B: ?Sized + ToOwned + core::fmt::Debug> core::fmt::Debug for Cow<'_, B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { core::fmt::Debug::fmt(self.as_ref(), f) }
}
impl<B: ?Sized + ToOwned + core::fmt::Display> core::fmt::Display for Cow<'_, B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { core::fmt::Display::fmt(self.as_ref(), f) }
}
impl<'a, B: ?Sized + ToOwned> From<&'a B> for Cow<'a, B> {
    fn from(value: &'a B) -> Self { Self::Borrowed(value) }
}
impl<'a> From<crate::string::String> for Cow<'a, str> {
    fn from(value: crate::string::String) -> Self { Self::Owned(value) }
}
impl<'a, T: Clone> From<crate::vec::Vec<T>> for Cow<'a, [T]> {
    fn from(value: crate::vec::Vec<T>) -> Self { Self::Owned(value) }
}
