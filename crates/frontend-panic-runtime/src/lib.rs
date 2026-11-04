#![no_std]

use core::fmt;
use core::sync::atomic::{AtomicBool, Ordering};

static PANICKING: AtomicBool = AtomicBool::new(false);

unsafe extern "C" {
    fn write(fd: i32, buf: *const u8, count: usize) -> isize;
    fn abort() -> !;
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
    unsafe { abort() }
}

struct Stderr;

impl fmt::Write for Stderr {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        // A failed or short write cannot be reported safely while handling a panic.
        unsafe {
            let _ = write(2, text.as_ptr(), text.len());
        }
        Ok(())
    }
}

fn write_fmt(args: fmt::Arguments<'_>) {
    let _ = fmt::write(&mut Stderr, args);
}

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo<'_>) -> ! {
    if PANICKING.swap(true, Ordering::Relaxed) {
        unsafe { abort() }
    }
    if let Some(location) = info.location() {
        write_fmt(format_args!("panicked at {}:{}: ", location.file(), location.line()));
    } else {
        write_fmt(format_args!("panicked: "));
    }
    write_fmt(format_args!("{}\n", info.message()));
    unsafe {
        abort()
    }
}
