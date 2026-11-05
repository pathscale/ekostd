use crate::clone::Clone;
use crate::convert::Infallible;
use crate::default::Default;
use crate::marker::{Copy, Destruct};
use crate::ops::{ControlFlow, FnOnce, FromResidual, Try};
use crate::result::Result::{self, Err, Ok};

#[lang = "Option"]
#[rustc_diagnostic_item = "Option"]
pub enum Option<T> {
    #[lang = "None"]
    None,
    #[lang = "Some"]
    Some(T),
}
use self::Option::{None, Some};

impl<T> Option<T> {
    pub const fn is_some(&self) -> bool {
        match self { Some(_) => true, None => false }
    }

    pub const fn is_none(&self) -> bool {
        match self { Some(_) => false, None => true }
    }

    pub const fn as_ref(&self) -> Option<&T> {
        match self { Some(value) => Some(value), None => None }
    }

    pub const fn as_mut(&mut self) -> Option<&mut T> {
        match self { Some(value) => Some(value), None => None }
    }

    #[track_caller]
    pub const fn unwrap(self) -> T
    where
        T: [const] Destruct,
    {
        match self {
            Some(value) => value,
            None => crate::panic::panic_nounwind("called `Option::unwrap()` on a `None` value"),
        }
    }

    #[track_caller]
    pub const fn expect(self, msg: &str) -> T
    where
        T: [const] Destruct,
    {
        match self {
            Some(value) => value,
            None => {
                let _ = msg;
                crate::panic::panic_nounwind("called `Option::expect()` on a `None` value")
            }
        }
    }

    pub const fn unwrap_or(self, default: T) -> T
    where
        T: [const] Destruct,
    {
        match self { Some(value) => value, None => default }
    }

    pub fn unwrap_or_else<F: FnOnce() -> T>(self, f: F) -> T {
        match self { Some(value) => value, None => f() }
    }

    pub const fn map<U, F>(self, f: F) -> Option<U>
    where
        F: [const] FnOnce(T) -> U + [const] Destruct,
    {
        match self { Some(value) => Some(f(value)), None => None }
    }

    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, f: F) -> U {
        match self { Some(value) => f(value), None => default }
    }

    pub fn and_then<U, F: FnOnce(T) -> Option<U>>(self, f: F) -> Option<U> {
        match self { Some(value) => f(value), None => None }
    }

    pub const fn ok_or<E: [const] Destruct>(self, err: E) -> Result<T, E> {
        match self { Some(value) => Ok(value), None => Err(err) }
    }

    pub fn ok_or_else<E, F: FnOnce() -> E>(self, err: F) -> Result<T, E> {
        match self { Some(value) => Ok(value), None => Err(err()) }
    }

    pub fn or(self, optb: Option<T>) -> Option<T> {
        match self { Some(x) => Some(x), None => optb }
    }

    pub fn or_else<F: FnOnce() -> Option<T>>(self, f: F) -> Option<T> {
        match self { Some(x) => Some(x), None => f() }
    }

    pub fn is_some_and<F: FnOnce(T) -> bool>(self, f: F) -> bool {
        match self { Some(x) => f(x), None => false }
    }
}

impl<'a, T> Option<&'a T> {
    pub const fn copied(self) -> Option<T>
    where
        T: Copy,
    {
        match self { Some(&t) => Some(t), None => None }
    }

    pub fn cloned(self) -> Option<T>
    where
        T: Clone,
    {
        match self { Some(t) => Some(t.clone()), None => None }
    }
}

impl<'a, T> Option<&'a mut T> {
    pub const fn copied(self) -> Option<T>
    where
        T: Copy,
    {
        match self { Some(&mut t) => Some(t), None => None }
    }

    pub fn cloned(self) -> Option<T>
    where
        T: Clone,
    {
        match self { Some(t) => Some(t.clone()), None => None }
    }
}

impl<T: Clone> Clone for Option<T> {
    fn clone(&self) -> Self {
        match self { Some(value) => Some(value.clone()), None => None }
    }
}
impl<T: Copy> Copy for Option<T> {}

const impl<T> Default for Option<T> {
    fn default() -> Self { None }
}

const impl<T> Try for Option<T> {
    type Output = T;
    type Residual = Option<Infallible>;

    fn from_output(output: T) -> Self { Some(output) }

    fn branch(self) -> ControlFlow<Self::Residual, T> {
        match self {
            Some(value) => ControlFlow::Continue(value),
            None => ControlFlow::Break(None),
        }
    }
}

const impl<T> FromResidual<Option<Infallible>> for Option<T> {
    fn from_residual(residual: Option<Infallible>) -> Self {
        match residual { None => None, Some(never) => match never {} }
    }
}
