use crate::clone::Clone;
use crate::marker::{Copy, Sized};

pub const trait From<T>: Sized {
    #[lang = "from"]
    fn from(value: T) -> Self;
}

pub const trait Into<T>: Sized {
    fn into(self) -> T;
}

const impl<T> From<T> for T {
    fn from(value: T) -> T { value }
}

const impl<T, U> Into<U> for T
where
    U: [const] From<T>,
{
    fn into(self) -> U { U::from(self) }
}

pub enum Infallible {}

impl Copy for Infallible {}
const impl Clone for Infallible {
    fn clone(&self) -> Self { match *self {} }
}
