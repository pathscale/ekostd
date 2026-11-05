//! Hygienic bootstrap macros and compiler-provided expansions.
#[macro_export]
#[allow_internal_unstable(core_intrinsics, fmt_internals)]
macro_rules! panic {
    () => { $crate::panic::panic_fmt($crate::fmt::Arguments::from_str("explicit panic")) };
    ($($arg:tt)+) => { $crate::panic::panic_fmt($crate::format_args!($($arg)+)) };
}
#[macro_export]
#[allow_internal_unstable(core_intrinsics, fmt_internals)]
macro_rules! assert {
    ($cond:expr $(,)?) => { if !$cond { $crate::panic!("assertion failed: {}", $crate::stringify!($cond)); } };
    ($cond:expr, $($arg:tt)+) => { if !$cond { $crate::panic!($($arg)+); } };
}
#[macro_export]
#[allow_internal_unstable(core_intrinsics, fmt_internals)]
macro_rules! assert_eq {
    ($left:expr, $right:expr $(,)?) => {
        match (&$left, &$right) { (left, right) => { if !(*left == *right) { $crate::panic!("assertion `left == right` failed\n  left: {:?}\n right: {:?}", &*left, &*right); } } }
    };
    ($left:expr, $right:expr, $($arg:tt)+) => {
        match (&$left, &$right) { (left, right) => { if !(*left == *right) { $crate::panic!("assertion `left == right` failed: {}\n  left: {:?}\n right: {:?}", $crate::format_args!($($arg)+), &*left, &*right); } } }
    };
}
#[macro_export]
#[allow_internal_unstable(core_intrinsics, fmt_internals)]
macro_rules! assert_ne {
    ($left:expr, $right:expr $(,)?) => {
        match (&$left, &$right) { (left, right) => { if *left == *right { $crate::panic!("assertion `left != right` failed\n  left: {:?}\n right: {:?}", &*left, &*right); } } }
    };
    ($left:expr, $right:expr, $($arg:tt)+) => {
        match (&$left, &$right) { (left, right) => { if *left == *right { $crate::panic!("assertion `left != right` failed: {}\n  left: {:?}\n right: {:?}", $crate::format_args!($($arg)+), &*left, &*right); } } }
    };
}
#[macro_export]
#[allow_internal_unstable(core_intrinsics, fmt_internals)]
macro_rules! debug_assert { ($($arg:tt)*) => { if $crate::cfg!(debug_assertions) { $crate::assert!($($arg)*); } }; }
#[macro_export]
#[allow_internal_unstable(core_intrinsics, fmt_internals)]
macro_rules! debug_assert_eq { ($($arg:tt)*) => { if $crate::cfg!(debug_assertions) { $crate::assert_eq!($($arg)*); } }; }
#[macro_export]
#[allow_internal_unstable(core_intrinsics, fmt_internals)]
macro_rules! debug_assert_ne { ($($arg:tt)*) => { if $crate::cfg!(debug_assertions) { $crate::assert_ne!($($arg)*); } }; }
#[macro_export]
#[allow_internal_unstable(core_intrinsics, fmt_internals)]
macro_rules! unreachable {
    () => { $crate::panic!("internal error: entered unreachable code") };
    ($($arg:tt)+) => { $crate::panic!("internal error: entered unreachable code: {}", $crate::format_args!($($arg)+)) };
}
#[macro_export]
#[allow_internal_unstable(core_intrinsics, fmt_internals)]
macro_rules! unimplemented {
    () => { $crate::panic!("not implemented") };
    ($($arg:tt)+) => { $crate::panic!("not implemented: {}", $crate::format_args!($($arg)+)) };
}
#[macro_export]
#[allow_internal_unstable(core_intrinsics, fmt_internals)]
macro_rules! todo {
    () => { $crate::panic!("not yet implemented") };
    ($($arg:tt)+) => { $crate::panic!("not yet implemented: {}", $crate::format_args!($($arg)+)) };
}
#[macro_export]
#[allow_internal_unstable(core_intrinsics, fmt_internals)]
macro_rules! write { ($dst:expr, $($arg:tt)*) => { ($dst).write_fmt($crate::format_args!($($arg)*)) }; }
#[macro_export]
#[allow_internal_unstable(core_intrinsics, fmt_internals)]
macro_rules! writeln {
    ($dst:expr $(,)?) => { ($dst).write_fmt($crate::format_args!("\n")) };
    ($dst:expr, $($arg:tt)*) => { ($dst).write_fmt($crate::format_args!("{}\n", $crate::format_args!($($arg)*))) };
}
#[macro_export]
#[allow_internal_unstable(core_intrinsics, fmt_internals)]
macro_rules! matches {
    ($expression:expr, $pattern:pat $(if $guard:expr)? $(,)?) => { match $expression { $pattern $(if $guard)? => true, _ => false } };
}
#[rustc_builtin_macro]
pub macro format_args($fmt:expr, $($args:tt)*) {}
#[rustc_builtin_macro]
pub macro cfg($($tokens:tt)*) {}
#[rustc_builtin_macro]
pub macro env($($tokens:tt)*) {}
#[rustc_builtin_macro]
pub macro option_env($($tokens:tt)*) {}
#[rustc_builtin_macro]
pub macro line($($tokens:tt)*) {}
#[rustc_builtin_macro]
pub macro column($($tokens:tt)*) {}
#[rustc_builtin_macro]
pub macro file($($tokens:tt)*) {}
#[rustc_builtin_macro]
pub macro stringify($($tokens:tt)*) {}
#[rustc_builtin_macro]
pub macro concat($($tokens:tt)*) {}
#[rustc_builtin_macro]
pub macro include_str($($tokens:tt)*) {}
#[rustc_builtin_macro]
pub macro include_bytes($($tokens:tt)*) {}
#[rustc_builtin_macro]
pub macro derive($($tokens:tt)*) {}

pub mod builtin {
    #[rustc_builtin_macro(Copy)]
    #[allow_internal_unstable(core_intrinsics, derive_clone_copy)]
    pub macro Copy($($tokens:tt)*) {}
    #[rustc_builtin_macro(Clone)]
    #[allow_internal_unstable(core_intrinsics, derive_clone_copy)]
    pub macro Clone($($tokens:tt)*) {}
    #[rustc_builtin_macro(Debug)]
    #[allow_internal_unstable(fmt_internals)]
    pub macro Debug($($tokens:tt)*) {}
    #[rustc_builtin_macro(Default)]
    pub macro Default($($tokens:tt)*) {}
    #[rustc_builtin_macro(Eq)]
    #[allow_internal_unstable(core_intrinsics, derive_eq, coverage_attribute)]
    pub macro Eq($($tokens:tt)*) {}
    #[rustc_builtin_macro(PartialEq)]
    #[allow_internal_unstable(core_intrinsics, structural_match)]
    pub macro PartialEq($($tokens:tt)*) {}
    #[rustc_builtin_macro(Ord)]
    #[allow_internal_unstable(core_intrinsics)]
    pub macro Ord($($tokens:tt)*) {}
    #[rustc_builtin_macro(PartialOrd)]
    #[allow_internal_unstable(core_intrinsics)]
    pub macro PartialOrd($($tokens:tt)*) {}
    #[rustc_builtin_macro(Hash)]
    #[allow_internal_unstable(core_intrinsics)]
    pub macro Hash($($tokens:tt)*) {}
}
