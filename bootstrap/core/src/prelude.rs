pub mod v1 {
    pub use crate::clone::Clone;
    pub use crate::cmp::{Eq, Ord, PartialEq, PartialOrd};
    pub use crate::convert::{From, Into};
    pub use crate::default::Default;
    pub use crate::marker::{Copy, Send, Sized, Sync, Unpin};
    pub use crate::ops::{Drop, Fn, FnMut, FnOnce};
    pub use crate::option::Option::{self, None, Some};
    pub use crate::result::Result::{self, Err, Ok};
}

pub mod rust_2015 { pub use super::v1::*; }
pub mod rust_2018 { pub use super::v1::*; }
pub mod rust_2021 { pub use super::v1::*; }
pub mod rust_2024 { pub use super::v1::*; }
