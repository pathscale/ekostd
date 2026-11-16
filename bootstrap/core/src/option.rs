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

    /// The value, without checking. `None` here is undefined behaviour.
    pub unsafe fn unwrap_unchecked(self) -> T {
        match self { Some(value) => value, None => unsafe { crate::hint::unreachable_unchecked() } }
    }

    /// Moves the value out, leaving `None`.
    pub fn take(&mut self) -> Option<T> { crate::mem::replace(self, None) }

    pub const fn as_mut(&mut self) -> Option<&mut T> {
        match self { Some(value) => Some(value), None => None }
    }

    #[track_caller]
    pub const fn unwrap(self) -> T
    {
        match self {
            Some(value) => value,
            None => crate::panic::panic_fmt(crate::fmt::Arguments::from_str("called `Option::unwrap()` on a `None` value")),
        }
    }

    #[track_caller]
    pub const fn expect(self, msg: &str) -> T
    {
        match self {
            Some(value) => value,
            None => crate::panic::panic_display(&msg),
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

    pub fn filter<P: FnOnce(&T) -> bool>(self, predicate: P) -> Option<T> {
        match self { Some(x) if predicate(&x) => Some(x), _ => None }
    }

    pub fn is_none_or<F: FnOnce(T) -> bool>(self, f: F) -> bool {
        match self { Some(x) => f(x), None => true }
    }

    pub fn unwrap_or_default(self) -> T where T: Default {
        match self { Some(x) => x, None => T::default() }
    }

    pub fn map_or_else<U, D: FnOnce() -> U, F: FnOnce(T) -> U>(self, default: D, f: F) -> U {
        match self { Some(x) => f(x), None => default() }
    }

    /// Puts `value` in when empty, and lends what is then inside.
    pub fn get_or_insert(&mut self, value: T) -> &mut T {
        self.get_or_insert_with(|| value)
    }

    /// Puts the result of `f` in when empty, and lends what is then inside.
    pub fn get_or_insert_with<F: FnOnce() -> T>(&mut self, f: F) -> &mut T {
        if self.is_none() { *self = Some(f()); }
        match self { Some(x) => x, None => unsafe { crate::hint::unreachable_unchecked() } }
    }

    /// Puts the default in when empty, and lends what is then inside.
    pub fn get_or_insert_default(&mut self) -> &mut T where T: crate::default::Default {
        self.get_or_insert_with(T::default)
    }

    /// Puts `value` in, dropping what was there, and lends it.
    pub fn insert(&mut self, value: T) -> &mut T {
        *self = Some(value);
        match self { Some(x) => x, None => unsafe { crate::hint::unreachable_unchecked() } }
    }

    /// Puts `value` in and returns what was there.
    pub fn replace(&mut self, value: T) -> Option<T> { crate::mem::replace(self, Some(value)) }

    pub fn is_some_and<F: FnOnce(T) -> bool>(self, f: F) -> bool {
        match self { Some(x) => f(x), None => false }
    }

    pub fn zip<U>(self, other: Option<U>) -> Option<(T, U)> {
        match (self, other) {
            (Some(a), Some(b)) => Some((a, b)),
            _ => None,
        }
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

impl<T: crate::ops::Deref> Option<T> {
    pub fn as_deref(&self) -> Option<&T::Target> {
        match self { Some(x) => Some(&**x), None => None }
    }
}

impl<T: crate::ops::DerefMut> Option<T> {
    pub fn as_deref_mut(&mut self) -> Option<&mut T::Target> {
        match self { Some(x) => Some(&mut **x), None => None }
    }
}

impl<T> Option<Option<T>> {
    pub fn flatten(self) -> Option<T> {
        match self { Some(inner) => inner, None => None }
    }
}

impl<T, E> Option<Result<T, E>> {
    pub fn transpose(self) -> Result<Option<T>, E> {
        match self { Some(Ok(x)) => Ok(Some(x)), Some(Err(e)) => Err(e), None => Ok(None) }
    }
}

/// The iterator `Option::into_iter` returns.
pub type IntoIter<A> = crate::iter::Once<A>;

impl<T> Option<T> {
    pub fn as_slice(&self) -> &[T] {
        match self {
            Some(value) => crate::slice::from_ref(value),
            None => &[],
        }
    }
}

impl<T> Option<T> {
    pub fn inspect<F: crate::ops::FnOnce(&T)>(self, f: F) -> Self {
        if let Some(value) = &self { f(value); }
        self
    }
}

impl<T> From<T> for Option<T> {
    fn from(value: T) -> Option<T> { Some(value) }
}
