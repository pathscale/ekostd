//! `eko::libc_lite` process and signal items against the `libc` crate.
//!
//! Every comparison is a compile-time assertion, so `cargo check --tests --target T` proves the
//! declarations without running anything. Constants must be equal and of the same type. Type
//! aliases must be the same type. Structs must have the same size, alignment, field offsets and
//! field types. Each function is assigned from both sides to one explicit pointer type. Where a
//! signature names a struct, the two sides name their own struct (they are different types) and
//! the struct is proven by the layout assertions.

#[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "aarch64")))]
mod checks {
    use core::ffi::{c_char, c_int, c_long, c_short, c_void};

    const fn same_type_of<T>(_: &T, _: &T) {}

    macro_rules! same_const {
        ($($n:ident),* $(,)?) => {
            $(const _: () = assert!(eko::libc_lite::$n == libc::$n);)*
        };
    }

    macro_rules! same_type {
        ($($t:ident),* $(,)?) => {
            $(
                const _: Option<libc::$t> = None::<eko::libc_lite::$t>;
                const _: () = assert!(
                    core::mem::size_of::<eko::libc_lite::$t>() == core::mem::size_of::<libc::$t>()
                );
            )*
        };
    }

    macro_rules! same_struct {
        ($t:ident { same: [$($f:ident),*], offset_only: [$($g:ident),*] }) => {
            const _: () = {
                assert!(
                    core::mem::size_of::<eko::libc_lite::$t>() == core::mem::size_of::<libc::$t>()
                );
                assert!(
                    core::mem::align_of::<eko::libc_lite::$t>()
                        == core::mem::align_of::<libc::$t>()
                );
                $(
                    assert!(
                        core::mem::offset_of!(eko::libc_lite::$t, $f)
                            == core::mem::offset_of!(libc::$t, $f)
                    );
                )*
                $(
                    assert!(
                        core::mem::offset_of!(eko::libc_lite::$t, $g)
                            == core::mem::offset_of!(libc::$t, $g)
                    );
                )*
                let a: eko::libc_lite::$t = unsafe { core::mem::zeroed() };
                let b: libc::$t = unsafe { core::mem::zeroed() };
                $(same_type_of(&a.$f, &b.$f);)*
            };
        };
    }

    macro_rules! same_fn {
        ($n:ident: $ty:ty) => {
            const _: $ty = eko::libc_lite::$n;
            const _: $ty = libc::$n;
        };
    }

    same_const!(
        EAGAIN, EIO, EWOULDBLOCK, FD_CLOEXEC, F_DUPFD_CLOEXEC, F_GETFD, F_GETFL, F_SETFD, F_SETFL,
        LOCK_EX, LOCK_NB, O_CLOEXEC, O_NONBLOCK, POLLIN, SA_RESTART, SIGHUP, SIGINT, SIGKILL,
        SIGTERM, STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO, WNOHANG, _SC_NPROCESSORS_ONLN,
    );

    // The lock kinds are `c_short` on macOS and `c_int` on Linux, and the resource ids are `c_uint`
    // on glibc: comparing the constants fixes their types too.
    same_const!(
        F_RDLCK, F_SETLK, F_SETLKW, F_UNLCK, F_WRLCK, LOCK_SH, POLLHUP, POSIX_SPAWN_SETPGROUP,
        RLIMIT_CPU, RLIMIT_NOFILE, RLIM_INFINITY, RUSAGE_SELF,
    );

    // Only Apple has these two.
    #[cfg(target_os = "macos")]
    same_const!(POSIX_SPAWN_CLOEXEC_DEFAULT, PROC_PIDTASKINFO);

    same_type!(nfds_t, pid_t, rlim_t);

    // Only Apple has the Mach task items.
    #[cfg(target_os = "macos")]
    same_const!(KERN_SUCCESS, MACH_TASK_BASIC_INFO, MACH_TASK_BASIC_INFO_COUNT);

    #[cfg(target_os = "macos")]
    same_type!(task_flavor_t);

    #[cfg(target_os = "macos")]
    same_struct!(time_value_t { same: [seconds, microseconds], offset_only: [] });

    // `mach_task_basic_info` is packed to 4 bytes, so a field cannot be borrowed. Each field is
    // copied out of the zeroed value before its type is compared. `user_time` and `system_time`
    // are `time_value_t`, a different Rust type on each side, so only their offsets are compared.
    #[cfg(target_os = "macos")]
    const _: () = {
        use core::mem::{align_of, offset_of, size_of};
        assert!(
            size_of::<eko::libc_lite::mach_task_basic_info>()
                == size_of::<libc::mach_task_basic_info>()
        );
        assert!(
            align_of::<eko::libc_lite::mach_task_basic_info>()
                == align_of::<libc::mach_task_basic_info>()
        );
        assert!(
            offset_of!(eko::libc_lite::mach_task_basic_info, virtual_size)
                == offset_of!(libc::mach_task_basic_info, virtual_size)
        );
        assert!(
            offset_of!(eko::libc_lite::mach_task_basic_info, resident_size)
                == offset_of!(libc::mach_task_basic_info, resident_size)
        );
        assert!(
            offset_of!(eko::libc_lite::mach_task_basic_info, resident_size_max)
                == offset_of!(libc::mach_task_basic_info, resident_size_max)
        );
        assert!(
            offset_of!(eko::libc_lite::mach_task_basic_info, user_time)
                == offset_of!(libc::mach_task_basic_info, user_time)
        );
        assert!(
            offset_of!(eko::libc_lite::mach_task_basic_info, system_time)
                == offset_of!(libc::mach_task_basic_info, system_time)
        );
        assert!(
            offset_of!(eko::libc_lite::mach_task_basic_info, policy)
                == offset_of!(libc::mach_task_basic_info, policy)
        );
        assert!(
            offset_of!(eko::libc_lite::mach_task_basic_info, suspend_count)
                == offset_of!(libc::mach_task_basic_info, suspend_count)
        );
        let a: eko::libc_lite::mach_task_basic_info = unsafe { core::mem::zeroed() };
        let b: libc::mach_task_basic_info = unsafe { core::mem::zeroed() };
        same_type_of(&{ a.virtual_size }, &{ b.virtual_size });
        same_type_of(&{ a.resident_size }, &{ b.resident_size });
        same_type_of(&{ a.resident_size_max }, &{ b.resident_size_max });
        same_type_of(&{ a.policy }, &{ b.policy });
        same_type_of(&{ a.suspend_count }, &{ b.suspend_count });
    };

    // `mach_port_t` is `c_uint`, the flavor and the count are `u32`, the info pointer is `*mut c_int`.
    #[cfg(target_os = "macos")]
    same_fn!(
        task_info: unsafe extern "C" fn(core::ffi::c_uint, u32, *mut c_int, *mut u32) -> c_int
    );

    // `mach_task_self_` is a static, so it is compared at run time: both names are the same symbol.
    #[cfg(target_os = "macos")]
    #[test]
    #[allow(deprecated)]
    fn libc_lite_mach_task_self_matches_the_libc_crate() {
        let ours: core::ffi::c_uint = unsafe { eko::libc_lite::mach_task_self_ };
        let theirs: libc::mach_port_t = unsafe { libc::mach_task_self_ };
        assert_eq!(ours, theirs);
    }

    same_struct!(pollfd { same: [fd, events, revents], offset_only: [] });

    same_struct!(rlimit { same: [rlim_cur, rlim_max], offset_only: [] });

    // `timeval` is a different Rust type on each side; its layout is proven in the thread test and
    // here by the offsets of the two fields of that type. The size also covers musl's reserved tail.
    same_struct!(rusage {
        same: [
            ru_maxrss, ru_ixrss, ru_idrss, ru_isrss, ru_minflt, ru_majflt, ru_nswap, ru_inblock,
            ru_oublock, ru_msgsnd, ru_msgrcv, ru_nsignals, ru_nvcsw, ru_nivcsw
        ],
        offset_only: [ru_utime, ru_stime]
    });

    same_struct!(flock { same: [l_start, l_len, l_pid, l_type, l_whence], offset_only: [] });

    #[cfg(target_os = "macos")]
    same_struct!(proc_taskinfo {
        same: [
            pti_virtual_size, pti_resident_size, pti_total_user, pti_total_system,
            pti_threads_user, pti_threads_system, pti_policy, pti_faults, pti_pageins,
            pti_cow_faults, pti_messages_sent, pti_messages_received, pti_syscalls_mach,
            pti_syscalls_unix, pti_csw, pti_threadnum, pti_numrunning, pti_priority
        ],
        offset_only: []
    });

    // Apple hands out both spawn objects as `*mut c_void`, so they must be the very same type.
    #[cfg(target_os = "macos")]
    same_type!(posix_spawnattr_t, posix_spawn_file_actions_t);

    // Linux has structs with private fields, so only size and alignment can be compared.
    #[cfg(target_os = "linux")]
    macro_rules! same_opaque {
        ($($t:ident),* $(,)?) => {
            $(
                const _: () = {
                    assert!(
                        core::mem::size_of::<eko::libc_lite::$t>() == core::mem::size_of::<libc::$t>()
                    );
                    assert!(
                        core::mem::align_of::<eko::libc_lite::$t>()
                            == core::mem::align_of::<libc::$t>()
                    );
                };
            )*
        };
    }
    #[cfg(target_os = "linux")]
    same_opaque!(posix_spawnattr_t, posix_spawn_file_actions_t);

    #[cfg(target_os = "macos")]
    same_struct!(sigaction { same: [sa_sigaction, sa_mask, sa_flags], offset_only: [] });

    #[cfg(target_os = "linux")]
    same_struct!(sigaction {
        same: [sa_sigaction, sa_flags, sa_restorer],
        offset_only: [sa_mask]
    });

    // `sigset_t` on Linux has a private field, so only its size and alignment can be compared.
    #[cfg(target_os = "linux")]
    const _: () = {
        assert!(
            core::mem::size_of::<eko::libc_lite::sigset_t>() == core::mem::size_of::<libc::sigset_t>()
        );
        assert!(
            core::mem::align_of::<eko::libc_lite::sigset_t>()
                == core::mem::align_of::<libc::sigset_t>()
        );
    };

    same_fn!(abort: unsafe extern "C" fn() -> !);
    same_fn!(atexit: unsafe extern "C" fn(extern "C" fn()) -> c_int);
    same_fn!(exit: unsafe extern "C" fn(c_int) -> !);
    same_fn!(_exit: unsafe extern "C" fn(c_int) -> !);
    same_fn!(fcntl: unsafe extern "C" fn(c_int, c_int, ...) -> c_int);
    same_fn!(chdir: unsafe extern "C" fn(*const c_char) -> c_int);
    same_fn!(dup2: unsafe extern "C" fn(c_int, c_int) -> c_int);
    same_fn!(
        execve: unsafe extern "C" fn(*const c_char, *const *const c_char, *const *const c_char) -> c_int
    );
    same_fn!(fork: unsafe extern "C" fn() -> i32);
    same_fn!(pipe: unsafe extern "C" fn(*mut c_int) -> c_int);
    same_fn!(setpgid: unsafe extern "C" fn(i32, i32) -> c_int);
    same_fn!(setsid: unsafe extern "C" fn() -> i32);
    same_fn!(waitpid: unsafe extern "C" fn(i32, *mut c_int, c_int) -> i32);
    same_fn!(kill: unsafe extern "C" fn(i32, c_int) -> c_int);
    same_fn!(setenv: unsafe extern "C" fn(*const c_char, *const c_char, c_int) -> c_int);
    same_fn!(ftruncate: unsafe extern "C" fn(c_int, i64) -> c_int);
    same_fn!(sysconf: unsafe extern "C" fn(c_int) -> c_long);
    same_fn!(flock: unsafe extern "C" fn(c_int, c_int) -> c_int);

    // The resource id is unsigned on glibc and `c_int` elsewhere.
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    type Resource = core::ffi::c_uint;
    #[cfg(not(all(target_os = "linux", target_env = "gnu")))]
    type Resource = c_int;

    // These signatures name a struct or a spawn object, which is a different type on each side, so
    // the function pointer type is generic over it and instantiated once per side.
    type RlimitFn<R, T> = unsafe extern "C" fn(R, T) -> c_int;
    type SpawnFn<A, F> = unsafe extern "C" fn(
        *mut i32,
        *const c_char,
        *const F,
        *const A,
        *const *mut c_char,
        *const *mut c_char,
    ) -> c_int;
    type AttrFn<A> = unsafe extern "C" fn(*mut A) -> c_int;
    type AttrFlagsFn<A> = unsafe extern "C" fn(*mut A, c_short) -> c_int;
    type AttrPgroupFn<A> = unsafe extern "C" fn(*mut A, i32) -> c_int;
    type ActionsFn<F> = unsafe extern "C" fn(*mut F) -> c_int;
    type ActionsFdFn<F> = unsafe extern "C" fn(*mut F, c_int) -> c_int;
    type ActionsDup2Fn<F> = unsafe extern "C" fn(*mut F, c_int, c_int) -> c_int;

    macro_rules! spawn_and_limit_fns {
        ($c:ident) => {
            const _: RlimitFn<Resource, *mut $c::rlimit> = $c::getrlimit;
            const _: RlimitFn<Resource, *const $c::rlimit> = $c::setrlimit;
            const _: RlimitFn<c_int, *mut $c::rusage> = $c::getrusage;
            const _: SpawnFn<$c::posix_spawnattr_t, $c::posix_spawn_file_actions_t> =
                $c::posix_spawn;
            const _: SpawnFn<$c::posix_spawnattr_t, $c::posix_spawn_file_actions_t> =
                $c::posix_spawnp;
            const _: AttrFn<$c::posix_spawnattr_t> = $c::posix_spawnattr_init;
            const _: AttrFn<$c::posix_spawnattr_t> = $c::posix_spawnattr_destroy;
            const _: AttrFlagsFn<$c::posix_spawnattr_t> = $c::posix_spawnattr_setflags;
            const _: AttrPgroupFn<$c::posix_spawnattr_t> = $c::posix_spawnattr_setpgroup;
            const _: ActionsFn<$c::posix_spawn_file_actions_t> = $c::posix_spawn_file_actions_init;
            const _: ActionsFn<$c::posix_spawn_file_actions_t> =
                $c::posix_spawn_file_actions_destroy;
            const _: ActionsFdFn<$c::posix_spawn_file_actions_t> =
                $c::posix_spawn_file_actions_addclose;
            const _: ActionsDup2Fn<$c::posix_spawn_file_actions_t> =
                $c::posix_spawn_file_actions_adddup2;
            #[cfg(target_os = "macos")]
            const _: unsafe extern "C" fn(c_int, c_int, u64, *mut c_void, c_int) -> c_int =
                $c::proc_pidinfo;
        };
    }

    mod ours {
        use super::*;
        use eko::libc_lite as c;
        spawn_and_limit_fns!(c);
    }

    mod theirs {
        use super::*;
        use libc as c;
        spawn_and_limit_fns!(c);
    }

    // The wait-status helpers are the one place `libc_lite` has a body, so they are compared at run
    // time against the libc crate over every 16-bit status and the 32-bit edges.
    #[test]
    fn libc_lite_wait_status_helpers_match_the_libc_crate() {
        for status in (0..=0xffff_i32).chain([-1, i32::MIN, i32::MAX]) {
            assert_eq!(eko::libc_lite::WIFEXITED(status), libc::WIFEXITED(status));
            assert_eq!(eko::libc_lite::WEXITSTATUS(status), libc::WEXITSTATUS(status));
            assert_eq!(eko::libc_lite::WTERMSIG(status), libc::WTERMSIG(status));
        }
    }

    #[cfg(target_os = "linux")]
    same_fn!(pipe2: unsafe extern "C" fn(*mut c_int, c_int) -> c_int);

    #[cfg(target_os = "macos")]
    same_fn!(umask: unsafe extern "C" fn(u16) -> u16);
    #[cfg(target_os = "linux")]
    same_fn!(umask: unsafe extern "C" fn(u32) -> u32);

    // These signatures name a struct, so each side is assigned to a pointer type naming its own.
    const _: unsafe extern "C" fn(*mut eko::libc_lite::pollfd, eko::libc_lite::nfds_t, c_int) -> c_int =
        eko::libc_lite::poll;
    const _: unsafe extern "C" fn(*mut libc::pollfd, libc::nfds_t, c_int) -> c_int = libc::poll;

    const _: unsafe extern "C" fn(
        c_int,
        *const eko::libc_lite::sigaction,
        *mut eko::libc_lite::sigaction,
    ) -> c_int = eko::libc_lite::sigaction;
    const _: unsafe extern "C" fn(c_int, *const libc::sigaction, *mut libc::sigaction) -> c_int =
        libc::sigaction;

    #[cfg(target_os = "macos")]
    same_fn!(sigemptyset: unsafe extern "C" fn(*mut u32) -> c_int);
    #[cfg(target_os = "linux")]
    const _: unsafe extern "C" fn(*mut eko::libc_lite::sigset_t) -> c_int = eko::libc_lite::sigemptyset;
    #[cfg(target_os = "linux")]
    const _: unsafe extern "C" fn(*mut libc::sigset_t) -> c_int = libc::sigemptyset;
}

#[test]
fn libc_lite_process_matches_the_libc_crate() {
    // The assertions are compile-time constants in `checks`. If this file compiles, they hold.
}
