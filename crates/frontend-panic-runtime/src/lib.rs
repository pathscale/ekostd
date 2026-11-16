#![no_std]

#[macro_use]
extern crate core;

// The one `#[panic_handler]` of the binaries: report, then abort. Every unit is compiled with
// `panic = "abort"`, so there is no personality routine and no unwinder to provide.
unsafe extern "C" {
    fn write(fd: i32, buf: *const u8, count: usize) -> isize;
    fn abort() -> !;
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
            walk(frame);
        }
    }

    /// Prints the return addresses up a frame-pointer chain, one `frame N 0x...` line each.
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    unsafe fn walk(mut frame: *const usize) {
        let mut index = 0u32;
        while !frame.is_null() && (frame as usize) % 8 == 0 && index < 96 {
            let next = unsafe { *frame } as *const usize;
            let ret = unsafe { *frame.add(1) };
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

    // SIGQUIT frame dump. A daemon that stops making progress has no debugger attached, and the
    // only thing it can say about itself is where it is. `kill -QUIT <pid>` makes the thread that
    // receives the signal print its program counter, link register and the frame-pointer chain
    // from the interrupted context (not the handler's own frames), then abort. The addresses are
    // raw; with the `image slide` line they symbolize against `nm -n` of the same binary (or
    // `atos -o <binary> -s <slide>`). The handler uses only `write`, which is async-signal-safe,
    // and no allocation. It is installed before `main` runs, from a Mach-O initializer, so no
    // caller has to remember to ask for it.
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    mod quit {
        use super::{_dyld_get_image_vmaddr_slide, walk, write_hex, write_str};

        const SIGQUIT: i32 = 3;
        const SA_SIGINFO: i32 = 0x40;

        /// Darwin `struct sigaction`: handler, mask, flags.
        #[repr(C)]
        struct SigAction {
            handler: extern "C" fn(i32, *const u8, *const u8),
            mask: u32,
            flags: i32,
        }

        unsafe extern "C" {
            fn sigaction(sig: i32, act: *const SigAction, old: *mut SigAction) -> i32;
            fn abort() -> !;
        }

        extern "C" fn on_quit(_sig: i32, _info: *const u8, ucontext: *const u8) {
            unsafe {
                // ucontext_t: onstack u32, sigmask u32, stack_t (24 bytes), link ptr, mcsize u64,
                // then the mcontext pointer at offset 48. mcontext64: exception state (16 bytes),
                // then thread state: x0..x28 (232 bytes), fp, lr, sp, pc.
                let mctx = *(ucontext.add(48) as *const *const u8);
                let state = mctx.add(16) as *const usize;
                let (fp, lr, sp, pc) = (*state.add(29), *state.add(30), *state.add(31), *state.add(32));
                write_str("SIGQUIT: frames of the interrupted thread\nimage slide ");
                write_hex(_dyld_get_image_vmaddr_slide(0) as usize);
                write_str("\npc ");
                write_hex(pc);
                write_str("\nlr ");
                write_hex(lr);
                write_str("\nsp ");
                write_hex(sp);
                write_str("\n");
                walk(fp as *const usize);
                abort()
            }
        }

        extern "C" fn install() {
            let act = SigAction { handler: on_quit, mask: 0, flags: SA_SIGINFO };
            unsafe {
                sigaction(SIGQUIT, &act, core::ptr::null_mut());
            }
        }

        #[used]
        #[unsafe(link_section = "__DATA,__mod_init_func")]
        static INSTALL: extern "C" fn() = install;
    }
}
