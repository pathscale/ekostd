pub mod v1 {
    pub use crate::clone::Clone;
    pub use crate::cmp::{Eq, Ord, PartialEq, PartialOrd};
    pub use crate::convert::{AsRef, AsMut, From, Into, TryFrom, TryInto};
    pub use crate::iter::{Iterator, IntoIterator, FromIterator, Extend, DoubleEndedIterator, ExactSizeIterator};
    pub use crate::mem::{drop, size_of, size_of_val, align_of, align_of_val};
    pub use crate::default::Default;
    pub use crate::marker::{Copy, Send, Sized, Sync, Unpin};
    pub use crate::ops::{Drop, Fn, FnMut, FnOnce};
    pub use crate::option::Option::{self, None, Some};
    pub use crate::result::Result::{self, Err, Ok};
    pub use crate::{
        assert, assert_eq, assert_ne, cfg, column, compile_error, concat, debug_assert, debug_assert_eq,
        debug_assert_ne, derive, env, file, format_args, include_bytes, include_str, line, matches,
        option_env, panic, stringify, todo, unimplemented, unreachable, write, writeln,
    };
    pub use crate::macros::builtin::{
        Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd,
    };
}

pub mod rust_2015 { pub use super::v1::*; }
pub mod rust_2018 { pub use super::v1::*; }
pub mod rust_2021 { pub use super::v1::*; }
pub mod rust_2024 { pub use super::v1::*; }
