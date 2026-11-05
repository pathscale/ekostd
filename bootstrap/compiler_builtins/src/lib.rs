#![no_core]
#![compiler_builtins]
#![feature(no_core, compiler_builtins, lang_items, intrinsics, rustc_attrs)]
#![crate_name = "compiler_builtins"]
#![crate_type = "rlib"]

extern crate core;

use core::intrinsics::{arith_offset, volatile_load, volatile_store};
use core::marker::Copy;

#[rustc_intrinsic]
const fn wrapping_add<T: Copy>(a: T, b: T) -> T;
#[rustc_intrinsic]
const fn wrapping_sub<T: Copy>(a: T, b: T) -> T;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcpy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0usize;
    while i < n {
        unsafe {
            let byte = volatile_load(arith_offset(src, i as isize));
            volatile_store(arith_offset(dst, i as isize) as *mut u8, byte);
        }
        i = wrapping_add(i, 1);
    }
    dst
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memmove(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if (dst as usize) <= (src as usize) {
        return unsafe { memcpy(dst, src, n) };
    }
    let mut i = n;
    while i != 0 {
        i = wrapping_sub(i, 1);
        unsafe {
            let byte = volatile_load(arith_offset(src, i as isize));
            volatile_store(arith_offset(dst, i as isize) as *mut u8, byte);
        }
    }
    dst
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memset(dst: *mut u8, value: i32, n: usize) -> *mut u8 {
    let mut i = 0usize;
    while i < n {
        unsafe { volatile_store(arith_offset(dst, i as isize) as *mut u8, value as u8) };
        i = wrapping_add(i, 1);
    }
    dst
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    let mut i = 0usize;
    while i < n {
        let left = unsafe { volatile_load(arith_offset(a, i as isize)) };
        let right = unsafe { volatile_load(arith_offset(b, i as isize)) };
        if left != right { return wrapping_sub(left as i32, right as i32); }
        i = wrapping_add(i, 1);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    unsafe { memcmp(a, b, n) }
}
