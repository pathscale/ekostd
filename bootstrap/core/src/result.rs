use crate::clone::Clone;
use crate::convert::{From, Infallible};
use crate::marker::{Copy, Destruct};
use crate::ops::{ControlFlow, FnOnce, FromResidual, Try};
use crate::option::Option::{self, None, Some};

#[must_use]
#[rustc_diagnostic_item = "Result"]
pub enum Result<T, E> {
    #[lang = "Ok"]
    Ok(T),
    #[lang = "Err"]
    Err(E),
}
use self::Result::{Err, Ok};

impl<T, E> Result<T, E> {
    pub const fn is_ok(&self) -> bool {
        match self { Ok(_) => true, Err(_) => false }
    }

    pub const fn is_err(&self) -> bool {
        match self { Ok(_) => false, Err(_) => true }
    }

    pub const fn as_ref(&self) -> Result<&T, &E> {
        match self { Ok(value) => Ok(value), Err(err) => Err(err) }
    }

    pub const fn as_mut(&mut self) -> Result<&mut T, &mut E> {
        match self { Ok(value) => Ok(value), Err(err) => Err(err) }
    }

    pub const fn ok(self) -> Option<T>
    where
        T: [const] Destruct,
        E: [const] Destruct,
    {
        match self { Ok(value) => Some(value), Err(_) => None }
    }

    pub const fn err(self) -> Option<E>
    where
        T: [const] Destruct,
        E: [const] Destruct,
    {
        match self { Ok(_) => None, Err(err) => Some(err) }
    }

    #[track_caller]
    /// The value, without checking. `Err` here is undefined behaviour.
    pub unsafe fn unwrap_unchecked(self) -> T {
        match self { Ok(value) => value, Err(_) => unsafe { crate::hint::unreachable_unchecked() } }
    }

    pub fn unwrap(self) -> T {
        match self {
            Ok(value) => value,
            Err(_) => crate::panic::panic_nounwind("called `Result::unwrap()` on an `Err` value"),
        }
    }

    #[track_caller]
    pub fn expect(self, msg: &str) -> T {
        match self {
            Ok(value) => value,
            Err(_) => {
                let _ = msg;
                crate::panic::panic_nounwind("called `Result::expect()` on an `Err` value")
            }
        }
    }

    pub fn unwrap_or(self, default: T) -> T {
        match self { Ok(value) => value, Err(_) => default }
    }

    pub fn unwrap_or_else<F: FnOnce(E) -> T>(self, op: F) -> T {
        match self { Ok(value) => value, Err(err) => op(err) }
    }

    pub fn unwrap_or_default(self) -> T where T: crate::default::Default {
        match self { Ok(x) => x, Err(_) => T::default() }
    }

    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, f: F) -> U {
        match self { Ok(x) => f(x), Err(_) => default }
    }

    pub fn map_or_else<U, D: FnOnce(E) -> U, F: FnOnce(T) -> U>(self, default: D, f: F) -> U {
        match self { Ok(x) => f(x), Err(e) => default(e) }
    }

    pub fn is_ok_and<F: FnOnce(T) -> bool>(self, f: F) -> bool {
        match self { Ok(x) => f(x), Err(_) => false }
    }

    pub fn and_then<U, F: FnOnce(T) -> Result<U, E>>(self, op: F) -> Result<U, E> {
        match self { Ok(value) => op(value), Err(err) => Err(err) }
    }

    pub fn or<F>(self, res: Result<T, F>) -> Result<T, F> {
        match self { Ok(value) => Ok(value), Err(_) => res }
    }

    pub fn or_else<F, O: FnOnce(E) -> Result<T, F>>(self, op: O) -> Result<T, F> {
        match self { Ok(value) => Ok(value), Err(err) => op(err) }
    }

    pub const fn map<U, F>(self, f: F) -> Result<U, E>
    where
        F: [const] FnOnce(T) -> U + [const] Destruct,
    {
        match self { Ok(value) => Ok(f(value)), Err(err) => Err(err) }
    }

    pub const fn map_err<F, O>(self, op: O) -> Result<T, F>
    where
        O: [const] FnOnce(E) -> F + [const] Destruct,
    {
        match self { Ok(value) => Ok(value), Err(err) => Err(op(err)) }
    }
}

impl<T: Clone, E: Clone> Clone for Result<T, E> {
    fn clone(&self) -> Self {
        match self {
            Ok(value) => Ok(value.clone()),
            Err(err) => Err(err.clone()),
        }
    }
}
impl<T: Copy, E: Copy> Copy for Result<T, E> {}

const impl<T, E> Try for Result<T, E> {
    type Output = T;
    type Residual = Result<Infallible, E>;

    fn from_output(output: T) -> Self { Ok(output) }

    fn branch(self) -> ControlFlow<Self::Residual, T> {
        match self {
            Ok(value) => ControlFlow::Continue(value),
            Err(err) => ControlFlow::Break(Err(err)),
        }
    }
}

const impl<T, E, F: [const] From<E>> FromResidual<Result<Infallible, E>> for Result<T, F> {
    fn from_residual(residual: Result<Infallible, E>) -> Self {
        match residual {
            Err(err) => Err(F::from(err)),
            Ok(never) => match never {},
        }
    }
}

impl<T, E> Result<&T, E> {
    pub const fn copied(self) -> Result<T, E>
    where
        T: Copy,
    {
        match self { Ok(&t) => Ok(t), Err(err) => Err(err) }
    }

    pub fn cloned(self) -> Result<T, E>
    where
        T: Clone,
    {
        match self { Ok(t) => Ok(t.clone()), Err(err) => Err(err) }
    }
}

impl<T, E> Result<&mut T, E> {
    pub const fn copied(self) -> Result<T, E>
    where
        T: Copy,
    {
        match self { Ok(&mut t) => Ok(t), Err(err) => Err(err) }
    }

    pub fn cloned(self) -> Result<T, E>
    where
        T: Clone,
    {
        match self { Ok(t) => Ok(t.clone()), Err(err) => Err(err) }
    }
}

impl<T, E> Result<Result<T, E>, E> {
    pub fn flatten(self) -> Result<T, E> {
        match self { Ok(inner) => inner, Err(err) => Err(err) }
    }
}

impl<T: crate::ops::Deref, E> Result<T, E> {
    pub fn as_deref(&self) -> Result<&T::Target, &E> {
        match self { Ok(t) => Ok(t.deref()), Err(e) => Err(e) }
    }
}

impl<T, E> Result<Option<T>, E> {
    /// `Ok(None)` becomes `None`; `Ok(Some(v))` and `Err(e)` become `Some`.
    pub fn transpose(self) -> Option<Result<T, E>> {
        match self {
            Ok(Some(value)) => Some(Ok(value)),
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        }
    }
}

impl<T, E> Result<T, E> {
    #[track_caller]
    pub fn unwrap_err(self) -> E
    where
        T: crate::fmt::Debug,
    {
        match self {
            Ok(_) => crate::panic::panic_nounwind("called `Result::unwrap_err()` on an `Ok` value"),
            Err(error) => error,
        }
    }
}

impl<T, E> Result<T, E> {
    pub fn inspect<F: FnOnce(&T)>(self, f: F) -> Self {
        if let Ok(value) = &self { f(value); }
        self
    }
    #[track_caller]
    pub fn expect_err(self, msg: &str) -> E where T: crate::fmt::Debug {
        match self {
            Ok(_) => { let _ = msg; crate::panic::panic_nounwind("called `Result::expect_err()` on an `Ok` value") }
            Err(error) => error,
        }
    }
}

impl<T, E> Result<T, E> {
    pub fn inspect_err<F: FnOnce(&E)>(self, f: F) -> Self {
        if let Err(error) = &self { f(error); }
        self
    }
}
