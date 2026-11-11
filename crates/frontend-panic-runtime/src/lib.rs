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

fn write_u32(mut value: u32) {
    let mut buf = [0u8; 10];
    let mut idx = buf.len();
    if value == 0 {
        write_str("0");
        return;
    }
    while value > 0 {
        idx -= 1;
        buf[idx] = b'0' + (value % 10) as u8;
        value /= 10;
    }
    if let Ok(text) = core::str::from_utf8(&buf[idx..]) {
        write_str(text);
    }
}

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo<'_>) -> ! {
    if let Some(loc) = info.location() {
        write_str("panicked at ");
        write_str(loc.file());
        write_str(":");
        write_u32(loc.line());
        write_str(":");
        write_u32(loc.column());
        write_str("\n");
    } else {
        write_str("panicked\n");
    }
    #[cfg(feature = "diagnostics")]
    diagnostics::report(info);
    unsafe { abort() }
}

/// What a host-built compiler prints before it aborts, beyond the location: the message, and a raw
/// backtrace. Behind a feature because this crate is also a unit the compiler compiles for itself,
/// against a `core` that has no formatting machinery to spare.
#[cfg(feature = "diagnostics")]
mod diagnostics {
    use super::{write_str, write_u32};

    struct Stderr;
    impl core::fmt::Write for Stderr {
        fn write_str(&mut self, text: &str) -> core::fmt::Result {
            write_str(text);
            Ok(())
        }
    }

    fn write_hex(mut value: usize) {
        let mut buf = [b'0'; 18];
        buf[1] = b'x';
        let mut idx = buf.len();
        while idx > 2 {
            idx -= 1;
            buf[idx] = b"0123456789abcdef"[value & 15];
            value >>= 4;
        }
        if let Ok(text) = core::str::from_utf8(&buf) {
            write_str(text);
        }
    }

    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    unsafe extern "C" {
        fn _dyld_get_image_vmaddr_slide(index: u32) -> isize;
    }

    pub fn report(info: &core::panic::PanicInfo<'_>) {
        // The message says what was violated; the location alone does not.
        let _ = core::fmt::write(&mut Stderr, format_args!("{}\n", info.message()));
        // The return addresses up the frame-pointer chain, raw, with the image slide: what
        // `atos -o <binary> -s <slide> <addresses>` needs to name every frame. No allocation, and
        // no unwinder: the frame records are read directly, which Darwin arm64 always keeps.
        #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
        unsafe {
            write_str("image slide ");
            write_hex(_dyld_get_image_vmaddr_slide(0) as usize);
            write_str("\n");
            let mut frame: *const usize;
            core::arch::asm!("mov {}, x29", out(reg) frame, options(nomem, nostack, preserves_flags));
            let mut index = 0u32;
            while !frame.is_null() && (frame as usize) % 8 == 0 && index < 96 {
                let next = *frame as *const usize;
                let ret = *frame.add(1);
                if ret == 0 {
                    break;
                }
                write_str("frame ");
                write_u32(index);
                write_str(" ");
                write_hex(ret);
                write_str("\n");
                if next <= frame {
                    break;
                }
                frame = next;
                index += 1;
            }
        }
    }
}
