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
