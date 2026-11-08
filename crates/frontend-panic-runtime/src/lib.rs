#![no_std]

#[macro_use]
extern crate core;

unsafe extern "C" {
    fn write(fd: i32, buf: *const u8, count: usize) -> isize;
    fn abort() -> !;
    #[link_name = "abort"]
    fn abort_trap();
}

// Match the Itanium personality ABI used by supported Darwin and Linux targets.
// This link-compatibility trap intentionally does not inspect unwinder state.
#[unsafe(no_mangle)]
pub extern "C" fn rust_eh_personality(
    _version: i32,
    _actions: i32,
    _exception_class: u64,
    _exception_object: *mut core::ffi::c_void,
    _context: *mut core::ffi::c_void,
) -> i32 {
    unsafe { abort_trap(); }
    5
}

// C spelling _Unwind_Resume exports as __Unwind_Resume on Mach-O.
// This link-compatibility trap never inspects or resumes the exception.
#[unsafe(no_mangle)]
pub extern "C" fn _Unwind_Resume(_exception: *mut core::ffi::c_void) -> ! {
    unsafe { abort() }
}

fn write_str(text: &str) {
    unsafe {
        let _ = write(2, text.as_ptr(), text.len());
    }
}

#[panic_handler]
fn panic_handler(_info: &core::panic::PanicInfo<'_>) -> ! {
    write_str("panicked\n");
    unsafe { abort() }
}
