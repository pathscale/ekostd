use super::*;

use core::ffi::{c_char, c_int, c_void};

// Layout and values below are copied from the `libc` crate, 0.2.189, per target. The `libc` crate
// path of each one is in the parity test's comment, `tests/libc_lite_parity_memnet.rs`.

#[allow(non_camel_case_types)]
pub type socklen_t = u32;

#[allow(non_camel_case_types)]
pub type size_t = usize;

// Apple only: the Mach port and virtual memory types the allocator names.
#[cfg(target_os = "macos")]
#[allow(non_camel_case_types)]
pub type mach_port_t = core::ffi::c_uint;
#[cfg(target_os = "macos")]
#[allow(non_camel_case_types)]
pub type mach_vm_address_t = u64;
#[cfg(target_os = "macos")]
#[allow(non_camel_case_types)]
pub type mach_vm_size_t = u64;
#[cfg(target_os = "macos")]
#[allow(non_camel_case_types)]
pub type vm_prot_t = c_int;

#[cfg(target_os = "macos")]
#[allow(non_camel_case_types)]
pub type time_t = core::ffi::c_long;
#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[allow(non_camel_case_types)]
pub type time_t = i64;
#[cfg(all(target_os = "linux", target_env = "musl"))]
#[allow(non_camel_case_types)]
pub type time_t = core::ffi::c_long;

#[cfg(target_os = "macos")]
#[allow(non_camel_case_types)]
pub type sa_family_t = u8;
#[cfg(target_os = "linux")]
#[allow(non_camel_case_types)]
pub type sa_family_t = u16;

#[allow(non_camel_case_types)]
pub type in_addr_t = u32;
#[allow(non_camel_case_types)]
pub type in_port_t = u16;

/// The generic address that `bind`, `connect` and `accept` take a pointer to. Kept in a private
/// module: it is part of those signatures and of nothing else this crate says.
mod abi {
    use super::sa_family_t;
    use core::ffi::c_char;

    #[repr(C)]
    #[derive(Clone, Copy)]
    #[allow(dead_code, non_camel_case_types)]
    pub struct sockaddr {
        #[cfg(target_os = "macos")]
        pub sa_len: u8,
        pub sa_family: sa_family_t,
        pub sa_data: [c_char; 14],
    }
}
pub use self::abi::sockaddr;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub struct in_addr {
    pub s_addr: in_addr_t,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub struct sockaddr_in {
    #[cfg(target_os = "macos")]
    pub sin_len: u8,
    pub sin_family: sa_family_t,
    pub sin_port: in_port_t,
    pub sin_addr: in_addr,
    pub sin_zero: [c_char; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub struct in6_addr {
    pub s6_addr: [u8; 16],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub struct sockaddr_in6 {
    #[cfg(target_os = "macos")]
    pub sin6_len: u8,
    pub sin6_family: sa_family_t,
    pub sin6_port: in_port_t,
    pub sin6_flowinfo: u32,
    pub sin6_addr: in6_addr,
    pub sin6_scope_id: u32,
}

#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub struct sockaddr_vm {
    pub svm_family: sa_family_t,
    pub svm_reserved1: u16,
    pub svm_port: u32,
    pub svm_cid: u32,
    pub svm_flags: u8,
    pub svm_zero: [u8; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
#[allow(non_camel_case_types)]
pub struct sockaddr_storage {
    #[cfg(target_os = "macos")]
    pub ss_len: u8,
    pub ss_family: sa_family_t,
    #[cfg(target_os = "macos")]
    pub __ss_pad1: [u8; 6],
    #[cfg(target_os = "macos")]
    pub __ss_align: i64,
    #[cfg(target_os = "macos")]
    pub __ss_pad2: [u8; 112],
    #[cfg(target_os = "linux")]
    pub __ss_padding: [u8; 118],
    #[cfg(target_os = "linux")]
    pub __ss_align: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
#[allow(non_camel_case_types)]
pub struct sockaddr_un {
    #[cfg(target_os = "macos")]
    pub sun_len: u8,
    pub sun_family: sa_family_t,
    #[cfg(target_os = "macos")]
    pub sun_path: [c_char; 104],
    #[cfg(target_os = "linux")]
    pub sun_path: [c_char; 108],
}

pub const AF_UNSPEC: c_int = 0;
// `AF_UNIX` is `AF_LOCAL` in the `libc` crate, and both are 1 on every target here.
pub const AF_UNIX: c_int = 1;
pub const AF_INET: c_int = 2;
#[cfg(target_os = "macos")]
pub const AF_INET6: c_int = 30;
#[cfg(target_os = "linux")]
pub const AF_INET6: c_int = 10;
#[cfg(target_os = "linux")]
pub const AF_VSOCK: c_int = 40;

#[cfg(target_os = "linux")]
pub const VMADDR_CID_ANY: u32 = u32::MAX;
#[cfg(target_os = "linux")]
pub const VMADDR_CID_LOCAL: u32 = 1;
#[cfg(target_os = "linux")]
pub const VMADDR_CID_HOST: u32 = 2;

pub const IPPROTO_IP: c_int = 0;
pub const IPPROTO_ICMP: c_int = 1;
pub const IPPROTO_TCP: c_int = 6;
pub const IPPROTO_UDP: c_int = 17;
pub const IPPROTO_IPV6: c_int = 41;

pub const TCP_NODELAY: c_int = 1;
#[cfg(target_os = "macos")]
pub const TCP_KEEPIDLE: c_int = 0x10;
#[cfg(target_os = "linux")]
pub const TCP_KEEPIDLE: c_int = 4;
#[cfg(target_os = "macos")]
pub const TCP_KEEPINTVL: c_int = 0x101;
#[cfg(target_os = "linux")]
pub const TCP_KEEPINTVL: c_int = 5;
#[cfg(target_os = "macos")]
pub const TCP_KEEPCNT: c_int = 0x102;
#[cfg(target_os = "linux")]
pub const TCP_KEEPCNT: c_int = 6;

#[cfg(target_os = "macos")]
pub const IPV6_V6ONLY: c_int = 27;
#[cfg(target_os = "linux")]
pub const IPV6_V6ONLY: c_int = 26;

#[cfg(target_os = "macos")]
pub const ENAMETOOLONG: c_int = 63;
#[cfg(target_os = "linux")]
pub const ENAMETOOLONG: c_int = 36;

pub const EPIPE: c_int = 32;
pub const ENFILE: c_int = 23;
pub const EMFILE: c_int = 24;

#[cfg(target_os = "macos")]
pub const EPROTO: c_int = 100;
#[cfg(target_os = "linux")]
pub const EPROTO: c_int = 71;

#[cfg(target_os = "macos")]
pub const EINPROGRESS: c_int = 36;
#[cfg(target_os = "linux")]
pub const EINPROGRESS: c_int = 115;

#[cfg(target_os = "macos")]
pub const EALREADY: c_int = 37;
#[cfg(target_os = "linux")]
pub const EALREADY: c_int = 114;

#[cfg(target_os = "macos")]
pub const ENOTSOCK: c_int = 38;
#[cfg(target_os = "linux")]
pub const ENOTSOCK: c_int = 88;

#[cfg(target_os = "macos")]
pub const EDESTADDRREQ: c_int = 39;
#[cfg(target_os = "linux")]
pub const EDESTADDRREQ: c_int = 89;

#[cfg(target_os = "macos")]
pub const EMSGSIZE: c_int = 40;
#[cfg(target_os = "linux")]
pub const EMSGSIZE: c_int = 90;

#[cfg(target_os = "macos")]
pub const EPROTOTYPE: c_int = 41;
#[cfg(target_os = "linux")]
pub const EPROTOTYPE: c_int = 91;

#[cfg(target_os = "macos")]
pub const ENOPROTOOPT: c_int = 42;
#[cfg(target_os = "linux")]
pub const ENOPROTOOPT: c_int = 92;

#[cfg(target_os = "macos")]
pub const EPROTONOSUPPORT: c_int = 43;
#[cfg(target_os = "linux")]
pub const EPROTONOSUPPORT: c_int = 93;

#[cfg(target_os = "macos")]
pub const EAFNOSUPPORT: c_int = 47;
#[cfg(target_os = "linux")]
pub const EAFNOSUPPORT: c_int = 97;

#[cfg(target_os = "macos")]
pub const EADDRINUSE: c_int = 48;
#[cfg(target_os = "linux")]
pub const EADDRINUSE: c_int = 98;

#[cfg(target_os = "macos")]
pub const EADDRNOTAVAIL: c_int = 49;
#[cfg(target_os = "linux")]
pub const EADDRNOTAVAIL: c_int = 99;

#[cfg(target_os = "macos")]
pub const ENETDOWN: c_int = 50;
#[cfg(target_os = "linux")]
pub const ENETDOWN: c_int = 100;

#[cfg(target_os = "macos")]
pub const ENETUNREACH: c_int = 51;
#[cfg(target_os = "linux")]
pub const ENETUNREACH: c_int = 101;

#[cfg(target_os = "macos")]
pub const ENETRESET: c_int = 52;
#[cfg(target_os = "linux")]
pub const ENETRESET: c_int = 102;

#[cfg(target_os = "macos")]
pub const ECONNABORTED: c_int = 53;
#[cfg(target_os = "linux")]
pub const ECONNABORTED: c_int = 103;

#[cfg(target_os = "macos")]
pub const ECONNRESET: c_int = 54;
#[cfg(target_os = "linux")]
pub const ECONNRESET: c_int = 104;

#[cfg(target_os = "macos")]
pub const ENOBUFS: c_int = 55;
#[cfg(target_os = "linux")]
pub const ENOBUFS: c_int = 105;

#[cfg(target_os = "macos")]
pub const EISCONN: c_int = 56;
#[cfg(target_os = "linux")]
pub const EISCONN: c_int = 106;

#[cfg(target_os = "macos")]
pub const ENOTCONN: c_int = 57;
#[cfg(target_os = "linux")]
pub const ENOTCONN: c_int = 107;

#[cfg(target_os = "macos")]
pub const ESHUTDOWN: c_int = 58;
#[cfg(target_os = "linux")]
pub const ESHUTDOWN: c_int = 108;

#[cfg(target_os = "macos")]
pub const ETIMEDOUT: c_int = 60;
#[cfg(target_os = "linux")]
pub const ETIMEDOUT: c_int = 110;

#[cfg(target_os = "macos")]
pub const ECONNREFUSED: c_int = 61;
#[cfg(target_os = "linux")]
pub const ECONNREFUSED: c_int = 111;

#[cfg(target_os = "macos")]
pub const EHOSTUNREACH: c_int = 65;
#[cfg(target_os = "linux")]
pub const EHOSTUNREACH: c_int = 113;

#[cfg(target_os = "macos")]
pub const SOL_SOCKET: c_int = 0xffff;
#[cfg(target_os = "linux")]
pub const SOL_SOCKET: c_int = 1;

#[cfg(target_os = "macos")]
pub const SO_REUSEADDR: c_int = 0x0004;
#[cfg(target_os = "linux")]
pub const SO_REUSEADDR: c_int = 2;

#[cfg(target_os = "macos")]
pub const SO_KEEPALIVE: c_int = 0x0008;
#[cfg(target_os = "linux")]
pub const SO_KEEPALIVE: c_int = 9;

#[cfg(target_os = "macos")]
pub const SO_REUSEPORT: c_int = 0x0200;
#[cfg(target_os = "linux")]
pub const SO_REUSEPORT: c_int = 15;

#[cfg(target_os = "macos")]
pub const SO_SNDBUF: c_int = 0x1001;
#[cfg(target_os = "linux")]
pub const SO_SNDBUF: c_int = 7;

#[cfg(target_os = "macos")]
pub const SO_RCVBUF: c_int = 0x1002;
#[cfg(target_os = "linux")]
pub const SO_RCVBUF: c_int = 8;

#[cfg(target_os = "macos")]
pub const SO_ERROR: c_int = 0x1007;
#[cfg(target_os = "linux")]
pub const SO_ERROR: c_int = 4;

#[cfg(target_os = "macos")]
pub const SO_RCVTIMEO: c_int = 0x1006;
#[cfg(target_os = "linux")]
pub const SO_RCVTIMEO: c_int = 20;
#[cfg(target_os = "macos")]
pub const SO_SNDTIMEO: c_int = 0x1005;
#[cfg(target_os = "linux")]
pub const SO_SNDTIMEO: c_int = 21;

// Only Apple has `SO_NOSIGPIPE`; the Linux targets use `MSG_NOSIGNAL` on each send instead.
#[cfg(target_os = "macos")]
pub const SO_NOSIGPIPE: c_int = 0x1022;

#[cfg(target_os = "macos")]
pub const MSG_NOSIGNAL: c_int = 0x80000;
#[cfg(target_os = "linux")]
pub const MSG_NOSIGNAL: c_int = 0x4000;

#[cfg(target_os = "macos")]
pub const MSG_DONTWAIT: c_int = 0x80;
#[cfg(target_os = "linux")]
pub const MSG_DONTWAIT: c_int = 0x40;

pub const SHUT_RD: c_int = 0;
pub const SHUT_WR: c_int = 1;
pub const SHUT_RDWR: c_int = 2;

#[cfg(target_os = "macos")]
pub const MAP_ANON: c_int = 0x1000;
#[cfg(target_os = "linux")]
pub const MAP_ANON: c_int = 0x0020;

pub const MAP_FAILED: *mut c_void = !0 as *mut c_void;
pub const MAP_SHARED: c_int = 0x0001;
pub const MAP_PRIVATE: c_int = 0x0002;
/// `msync`: invalidate other mappings of the same file. 2 on macOS and on Linux.
pub const MS_INVALIDATE: c_int = 0x0002;
pub const MAP_FIXED: c_int = 0x0010;

#[cfg(target_os = "macos")]
pub const MAP_NORESERVE: c_int = 0x0040;
#[cfg(target_os = "linux")]
pub const MAP_NORESERVE: c_int = 0x4000;

pub const MADV_DONTNEED: c_int = 4;
#[cfg(target_os = "macos")]
pub const MADV_FREE: c_int = 5;
#[cfg(target_os = "linux")]
pub const MADV_FREE: c_int = 8;

#[cfg(target_os = "macos")]
pub const VM_PROT_NONE: vm_prot_t = 0x00;

pub const PROT_NONE: c_int = 0;
pub const PROT_READ: c_int = 1;
pub const PROT_WRITE: c_int = 2;
pub const PROT_EXEC: c_int = 4;

pub const SOCK_STREAM: c_int = 1;
pub const SOCK_DGRAM: c_int = 2;
#[cfg(target_os = "linux")]
pub const SOCK_NONBLOCK: c_int = 0x800;
#[cfg(target_os = "linux")]
pub const SOCK_CLOEXEC: c_int = 0x80000;

#[cfg(target_os = "macos")]
pub const EVFILT_READ: i16 = -1;
#[cfg(target_os = "macos")]
pub const EVFILT_WRITE: i16 = -2;
#[cfg(target_os = "macos")]
pub const EVFILT_USER: i16 = -10;

#[cfg(target_os = "macos")]
pub const EV_ADD: u16 = 0x0001;
#[cfg(target_os = "macos")]
pub const EV_DELETE: u16 = 0x0002;
#[cfg(target_os = "macos")]
pub const EV_ENABLE: u16 = 0x0004;
#[cfg(target_os = "macos")]
pub const EV_DISABLE: u16 = 0x0008;
#[cfg(target_os = "macos")]
pub const EV_ONESHOT: u16 = 0x0010;
#[cfg(target_os = "macos")]
pub const EV_CLEAR: u16 = 0x0020;
#[cfg(target_os = "macos")]
pub const EV_RECEIPT: u16 = 0x0040;
#[cfg(target_os = "macos")]
pub const EV_DISPATCH: u16 = 0x0080;
#[cfg(target_os = "macos")]
pub const EV_EOF: u16 = 0x8000;
#[cfg(target_os = "macos")]
pub const EV_ERROR: u16 = 0x4000;
#[cfg(target_os = "macos")]
pub const NOTE_TRIGGER: u32 = 0x01000000;

#[cfg(target_os = "macos")]
#[repr(C, packed(4))]
#[derive(Clone, Copy, Debug)]
#[allow(non_camel_case_types)]
pub struct kevent {
    pub ident: usize,
    pub filter: i16,
    pub flags: u16,
    pub fflags: u32,
    pub data: isize,
    pub udata: *mut c_void,
}

#[cfg(target_os = "linux")]
pub const EPOLLIN: c_int = 0x001;
#[cfg(target_os = "linux")]
pub const EPOLLPRI: c_int = 0x002;
#[cfg(target_os = "linux")]
pub const EPOLLOUT: c_int = 0x004;
#[cfg(target_os = "linux")]
pub const EPOLLERR: c_int = 0x008;
#[cfg(target_os = "linux")]
pub const EPOLLHUP: c_int = 0x010;
#[cfg(target_os = "linux")]
pub const EPOLLRDHUP: c_int = 0x2000;
#[cfg(target_os = "linux")]
pub const EPOLLONESHOT: c_int = 1 << 30;
#[cfg(target_os = "linux")]
pub const EPOLLET: c_int = 1 << 31;
#[cfg(target_os = "linux")]
pub const EPOLL_CLOEXEC: c_int = 0x80000;

#[cfg(target_os = "linux")]
pub const EPOLL_CTL_ADD: c_int = 1;
#[cfg(target_os = "linux")]
pub const EPOLL_CTL_DEL: c_int = 2;
#[cfg(target_os = "linux")]
pub const EPOLL_CTL_MOD: c_int = 3;

#[cfg(target_os = "linux")]
pub const EFD_SEMAPHORE: c_int = 1;
#[cfg(target_os = "linux")]
pub const EFD_NONBLOCK: c_int = 0x800;
#[cfg(target_os = "linux")]
pub const EFD_CLOEXEC: c_int = 0x80000;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub struct epoll_event {
    pub events: u32,
    pub u64: u64,
}

#[cfg(all(target_os = "linux", not(target_arch = "x86_64")))]
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub struct epoll_event {
    pub events: u32,
    pub u64: u64,
}

#[cfg(target_os = "macos")]
pub const _SC_PAGESIZE: c_int = 29;
#[cfg(target_os = "linux")]
pub const _SC_PAGESIZE: c_int = 30;

unsafe extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn calloc(nobj: usize, size: usize) -> *mut c_void;
    pub fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);
    pub fn posix_memalign(memptr: *mut *mut c_void, align: usize, size: usize) -> c_int;

    pub fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: i64,
    ) -> *mut c_void;
    pub fn munmap(addr: *mut c_void, len: usize) -> c_int;
    pub fn msync(addr: *mut c_void, len: usize, flags: c_int) -> c_int;
    pub fn mprotect(addr: *mut c_void, len: usize, prot: c_int) -> c_int;
    pub fn madvise(addr: *mut c_void, len: usize, advice: c_int) -> c_int;
    // Apple takes a `const` address and a `c_char` vector; Linux takes a `mut` address and `c_uchar`.
    #[cfg(target_os = "macos")]
    pub fn mincore(addr: *const c_void, len: usize, vec: *mut c_char) -> c_int;
    #[cfg(target_os = "linux")]
    pub fn mincore(addr: *mut c_void, len: usize, vec: *mut core::ffi::c_uchar) -> c_int;

    pub fn socket(domain: c_int, ty: c_int, protocol: c_int) -> c_int;
    pub fn bind(socket: c_int, address: *const sockaddr, address_len: socklen_t) -> c_int;
    pub fn connect(socket: c_int, address: *const sockaddr, len: socklen_t) -> c_int;
    pub fn listen(socket: c_int, backlog: c_int) -> c_int;
    pub fn accept(socket: c_int, address: *mut sockaddr, address_len: *mut socklen_t) -> c_int;
    #[cfg(target_os = "linux")]
    pub fn accept4(
        socket: c_int,
        address: *mut sockaddr,
        address_len: *mut socklen_t,
        flags: c_int,
    ) -> c_int;
    pub fn dup(fd: c_int) -> c_int;
    pub fn getsockopt(
        socket: c_int,
        level: c_int,
        name: c_int,
        value: *mut c_void,
        option_len: *mut socklen_t,
    ) -> c_int;
    pub fn setsockopt(
        socket: c_int,
        level: c_int,
        name: c_int,
        value: *const c_void,
        option_len: socklen_t,
    ) -> c_int;
    pub fn getsockname(socket: c_int, address: *mut sockaddr, address_len: *mut socklen_t)
        -> c_int;
    pub fn getpeername(socket: c_int, address: *mut sockaddr, address_len: *mut socklen_t)
        -> c_int;
    pub fn shutdown(socket: c_int, how: c_int) -> c_int;
    pub fn socketpair(
        domain: c_int,
        ty: c_int,
        protocol: c_int,
        socket_vector: *mut c_int,
    ) -> c_int;
    pub fn send(socket: c_int, buf: *const c_void, len: usize, flags: c_int) -> isize;
    pub fn recv(socket: c_int, buf: *mut c_void, len: usize, flags: c_int) -> isize;
    pub fn sendto(
        socket: c_int,
        buf: *const c_void,
        len: usize,
        flags: c_int,
        addr: *const sockaddr,
        addrlen: socklen_t,
    ) -> isize;
    pub fn recvfrom(
        socket: c_int,
        buf: *mut c_void,
        len: usize,
        flags: c_int,
        addr: *mut sockaddr,
        addrlen: *mut socklen_t,
    ) -> isize;

    #[cfg(target_os = "macos")]
    pub fn kqueue() -> c_int;
    #[cfg(target_os = "macos")]
    pub fn kevent(
        kq: c_int,
        changelist: *const kevent,
        nchanges: c_int,
        eventlist: *mut kevent,
        nevents: c_int,
        timeout: *const timespec,
    ) -> c_int;

    #[cfg(target_os = "linux")]
    pub fn epoll_create1(flags: c_int) -> c_int;
    #[cfg(target_os = "linux")]
    pub fn epoll_ctl(epfd: c_int, op: c_int, fd: c_int, event: *mut epoll_event) -> c_int;
    #[cfg(target_os = "linux")]
    pub fn epoll_wait(
        epfd: c_int,
        events: *mut epoll_event,
        maxevents: c_int,
        timeout: c_int,
    ) -> c_int;
    #[cfg(target_os = "linux")]
    pub fn epoll_pwait(
        epfd: c_int,
        events: *mut epoll_event,
        maxevents: c_int,
        timeout: c_int,
        sigmask: *const sigset_t,
    ) -> c_int;
    #[cfg(target_os = "linux")]
    pub fn eventfd(initval: core::ffi::c_uint, flags: c_int) -> c_int;

    pub fn nanosleep(rqtp: *const timespec, rmtp: *mut timespec) -> c_int;

    #[cfg(target_os = "macos")]
    pub fn __error() -> *mut c_int;
    #[cfg(target_os = "linux")]
    pub fn __errno_location() -> *mut c_int;

    // glibc names the XSI form `__xpg_strerror_r`; musl exports `strerror_r` as that form.
    #[cfg_attr(all(target_os = "linux", target_env = "gnu"), link_name = "__xpg_strerror_r")]
    pub fn strerror_r(errnum: c_int, buf: *mut c_char, buflen: usize) -> c_int;
}
