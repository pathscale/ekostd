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
    write_bytes(text.as_bytes());
}

fn write_bytes(bytes: &[u8]) {
    unsafe {
        let _ = write(2, bytes.as_ptr(), bytes.len());
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
    use super::{write_bytes, write_str, write_u32};

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

    /// `value` in hex without leading zeros: the offset after a symbol name.
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    fn write_offset(mut value: usize) {
        let mut buf = [0u8; 18];
        let mut idx = buf.len();
        loop {
            idx -= 1;
            buf[idx] = b"0123456789abcdef"[value & 15];
            value >>= 4;
            if value == 0 {
                break;
            }
        }
        write_str("0x");
        write_bytes(&buf[idx..]);
    }

    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    unsafe extern "C" {
        fn _dyld_get_image_vmaddr_slide(index: u32) -> isize;
        fn _dyld_get_image_header(index: u32) -> *const u8;
    }

    /// One line: `addr` in hex and, when it lies in this executable's `__TEXT`, the nearest symbol
    /// at or below it plus the offset (`0x... _name+0x1c`). The names come from the running
    /// image's own symbol table, so a dump names its frames without any external tool.
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    fn write_addr(addr: usize) {
        write_hex(addr);
        if let Some(table) = unsafe { Symtab::of_main_image() } {
            table.write_name(addr);
        }
        write_str("\n");
    }

    /// The executable's `LC_SYMTAB` as dyld mapped it (inside `__LINKEDIT`), found through the
    /// Mach-O header of image 0. Read in place: nothing is copied or allocated, so it is usable
    /// from the SIGQUIT handler.
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    struct Symtab {
        slide: usize,
        /// Unslid `__TEXT` range: only addresses inside it are named; anything else is in another image.
        text: (u64, u64),
        symbols: *const u8,
        count: usize,
        strings: *const u8,
        strings_len: usize,
    }

    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    impl Symtab {
        const MH_MAGIC_64: u32 = 0xfeed_facf;
        const MACH_HEADER_64: usize = 32;
        const LC_SYMTAB: u32 = 0x2;
        const LC_SEGMENT_64: u32 = 0x19;
        /// `nlist_64`: n_strx u32, n_type u8, n_sect u8, n_desc u16, n_value u64.
        const NLIST_64: usize = 16;
        const N_STAB: u8 = 0xe0;
        const N_TYPE: u8 = 0x0e;
        const N_SECT: u8 = 0x0e;

        unsafe fn u32_at(at: *const u8) -> u32 {
            unsafe { core::ptr::read_unaligned(at as *const u32) }
        }

        unsafe fn u64_at(at: *const u8) -> u64 {
            unsafe { core::ptr::read_unaligned(at as *const u64) }
        }

        unsafe fn of_main_image() -> Option<Symtab> {
            unsafe {
                let header = _dyld_get_image_header(0);
                if header.is_null() || Self::u32_at(header) != Self::MH_MAGIC_64 {
                    return None;
                }
                let slide = _dyld_get_image_vmaddr_slide(0) as usize;
                let ncmds = Self::u32_at(header.add(16));
                let sizeofcmds = Self::u32_at(header.add(20)) as usize;
                let mut offset = 0usize;
                let (mut text, mut linkedit, mut symtab) = (None, None, None);
                for _ in 0..ncmds {
                    if offset + 8 > sizeofcmds {
                        return None;
                    }
                    let command = header.add(Self::MACH_HEADER_64 + offset);
                    let (kind, size) = (Self::u32_at(command), Self::u32_at(command.add(4)) as usize);
                    if size < 8 || offset + size > sizeofcmds {
                        return None;
                    }
                    if kind == Self::LC_SEGMENT_64 && size >= 72 {
                        // segment_command_64: segname[16] at 8, vmaddr at 24, vmsize at 32, fileoff at 40.
                        let name = core::slice::from_raw_parts(command.add(8), 16);
                        let (vmaddr, vmsize, fileoff) = (Self::u64_at(command.add(24)), Self::u64_at(command.add(32)), Self::u64_at(command.add(40)));
                        if name.starts_with(b"__TEXT\0") {
                            text = Some((vmaddr, vmaddr.wrapping_add(vmsize)));
                        } else if name.starts_with(b"__LINKEDIT\0") {
                            linkedit = Some((vmaddr, fileoff));
                        }
                    } else if kind == Self::LC_SYMTAB && size >= 24 {
                        // symtab_command: symoff, nsyms, stroff, strsize.
                        symtab = Some((Self::u32_at(command.add(8)), Self::u32_at(command.add(12)), Self::u32_at(command.add(16)), Self::u32_at(command.add(20))));
                    }
                    offset += size;
                }
                let (text, (vmaddr, fileoff), (symoff, nsyms, stroff, strsize)) = (text?, linkedit?, symtab?);
                // A file offset inside __LINKEDIT is mapped at slide + vmaddr + (offset - fileoff).
                let base = (slide as u64).wrapping_add(vmaddr).wrapping_sub(fileoff) as usize;
                Some(Symtab {
                    slide,
                    text,
                    symbols: base.wrapping_add(symoff as usize) as *const u8,
                    count: nsyms as usize,
                    strings: base.wrapping_add(stroff as usize) as *const u8,
                    strings_len: strsize as usize,
                })
            }
        }

        /// ` name+0xoff` for the defined section symbol with the greatest address not above
        /// `addr`; nothing when `addr` is outside `__TEXT` or no symbol precedes it. One linear
        /// pass over the table: no sorted copy, because the handler must not allocate.
        fn write_name(&self, addr: usize) {
            let target = addr.wrapping_sub(self.slide) as u64;
            if target < self.text.0 || target >= self.text.1 {
                return;
            }
            let mut best: Option<(u64, usize)> = None;
            for index in 0..self.count {
                let entry = self.symbols.wrapping_add(index * Self::NLIST_64);
                let (kind, value) = unsafe { (*entry.add(4), Self::u64_at(entry.add(8))) };
                if kind & Self::N_STAB != 0 || kind & Self::N_TYPE != Self::N_SECT || value > target {
                    continue;
                }
                if best.map_or(true, |(seen, _)| value > seen) {
                    best = Some((value, unsafe { Self::u32_at(entry) } as usize));
                }
            }
            let Some((value, strx)) = best else { return };
            if strx >= self.strings_len {
                return;
            }
            let rest = unsafe { core::slice::from_raw_parts(self.strings.add(strx), self.strings_len - strx) };
            let len = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
            write_str(" ");
            write_bytes(&rest[..len]);
            write_str("+");
            write_offset((target - value) as usize);
        }
    }

    pub fn report(info: &core::panic::PanicInfo<'_>) {
        // The message says what was violated; the location alone does not.
        let _ = core::fmt::write(&mut Stderr, format_args!("{}\n", info.message()));
        // The return addresses up the frame-pointer chain, each named from the executable's own
        // symbol table (`Symtab`). No allocation, and no unwinder: the frame records are read
        // directly, which Darwin arm64 always keeps.
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
            write_addr(ret);
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
    // from the interrupted context (not the handler's own frames), then the same for every other
    // thread of the task, then abort. The kernel hands a process-directed signal to any thread
    // that does not block it, usually the one parked in `accept`, so the interrupted thread alone
    // rarely is the one doing the work; the others are suspended and read through Mach
    // `thread_get_state`. Every pc, lr and frame address carries its symbol name, read from the
    // executable's own `LC_SYMTAB` (`Symtab`). The handler uses only `write`, dyld's image header
    // and the Mach thread calls, and no heap allocation (the thread list is a kernel-provided array, never freed: the
    // process aborts). It is installed before `main` runs, from a Mach-O initializer, so no
    // caller has to remember to ask for it.
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    mod quit {
        use super::{_dyld_get_image_vmaddr_slide, walk, write_addr, write_hex, write_str, write_u32};

        const SIGQUIT: i32 = 3;
        const SA_SIGINFO: i32 = 0x40;
        /// `ARM_THREAD_STATE64`: x0..x28, fp, lr, sp, pc, cpsr and pad; 272 bytes, 68 words.
        const ARM_THREAD_STATE64: i32 = 6;
        const ARM_THREAD_STATE64_COUNT: u32 = 68;

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
            static mach_task_self_: u32;
            fn mach_thread_self() -> u32;
            fn task_threads(task: u32, threads: *mut *mut u32, count: *mut u32) -> i32;
            fn thread_suspend(thread: u32) -> i32;
            fn thread_get_state(thread: u32, flavor: i32, state: *mut u64, count: *mut u32) -> i32;
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
                write_addr(pc);
                write_str("lr ");
                write_addr(lr);
                write_str("sp ");
                write_hex(sp);
                write_str("\n");
                walk(fp as *const usize);
                others();
                abort()
            }
        }

        /// Every thread but the interrupted one: suspended, then pc, lr and its frame chain.
        unsafe fn others() {
            unsafe {
                let me = mach_thread_self();
                let mut threads: *mut u32 = core::ptr::null_mut();
                let mut count = 0u32;
                if task_threads(mach_task_self_, &mut threads, &mut count) != 0 {
                    return;
                }
                for index in 0..count {
                    let thread = *threads.add(index as usize);
                    if thread == me || thread_suspend(thread) != 0 {
                        continue;
                    }
                    let mut state = [0u64; 34];
                    let mut words = ARM_THREAD_STATE64_COUNT;
                    if thread_get_state(thread, ARM_THREAD_STATE64, state.as_mut_ptr(), &mut words) != 0 {
                        continue;
                    }
                    write_str("thread ");
                    write_u32(index);
                    write_str("\npc ");
                    write_addr(state[32] as usize);
                    write_str("lr ");
                    write_addr(state[30] as usize);
                    walk(state[29] as *const usize);
                }
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
