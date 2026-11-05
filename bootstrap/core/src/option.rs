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

    pub const fn unwrap_or(self, default: T) -> T
    where
        T: [const] Destruct,
    {
        match self { Some(value) => value, None => default }
    }

    pub const fn map<U, F>(self, f: F) -> Option<U>
    where
        F: [const] FnOnce(T) -> U + [const] Destruct,
    {
        match self { Some(value) => Some(f(value)), None => None }
    }

    pub const fn ok_or<E: [const] Destruct>(self, err: E) -> Result<T, E> {
        match self { Some(value) => Ok(value), None => Err(err) }
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
