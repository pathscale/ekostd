//! Nonblocking-capable IPv4/IPv6 TCP/UDP and Virtio-Vsock sockets.

pub use core::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

#[cfg(not(target_os = "none"))]
use crate::libc_lite as libc;
use crate::Errno;

/// Wildcard vsock context ID.
pub const VMADDR_CID_ANY: u32 = u32::MAX;
/// Local loopback vsock context ID.
pub const VMADDR_CID_LOCAL: u32 = 1;
/// Host hypervisor vsock context ID.
pub const VMADDR_CID_HOST: u32 = 2;

/// Shut down the reading side of a socket.
pub const SHUT_RD: i32 = 0;
/// Shut down the writing side of a socket.
pub const SHUT_WR: i32 = 1;
/// Shut down both reading and writing sides of a socket.
pub const SHUT_RDWR: i32 = 2;

/// Which half of a full-duplex connection to shut down.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Shutdown {
    /// Shut down the reading half (`SHUT_RD`).
    Read,
    /// Shut down the writing half (`SHUT_WR`).
    Write,
    /// Shut down both halves (`SHUT_RDWR`).
    Both,
}

impl Shutdown {
    #[must_use]
    const fn to_raw(self) -> i32 {
        match self {
            Self::Read => SHUT_RD,
            Self::Write => SHUT_WR,
            Self::Both => SHUT_RDWR,
        }
    }
}

/// Virtio-Vsock address (`(cid, port)`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct VsockAddr {
    /// Context ID (`VMADDR_CID_*` or guest CID).
    pub cid: u32,
    /// Vsock port number.
    pub port: u32,
}

impl VsockAddr {
    /// Creates a new [`VsockAddr`].
    #[must_use]
    pub const fn new(cid: u32, port: u32) -> Self {
        Self { cid, port }
    }
}

impl core::fmt::Display for VsockAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "vsock://{}:{}", self.cid, self.port)
    }
}

#[cfg(not(target_os = "none"))]
#[allow(non_camel_case_types)]
mod host_ffi {
    use core::ffi::{c_int, c_void};

    pub type socklen_t = u32;

    #[cfg(target_os = "macos")]
    pub type sa_family_t = u8;
    #[cfg(target_os = "linux")]
    pub type sa_family_t = u16;

    pub const AF_INET: c_int = 2;
    #[cfg(target_os = "macos")]
    pub const AF_INET6: c_int = 30;
    #[cfg(target_os = "linux")]
    pub const AF_INET6: c_int = 10;
    #[cfg(target_os = "linux")]
    pub const AF_VSOCK: c_int = 40;

    pub const SOCK_STREAM: c_int = 1;
    pub const SOCK_DGRAM: c_int = 2;
    #[cfg(target_os = "linux")]
    pub const SOCK_NONBLOCK: c_int = 0x800;
    #[cfg(target_os = "linux")]
    pub const SOCK_CLOEXEC: c_int = 0x80000;

    pub const IPPROTO_TCP: c_int = 6;
    pub const IPPROTO_UDP: c_int = 17;
    pub const IPPROTO_IPV6: c_int = 41;

    pub const TCP_NODELAY: c_int = 1;

    #[cfg(target_os = "macos")]
    pub const IPV6_V6ONLY: c_int = 27;
    #[cfg(target_os = "linux")]
    pub const IPV6_V6ONLY: c_int = 26;

    #[cfg(target_os = "macos")]
    pub const SOL_SOCKET: c_int = 0xffff;
    #[cfg(target_os = "linux")]
    pub const SOL_SOCKET: c_int = 1;

    #[cfg(target_os = "macos")]
    pub const SO_REUSEADDR: c_int = 0x0004;
    #[cfg(target_os = "linux")]
    pub const SO_REUSEADDR: c_int = 2;

    #[cfg(target_os = "macos")]
    pub const SO_REUSEPORT: c_int = 0x0200;
    #[cfg(target_os = "linux")]
    pub const SO_REUSEPORT: c_int = 15;

    #[cfg(target_os = "macos")]
    pub const SO_ERROR: c_int = 0x1007;
    #[cfg(target_os = "linux")]
    pub const SO_ERROR: c_int = 4;

    #[cfg(target_os = "macos")]
    pub const SO_SNDBUF: c_int = 0x1001;
    #[cfg(target_os = "linux")]
    pub const SO_SNDBUF: c_int = 7;

    #[cfg(target_os = "macos")]
    pub const SO_RCVBUF: c_int = 0x1002;
    #[cfg(target_os = "linux")]
    pub const SO_RCVBUF: c_int = 8;

    #[cfg(target_os = "macos")]
    pub const SO_NOSIGPIPE: c_int = 0x1022;

    #[cfg(target_os = "macos")]
    pub const MSG_NOSIGNAL: c_int = 0;
    #[cfg(target_os = "linux")]
    pub const MSG_NOSIGNAL: c_int = 0x4000;

    #[cfg(target_os = "macos")]
    pub const EINPROGRESS: c_int = 36;
    #[cfg(target_os = "linux")]
    pub const EINPROGRESS: c_int = 115;

    #[cfg(target_os = "macos")]
    pub const EAFNOSUPPORT: c_int = 47;
    #[cfg(target_os = "linux")]
    pub const EAFNOSUPPORT: c_int = 97;

    pub const EINVAL: c_int = 22;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct InAddr {
        pub s_addr: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct SockAddrIn {
        #[cfg(target_os = "macos")]
        pub sin_len: u8,
        pub sin_family: sa_family_t,
        pub sin_port: u16,
        pub sin_addr: InAddr,
        pub sin_zero: [u8; 8],
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct In6Addr {
        pub s6_addr: [u8; 16],
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct SockAddrIn6 {
        #[cfg(target_os = "macos")]
        pub sin6_len: u8,
        pub sin6_family: sa_family_t,
        pub sin6_port: u16,
        pub sin6_flowinfo: u32,
        pub sin6_addr: In6Addr,
        pub sin6_scope_id: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct SockAddrVm {
        #[cfg(target_os = "macos")]
        pub svm_len: u8,
        pub svm_family: sa_family_t,
        pub svm_reserved1: u16,
        pub svm_port: u32,
        pub svm_cid: u32,
        pub svm_flags: u8,
        pub svm_zero: [u8; 3],
    }

    #[repr(C, align(8))]
    #[derive(Clone, Copy)]
    pub struct SockAddrStorage {
        #[cfg(target_os = "macos")]
        pub ss_len: u8,
        pub ss_family: sa_family_t,
        pub ss_pad: [u8; 126],
    }

    unsafe extern "C" {
        pub fn getsockopt(
            socket: c_int,
            level: c_int,
            name: c_int,
            value: *mut c_void,
            option_len: *mut socklen_t,
        ) -> c_int;
        pub fn getsockname(
            socket: c_int,
            address: *mut c_void,
            address_len: *mut socklen_t,
        ) -> c_int;
        pub fn getpeername(
            socket: c_int,
            address: *mut c_void,
            address_len: *mut socklen_t,
        ) -> c_int;
        pub fn sendto(
            socket: c_int,
            buf: *const c_void,
            len: usize,
            flags: c_int,
            dest_addr: *const c_void,
            dest_len: socklen_t,
        ) -> isize;
        pub fn recvfrom(
            socket: c_int,
            buf: *mut c_void,
            len: usize,
            flags: c_int,
            src_addr: *mut c_void,
            src_len: *mut socklen_t,
        ) -> isize;
        pub fn shutdown(socket: c_int, how: c_int) -> c_int;
    }
}

fn set_fd_nonblocking(fd: i32, nonblocking: bool) -> Result<(), Errno> {
    #[cfg(target_os = "none")]
    {
        let flags = ::libc_rs::io::fcntl(fd, ::libc_rs::io::F_GETFL, 0).map_err(|e| Errno(e.0))?;
        let next = if nonblocking {
            flags | ::libc_rs::fs::O_NONBLOCK
        } else {
            flags & !::libc_rs::fs::O_NONBLOCK
        };
        ::libc_rs::io::fcntl(fd, ::libc_rs::io::F_SETFL, next as usize).map_err(|e| Errno(e.0))?;
        Ok(())
    }

    #[cfg(not(target_os = "none"))]
    {
        let flags = crate::checked(unsafe { libc::fcntl2(fd, libc::F_GETFL) })?;
        let next = if nonblocking { flags | libc::O_NONBLOCK } else { flags & !libc::O_NONBLOCK };
        crate::checked(unsafe { libc::fcntl3(fd, libc::F_SETFL, next) })?;
        Ok(())
    }
}

#[cfg(not(target_os = "none"))]
fn init_host_socket(fd: i32, nonblocking: bool) -> Result<(), Errno> {
    crate::checked(unsafe { libc::fcntl3(fd, libc::F_SETFD, libc::FD_CLOEXEC) })?;
    if nonblocking {
        set_fd_nonblocking(fd, true)?;
    }
    #[cfg(target_os = "macos")]
    {
        let one: i32 = 1;
        let _ = unsafe {
            libc::setsockopt(
                fd,
                host_ffi::SOL_SOCKET,
                host_ffi::SO_NOSIGPIPE,
                core::ptr::from_ref(&one).cast(),
                core::mem::size_of::<i32>() as u32,
            )
        };
    }
    Ok(())
}

fn create_ip_socket(addr: &SocketAddr, dgram: bool, nonblocking: bool) -> Result<i32, Errno> {
    #[cfg(target_os = "none")]
    {
        let domain = match addr {
            SocketAddr::V4(_) => ::libc_rs::net::AF_INET,
            SocketAddr::V6(_) => ::libc_rs::net::AF_INET6,
        };
        let mut ty = if dgram { ::libc_rs::net::SOCK_DGRAM } else { ::libc_rs::net::SOCK_STREAM };
        ty |= ::libc_rs::net::SOCK_CLOEXEC;
        if nonblocking {
            ty |= ::libc_rs::net::SOCK_NONBLOCK;
        }
        let proto = if dgram { ::libc_rs::net::IPPROTO_UDP } else { ::libc_rs::net::IPPROTO_TCP };
        let fd = ::libc_rs::net::socket(domain, ty, proto).map_err(|e| Errno(e.0))?;
        if matches!(addr, SocketAddr::V6(_)) {
            let one: i32 = 1;
            let _ = unsafe {
                ::libc_rs::net::setsockopt_raw(
                    fd,
                    ::libc_rs::net::IPPROTO_IPV6,
                    ::libc_rs::net::IPV6_V6ONLY,
                    core::ptr::from_ref(&one).cast::<u8>(),
                    core::mem::size_of::<i32>() as u32,
                )
            };
        }
        Ok(fd)
    }

    #[cfg(not(target_os = "none"))]
    {
        let domain = match addr {
            SocketAddr::V4(_) => host_ffi::AF_INET,
            SocketAddr::V6(_) => host_ffi::AF_INET6,
        };
        #[allow(unused_mut)]
        let mut ty = if dgram { host_ffi::SOCK_DGRAM } else { host_ffi::SOCK_STREAM };
        #[cfg(target_os = "linux")]
        {
            ty |= host_ffi::SOCK_CLOEXEC;
            if nonblocking {
                ty |= host_ffi::SOCK_NONBLOCK;
            }
        }
        let proto = if dgram { host_ffi::IPPROTO_UDP } else { host_ffi::IPPROTO_TCP };
        let fd = crate::checked(unsafe { libc::socket(domain, ty, proto) })?;
        #[cfg(not(target_os = "linux"))]
        if let Err(e) = init_host_socket(fd, nonblocking) {
            unsafe { libc::close(fd) };
            return Err(e);
        }
        if matches!(addr, SocketAddr::V6(_)) {
            let one: i32 = 1;
            let _ = unsafe {
                libc::setsockopt(
                    fd,
                    host_ffi::IPPROTO_IPV6,
                    host_ffi::IPV6_V6ONLY,
                    core::ptr::from_ref(&one).cast(),
                    core::mem::size_of::<i32>() as u32,
                )
            };
        }
        Ok(fd)
    }
}

fn set_reuseaddr(fd: i32, reuse: bool) -> Result<(), Errno> {
    let val: i32 = i32::from(reuse);
    #[cfg(target_os = "none")]
    {
        unsafe {
            ::libc_rs::net::setsockopt_raw(
                fd,
                ::libc_rs::net::SOL_SOCKET,
                ::libc_rs::net::SO_REUSEADDR,
                core::ptr::from_ref(&val).cast::<u8>(),
                core::mem::size_of::<i32>() as u32,
            )
            .map_err(|e| Errno(e.0))
        }
    }
    #[cfg(not(target_os = "none"))]
    {
        crate::checked(unsafe {
            libc::setsockopt(
                fd,
                host_ffi::SOL_SOCKET,
                host_ffi::SO_REUSEADDR,
                core::ptr::from_ref(&val).cast(),
                core::mem::size_of::<i32>() as u32,
            )
        })?;
        Ok(())
    }
}

fn set_reuseport(fd: i32, reuse: bool) -> Result<(), Errno> {
    let val: i32 = i32::from(reuse);
    #[cfg(target_os = "none")]
    {
        unsafe {
            ::libc_rs::net::setsockopt_raw(
                fd,
                ::libc_rs::net::SOL_SOCKET,
                ::libc_rs::net::SO_REUSEPORT,
                core::ptr::from_ref(&val).cast::<u8>(),
                core::mem::size_of::<i32>() as u32,
            )
            .map_err(|e| Errno(e.0))
        }
    }
    #[cfg(not(target_os = "none"))]
    {
        crate::checked(unsafe {
            libc::setsockopt(
                fd,
                host_ffi::SOL_SOCKET,
                host_ffi::SO_REUSEPORT,
                core::ptr::from_ref(&val).cast(),
                core::mem::size_of::<i32>() as u32,
            )
        })?;
        Ok(())
    }
}

fn bind_socket_addr(fd: i32, addr: &SocketAddr) -> Result<(), Errno> {
    #[cfg(target_os = "none")]
    {
        match addr {
            SocketAddr::V4(v4) => {
                let raw = ::libc_rs::net::SockAddrIn::new(v4.ip().octets(), v4.port());
                ::libc_rs::net::bind_in(fd, &raw).map_err(|e| Errno(e.0))
            }
            SocketAddr::V6(v6) => {
                let raw = ::libc_rs::net::SockAddrIn6::new(
                    v6.ip().octets(),
                    v6.port(),
                    v6.flowinfo(),
                    v6.scope_id(),
                );
                ::libc_rs::net::bind_in6(fd, &raw).map_err(|e| Errno(e.0))
            }
        }
    }

    #[cfg(not(target_os = "none"))]
    {
        match addr {
            SocketAddr::V4(v4) => {
                let raw = host_ffi::SockAddrIn {
                    #[cfg(target_os = "macos")]
                    sin_len: core::mem::size_of::<host_ffi::SockAddrIn>() as u8,
                    sin_family: host_ffi::AF_INET as host_ffi::sa_family_t,
                    sin_port: v4.port().to_be(),
                    sin_addr: host_ffi::InAddr { s_addr: u32::from_ne_bytes(v4.ip().octets()) },
                    sin_zero: [0; 8],
                };
                crate::checked(unsafe {
                    libc::bind(
                        fd,
                        core::ptr::from_ref(&raw).cast(),
                        core::mem::size_of::<host_ffi::SockAddrIn>() as u32,
                    )
                })?;
                Ok(())
            }
            SocketAddr::V6(v6) => {
                let raw = host_ffi::SockAddrIn6 {
                    #[cfg(target_os = "macos")]
                    sin6_len: core::mem::size_of::<host_ffi::SockAddrIn6>() as u8,
                    sin6_family: host_ffi::AF_INET6 as host_ffi::sa_family_t,
                    sin6_port: v6.port().to_be(),
                    sin6_flowinfo: v6.flowinfo(),
                    sin6_addr: host_ffi::In6Addr { s6_addr: v6.ip().octets() },
                    sin6_scope_id: v6.scope_id(),
                };
                crate::checked(unsafe {
                    libc::bind(
                        fd,
                        core::ptr::from_ref(&raw).cast(),
                        core::mem::size_of::<host_ffi::SockAddrIn6>() as u32,
                    )
                })?;
                Ok(())
            }
        }
    }
}

fn connect_socket_addr(fd: i32, addr: &SocketAddr) -> Result<(), Errno> {
    #[cfg(target_os = "none")]
    {
        match addr {
            SocketAddr::V4(v4) => {
                let raw = ::libc_rs::net::SockAddrIn::new(v4.ip().octets(), v4.port());
                ::libc_rs::net::connect_in(fd, &raw).map_err(|e| Errno(e.0))
            }
            SocketAddr::V6(v6) => {
                let raw = ::libc_rs::net::SockAddrIn6::new(
                    v6.ip().octets(),
                    v6.port(),
                    v6.flowinfo(),
                    v6.scope_id(),
                );
                ::libc_rs::net::connect_in6(fd, &raw).map_err(|e| Errno(e.0))
            }
        }
    }

    #[cfg(not(target_os = "none"))]
    {
        match addr {
            SocketAddr::V4(v4) => {
                let raw = host_ffi::SockAddrIn {
                    #[cfg(target_os = "macos")]
                    sin_len: core::mem::size_of::<host_ffi::SockAddrIn>() as u8,
                    sin_family: host_ffi::AF_INET as host_ffi::sa_family_t,
                    sin_port: v4.port().to_be(),
                    sin_addr: host_ffi::InAddr { s_addr: u32::from_ne_bytes(v4.ip().octets()) },
                    sin_zero: [0; 8],
                };
                crate::checked(unsafe {
                    libc::connect(
                        fd,
                        core::ptr::from_ref(&raw).cast(),
                        core::mem::size_of::<host_ffi::SockAddrIn>() as u32,
                    )
                })?;
                Ok(())
            }
            SocketAddr::V6(v6) => {
                let raw = host_ffi::SockAddrIn6 {
                    #[cfg(target_os = "macos")]
                    sin6_len: core::mem::size_of::<host_ffi::SockAddrIn6>() as u8,
                    sin6_family: host_ffi::AF_INET6 as host_ffi::sa_family_t,
                    sin6_port: v6.port().to_be(),
                    sin6_flowinfo: v6.flowinfo(),
                    sin6_addr: host_ffi::In6Addr { s6_addr: v6.ip().octets() },
                    sin6_scope_id: v6.scope_id(),
                };
                crate::checked(unsafe {
                    libc::connect(
                        fd,
                        core::ptr::from_ref(&raw).cast(),
                        core::mem::size_of::<host_ffi::SockAddrIn6>() as u32,
                    )
                })?;
                Ok(())
            }
        }
    }
}

fn query_socket_addr(fd: i32, peer: bool) -> Result<SocketAddr, Errno> {
    #[cfg(target_os = "none")]
    {
        let mut storage = ::libc_rs::net::SockAddrStorage::default();
        let mut len = core::mem::size_of::<::libc_rs::net::SockAddrStorage>() as u32;
        unsafe {
            if peer {
                ::libc_rs::net::getpeername_raw(
                    fd,
                    core::ptr::from_mut(&mut storage).cast::<u8>(),
                    core::ptr::from_mut(&mut len),
                )
                .map_err(|e| Errno(e.0))?;
            } else {
                ::libc_rs::net::getsockname_raw(
                    fd,
                    core::ptr::from_mut(&mut storage).cast::<u8>(),
                    core::ptr::from_mut(&mut len),
                )
                .map_err(|e| Errno(e.0))?;
            }
        }
        match i32::from(storage.ss_family) {
            ::libc_rs::net::AF_INET => {
                let sin = unsafe {
                    &*(core::ptr::from_ref(&storage).cast::<::libc_rs::net::SockAddrIn>())
                };
                Ok(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::from(sin.ip()), sin.port())))
            }
            ::libc_rs::net::AF_INET6 => {
                let sin6 = unsafe {
                    &*(core::ptr::from_ref(&storage).cast::<::libc_rs::net::SockAddrIn6>())
                };
                Ok(SocketAddr::V6(SocketAddrV6::new(
                    Ipv6Addr::from(sin6.ip()),
                    sin6.port(),
                    sin6.sin6_flowinfo,
                    sin6.sin6_scope_id,
                )))
            }
            _ => Err(Errno(::libc_rs::errno::EAFNOSUPPORT)),
        }
    }

    #[cfg(not(target_os = "none"))]
    {
        let mut storage: host_ffi::SockAddrStorage = unsafe { core::mem::zeroed() };
        let mut len = core::mem::size_of::<host_ffi::SockAddrStorage>() as u32;
        let rc = unsafe {
            if peer {
                host_ffi::getpeername(
                    fd,
                    core::ptr::from_mut(&mut storage).cast(),
                    core::ptr::from_mut(&mut len),
                )
            } else {
                host_ffi::getsockname(
                    fd,
                    core::ptr::from_mut(&mut storage).cast(),
                    core::ptr::from_mut(&mut len),
                )
            }
        };
        crate::checked(rc)?;
        decode_host_storage(&storage)
    }
}

#[cfg(not(target_os = "none"))]
fn decode_host_storage(storage: &host_ffi::SockAddrStorage) -> Result<SocketAddr, Errno> {
    match i32::from(storage.ss_family) {
        host_ffi::AF_INET => {
            let sin = unsafe { &*(core::ptr::from_ref(storage).cast::<host_ffi::SockAddrIn>()) };
            Ok(SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::from(sin.sin_addr.s_addr.to_ne_bytes()),
                u16::from_be(sin.sin_port),
            )))
        }
        host_ffi::AF_INET6 => {
            let sin6 = unsafe { &*(core::ptr::from_ref(storage).cast::<host_ffi::SockAddrIn6>()) };
            Ok(SocketAddr::V6(SocketAddrV6::new(
                Ipv6Addr::from(sin6.sin6_addr.s6_addr),
                u16::from_be(sin6.sin6_port),
                sin6.sin6_flowinfo,
                sin6.sin6_scope_id,
            )))
        }
        _ => Err(Errno(host_ffi::EAFNOSUPPORT)),
    }
}

fn query_so_error(fd: i32) -> Result<Option<Errno>, Errno> {
    #[cfg(target_os = "none")]
    {
        let code = ::libc_rs::net::get_so_error(fd).map_err(|e| Errno(e.0))?;
        if code == 0 {
            Ok(None)
        } else {
            Ok(Some(Errno(code)))
        }
    }

    #[cfg(not(target_os = "none"))]
    {
        let mut code: i32 = 0;
        let mut len = core::mem::size_of::<i32>() as u32;
        crate::checked(unsafe {
            host_ffi::getsockopt(
                fd,
                host_ffi::SOL_SOCKET,
                host_ffi::SO_ERROR,
                core::ptr::from_mut(&mut code).cast(),
                core::ptr::from_mut(&mut len),
            )
        })?;
        if code == 0 {
            Ok(None)
        } else {
            Ok(Some(Errno(code)))
        }
    }
}

fn read_fd(fd: i32, buf: &mut [u8]) -> Result<usize, Errno> {
    #[cfg(target_os = "none")]
    {
        ::libc_rs::io::read(fd, buf).map_err(|e| Errno(e.0))
    }

    #[cfg(not(target_os = "none"))]
    {
        let n = unsafe { libc::read(fd, buf.as_mut_ptr().cast(), buf.len()) };
        if n < 0 {
            Err(Errno::current())
        } else {
            Ok(n as usize)
        }
    }
}

fn write_fd(fd: i32, buf: &[u8]) -> Result<usize, Errno> {
    #[cfg(target_os = "none")]
    {
        ::libc_rs::net::send(fd, buf, ::libc_rs::net::MSG_NOSIGNAL).map_err(|e| Errno(e.0))
    }

    #[cfg(not(target_os = "none"))]
    {
        let n = unsafe { libc::send(fd, buf.as_ptr().cast(), buf.len(), host_ffi::MSG_NOSIGNAL) };
        if n < 0 {
            Err(Errno::current())
        } else {
            Ok(n as usize)
        }
    }
}

fn write_all_fd(fd: i32, mut buf: &[u8]) -> Result<(), Errno> {
    while !buf.is_empty() {
        match write_fd(fd, buf) {
            Ok(0) => {
                #[cfg(target_os = "none")]
                return Err(Errno(::libc_rs::errno::EPIPE));
                #[cfg(not(target_os = "none"))]
                return Err(Errno(libc::EPIPE));
            }
            Ok(n) => buf = &buf[n..],
            Err(e) if e.interrupted() => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

fn close_fd(fd: i32) {
    if fd >= 0 {
        #[cfg(target_os = "none")]
        let _ = ::libc_rs::io::close(fd);
        #[cfg(not(target_os = "none"))]
        unsafe {
            libc::close(fd);
        }
    }
}

/// An IPv4 or IPv6 TCP stream socket.
#[derive(Debug)]
pub struct TcpStream {
    fd: i32,
}

impl TcpStream {
    /// Opens a blocking TCP connection to `addr`.
    pub fn connect(addr: SocketAddr) -> Result<Self, Errno> {
        let fd = create_ip_socket(&addr, false, false)?;
        if let Err(e) = connect_socket_addr(fd, &addr) {
            close_fd(fd);
            return Err(e);
        }
        let stream = Self { fd };
        let _ = stream.set_nodelay(true);
        Ok(stream)
    }

    /// Initiates a nonblocking TCP connection to `addr` (`EINPROGRESS` is treated as in-flight success).
    pub fn connect_nonblocking(addr: SocketAddr) -> Result<Self, Errno> {
        let fd = create_ip_socket(&addr, false, true)?;
        if let Err(e) = connect_socket_addr(fd, &addr) {
            #[cfg(target_os = "none")]
            let in_progress = e.0 == ::libc_rs::errno::EINPROGRESS;
            #[cfg(not(target_os = "none"))]
            let in_progress = e.0 == host_ffi::EINPROGRESS;
            if !in_progress {
                close_fd(fd);
                return Err(e);
            }
        }
        let stream = Self { fd };
        let _ = stream.set_nodelay(true);
        Ok(stream)
    }

    /// Adopts an existing raw socket descriptor, taking exclusive ownership.
    ///
    /// # Safety
    ///
    /// `fd` must be a valid open socket file descriptor not owned elsewhere.
    #[must_use]
    pub const unsafe fn from_raw_fd(fd: i32) -> Self {
        Self { fd }
    }

    /// Adopts an existing raw socket descriptor, taking exclusive ownership.
    ///
    /// # Safety
    ///
    /// `fd` must be a valid open socket file descriptor not owned elsewhere.
    #[must_use]
    pub const unsafe fn from_raw(fd: i32) -> Self {
        Self { fd }
    }

    /// Returns the underlying raw descriptor without transferring ownership.
    #[must_use]
    pub const fn as_raw_fd(&self) -> i32 {
        self.fd
    }

    /// Returns the underlying raw descriptor without transferring ownership.
    #[must_use]
    pub const fn raw(&self) -> i32 {
        self.fd
    }

    /// Consumes the stream and returns the raw descriptor without closing it.
    #[must_use]
    pub const fn into_raw_fd(self) -> i32 {
        let fd = self.fd;
        core::mem::forget(self);
        fd
    }

    /// Sets or clears nonblocking mode (`O_NONBLOCK`) on this socket.
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), Errno> {
        set_fd_nonblocking(self.fd, nonblocking)
    }

    /// Enables or disables `TCP_NODELAY` (Nagle's algorithm).
    pub fn set_nodelay(&self, nodelay: bool) -> Result<(), Errno> {
        #[cfg(target_os = "none")]
        {
            ::libc_rs::net::set_tcp_nodelay(self.fd, nodelay).map_err(|e| Errno(e.0))
        }

        #[cfg(not(target_os = "none"))]
        {
            let val: i32 = i32::from(nodelay);
            crate::checked(unsafe {
                libc::setsockopt(
                    self.fd,
                    host_ffi::IPPROTO_TCP,
                    host_ffi::TCP_NODELAY,
                    core::ptr::from_ref(&val).cast(),
                    core::mem::size_of::<i32>() as u32,
                )
            })?;
            Ok(())
        }
    }

    /// Queries and clears any pending `SO_ERROR` on the socket.
    pub fn take_error(&self) -> Result<Option<Errno>, Errno> {
        query_so_error(self.fd)
    }

    /// Checks whether an in-flight nonblocking connect has succeeded (`Ok(())`) or failed (`Err(Errno)`).
    pub fn connect_error(&self) -> Result<(), Errno> {
        match self.take_error()? {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }

    /// Returns the local socket address of this connection.
    pub fn local_addr(&self) -> Result<SocketAddr, Errno> {
        query_socket_addr(self.fd, false)
    }

    /// Returns the remote peer socket address of this connection.
    pub fn peer_addr(&self) -> Result<SocketAddr, Errno> {
        query_socket_addr(self.fd, true)
    }

    /// Reads bytes from the stream into `buf`.
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, Errno> {
        read_fd(self.fd, buf)
    }

    /// Receives bytes from the stream into `buf`.
    pub fn recv(&self, buf: &mut [u8]) -> Result<usize, Errno> {
        read_fd(self.fd, buf)
    }

    /// Writes bytes from `buf` to the stream.
    pub fn write(&self, buf: &[u8]) -> Result<usize, Errno> {
        write_fd(self.fd, buf)
    }

    /// Sends bytes from `buf` on the stream.
    pub fn send(&self, buf: &[u8]) -> Result<usize, Errno> {
        write_fd(self.fd, buf)
    }

    /// Writes all bytes in `buf` to the stream.
    pub fn write_all(&self, buf: &[u8]) -> Result<(), Errno> {
        write_all_fd(self.fd, buf)
    }

    /// Shuts down the read, write, or both halves of this connection.
    pub fn shutdown(&self, how: Shutdown) -> Result<(), Errno> {
        #[cfg(target_os = "none")]
        {
            ::libc_rs::net::shutdown(self.fd, how.to_raw()).map_err(|e| Errno(e.0))
        }
        #[cfg(not(target_os = "none"))]
        {
            crate::checked(unsafe { host_ffi::shutdown(self.fd, how.to_raw()) })?;
            Ok(())
        }
    }

    /// Duplicates the underlying socket descriptor into a new [`TcpStream`].
    pub fn try_clone(&self) -> Result<Self, Errno> {
        #[cfg(target_os = "none")]
        {
            let fd = ::libc_rs::io::fcntl(self.fd, ::libc_rs::io::F_DUPFD_CLOEXEC, 0)
                .map_err(|e| Errno(e.0))?;
            Ok(Self { fd })
        }
        #[cfg(not(target_os = "none"))]
        {
            let fd = crate::checked(unsafe { libc::fcntl3(self.fd, libc::F_DUPFD_CLOEXEC, 0) })?;
            Ok(Self { fd })
        }
    }
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        close_fd(self.fd);
    }
}

/// An IPv4 or IPv6 TCP listening socket.
#[derive(Debug)]
pub struct TcpListener {
    fd: i32,
}

impl TcpListener {
    /// Binds a TCP listener to `addr` with a default backlog of `128`.
    pub fn bind(addr: SocketAddr) -> Result<Self, Errno> {
        Self::bind_with_backlog(addr, 128, false, false)
    }

    /// Binds a nonblocking TCP listener to `addr` with a default backlog of `128`.
    pub fn bind_nonblocking(addr: SocketAddr) -> Result<Self, Errno> {
        Self::bind_with_backlog(addr, 128, false, true)
    }

    /// Binds a TCP listener to `addr` with explicit `backlog`, `reuse_port`, and `nonblocking` settings.
    pub fn bind_with_backlog(
        addr: SocketAddr,
        backlog: i32,
        reuse_port: bool,
        nonblocking: bool,
    ) -> Result<Self, Errno> {
        let fd = create_ip_socket(&addr, false, nonblocking)?;
        if let Err(e) = (|| -> Result<(), Errno> {
            set_reuseaddr(fd, true)?;
            if reuse_port {
                set_reuseport(fd, true)?;
            }
            bind_socket_addr(fd, &addr)?;
            #[cfg(target_os = "none")]
            {
                ::libc_rs::net::listen(fd, backlog).map_err(|e| Errno(e.0))?;
            }
            #[cfg(not(target_os = "none"))]
            {
                crate::checked(unsafe { libc::listen(fd, backlog) })?;
            }
            Ok(())
        })() {
            close_fd(fd);
            return Err(e);
        }
        Ok(Self { fd })
    }

    /// Adopts an existing raw socket descriptor, taking exclusive ownership.
    ///
    /// # Safety
    ///
    /// `fd` must be a valid open socket file descriptor not owned elsewhere.
    #[must_use]
    pub const unsafe fn from_raw_fd(fd: i32) -> Self {
        Self { fd }
    }

    /// Returns the underlying raw descriptor without transferring ownership.
    #[must_use]
    pub const fn as_raw_fd(&self) -> i32 {
        self.fd
    }

    /// Returns the underlying raw descriptor without transferring ownership.
    #[must_use]
    pub const fn raw(&self) -> i32 {
        self.fd
    }

    /// Consumes the listener and returns the raw descriptor without closing it.
    #[must_use]
    pub const fn into_raw_fd(self) -> i32 {
        let fd = self.fd;
        core::mem::forget(self);
        fd
    }

    /// Sets or clears nonblocking mode (`O_NONBLOCK`) on the listener.
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), Errno> {
        set_fd_nonblocking(self.fd, nonblocking)
    }

    /// Returns the local address this listener is bound to.
    pub fn local_addr(&self) -> Result<SocketAddr, Errno> {
        query_socket_addr(self.fd, false)
    }

    /// Accepts a new incoming connection, returning the connected [`TcpStream`] and peer [`SocketAddr`].
    pub fn accept(&self) -> Result<(TcpStream, SocketAddr), Errno> {
        self.accept_inner(false)
    }

    /// Accepts a new incoming connection with `O_NONBLOCK` and `FD_CLOEXEC` enabled on the accepted stream.
    pub fn accept_nonblocking(&self) -> Result<(TcpStream, SocketAddr), Errno> {
        self.accept_inner(true)
    }

    fn accept_inner(&self, nonblocking: bool) -> Result<(TcpStream, SocketAddr), Errno> {
        #[cfg(target_os = "none")]
        {
            let mut storage = ::libc_rs::net::SockAddrStorage::default();
            let mut len = core::mem::size_of::<::libc_rs::net::SockAddrStorage>() as u32;
            let mut flags = ::libc_rs::net::SOCK_CLOEXEC;
            if nonblocking {
                flags |= ::libc_rs::net::SOCK_NONBLOCK;
            }
            let client_fd = unsafe {
                ::libc_rs::net::accept4_raw(
                    self.fd,
                    core::ptr::from_mut(&mut storage).cast::<u8>(),
                    core::ptr::from_mut(&mut len),
                    flags,
                )
                .map_err(|e| Errno(e.0))?
            };
            let peer = match i32::from(storage.ss_family) {
                ::libc_rs::net::AF_INET => {
                    let sin = unsafe {
                        &*(core::ptr::from_ref(&storage).cast::<::libc_rs::net::SockAddrIn>())
                    };
                    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::from(sin.ip()), sin.port()))
                }
                ::libc_rs::net::AF_INET6 => {
                    let sin6 = unsafe {
                        &*(core::ptr::from_ref(&storage).cast::<::libc_rs::net::SockAddrIn6>())
                    };
                    SocketAddr::V6(SocketAddrV6::new(
                        Ipv6Addr::from(sin6.ip()),
                        sin6.port(),
                        sin6.sin6_flowinfo,
                        sin6.sin6_scope_id,
                    ))
                }
                _ => {
                    close_fd(client_fd);
                    return Err(Errno(::libc_rs::errno::EAFNOSUPPORT));
                }
            };
            let stream = TcpStream { fd: client_fd };
            let _ = stream.set_nodelay(true);
            Ok((stream, peer))
        }

        #[cfg(not(target_os = "none"))]
        {
            let mut storage: host_ffi::SockAddrStorage = unsafe { core::mem::zeroed() };
            let mut len = core::mem::size_of::<host_ffi::SockAddrStorage>() as u32;
            let client_fd = crate::checked(unsafe {
                libc::accept(
                    self.fd,
                    core::ptr::from_mut(&mut storage).cast(),
                    core::ptr::from_mut(&mut len),
                )
            })?;
            if let Err(e) = init_host_socket(client_fd, nonblocking) {
                close_fd(client_fd);
                return Err(e);
            }
            let peer = match decode_host_storage(&storage) {
                Ok(addr) => addr,
                Err(e) => {
                    close_fd(client_fd);
                    return Err(e);
                }
            };
            let stream = TcpStream { fd: client_fd };
            let _ = stream.set_nodelay(true);
            Ok((stream, peer))
        }
    }
}

impl Drop for TcpListener {
    fn drop(&mut self) {
        close_fd(self.fd);
    }
}

/// An IPv4 or IPv6 UDP datagram socket.
#[derive(Debug)]
pub struct UdpSocket {
    fd: i32,
}

impl UdpSocket {
    /// Binds a UDP socket to `addr`.
    pub fn bind(addr: SocketAddr) -> Result<Self, Errno> {
        Self::bind_internal(addr, false)
    }

    /// Binds a nonblocking UDP socket to `addr`.
    pub fn bind_nonblocking(addr: SocketAddr) -> Result<Self, Errno> {
        Self::bind_internal(addr, true)
    }

    fn bind_internal(addr: SocketAddr, nonblocking: bool) -> Result<Self, Errno> {
        let fd = create_ip_socket(&addr, true, nonblocking)?;
        if let Err(e) = bind_socket_addr(fd, &addr) {
            close_fd(fd);
            return Err(e);
        }
        Ok(Self { fd })
    }

    /// Adopts an existing raw socket descriptor, taking exclusive ownership.
    ///
    /// # Safety
    ///
    /// `fd` must be a valid open socket file descriptor not owned elsewhere.
    #[must_use]
    pub const unsafe fn from_raw_fd(fd: i32) -> Self {
        Self { fd }
    }

    /// Returns the underlying raw descriptor without transferring ownership.
    #[must_use]
    pub const fn as_raw_fd(&self) -> i32 {
        self.fd
    }

    /// Returns the underlying raw descriptor without transferring ownership.
    #[must_use]
    pub const fn raw(&self) -> i32 {
        self.fd
    }

    /// Consumes the socket and returns the raw descriptor without closing it.
    #[must_use]
    pub const fn into_raw_fd(self) -> i32 {
        let fd = self.fd;
        core::mem::forget(self);
        fd
    }

    /// Sets or clears nonblocking mode (`O_NONBLOCK`) on the socket.
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), Errno> {
        set_fd_nonblocking(self.fd, nonblocking)
    }

    /// Connects this UDP socket to a default peer `addr`.
    pub fn connect(&self, addr: SocketAddr) -> Result<(), Errno> {
        connect_socket_addr(self.fd, &addr)
    }

    /// Returns the local socket address of this UDP socket.
    pub fn local_addr(&self) -> Result<SocketAddr, Errno> {
        query_socket_addr(self.fd, false)
    }

    /// Returns the connected peer address of this UDP socket, if any.
    pub fn peer_addr(&self) -> Result<SocketAddr, Errno> {
        query_socket_addr(self.fd, true)
    }

    /// Queries and clears any pending `SO_ERROR` on the socket.
    pub fn take_error(&self) -> Result<Option<Errno>, Errno> {
        query_so_error(self.fd)
    }

    /// Sends a datagram to the connected peer.
    pub fn send(&self, buf: &[u8]) -> Result<usize, Errno> {
        write_fd(self.fd, buf)
    }

    /// Receives a datagram from the connected peer into `buf`.
    pub fn recv(&self, buf: &mut [u8]) -> Result<usize, Errno> {
        read_fd(self.fd, buf)
    }

    /// Sends a datagram to `target`.
    pub fn send_to(&self, buf: &[u8], target: SocketAddr) -> Result<usize, Errno> {
        #[cfg(target_os = "none")]
        {
            match target {
                SocketAddr::V4(v4) => {
                    let raw = ::libc_rs::net::SockAddrIn::new(v4.ip().octets(), v4.port());
                    ::libc_rs::net::sendto_in(self.fd, buf, ::libc_rs::net::MSG_NOSIGNAL, &raw)
                        .map_err(|e| Errno(e.0))
                }
                SocketAddr::V6(v6) => {
                    let raw = ::libc_rs::net::SockAddrIn6::new(
                        v6.ip().octets(),
                        v6.port(),
                        v6.flowinfo(),
                        v6.scope_id(),
                    );
                    ::libc_rs::net::sendto_in6(self.fd, buf, ::libc_rs::net::MSG_NOSIGNAL, &raw)
                        .map_err(|e| Errno(e.0))
                }
            }
        }

        #[cfg(not(target_os = "none"))]
        {
            match target {
                SocketAddr::V4(v4) => {
                    let raw = host_ffi::SockAddrIn {
                        #[cfg(target_os = "macos")]
                        sin_len: core::mem::size_of::<host_ffi::SockAddrIn>() as u8,
                        sin_family: host_ffi::AF_INET as host_ffi::sa_family_t,
                        sin_port: v4.port().to_be(),
                        sin_addr: host_ffi::InAddr { s_addr: u32::from_ne_bytes(v4.ip().octets()) },
                        sin_zero: [0; 8],
                    };
                    let n = unsafe {
                        host_ffi::sendto(
                            self.fd,
                            buf.as_ptr().cast(),
                            buf.len(),
                            host_ffi::MSG_NOSIGNAL,
                            core::ptr::from_ref(&raw).cast(),
                            core::mem::size_of::<host_ffi::SockAddrIn>() as u32,
                        )
                    };
                    if n < 0 {
                        Err(Errno::current())
                    } else {
                        Ok(n as usize)
                    }
                }
                SocketAddr::V6(v6) => {
                    let raw = host_ffi::SockAddrIn6 {
                        #[cfg(target_os = "macos")]
                        sin6_len: core::mem::size_of::<host_ffi::SockAddrIn6>() as u8,
                        sin6_family: host_ffi::AF_INET6 as host_ffi::sa_family_t,
                        sin6_port: v6.port().to_be(),
                        sin6_flowinfo: v6.flowinfo(),
                        sin6_addr: host_ffi::In6Addr { s6_addr: v6.ip().octets() },
                        sin6_scope_id: v6.scope_id(),
                    };
                    let n = unsafe {
                        host_ffi::sendto(
                            self.fd,
                            buf.as_ptr().cast(),
                            buf.len(),
                            host_ffi::MSG_NOSIGNAL,
                            core::ptr::from_ref(&raw).cast(),
                            core::mem::size_of::<host_ffi::SockAddrIn6>() as u32,
                        )
                    };
                    if n < 0 {
                        Err(Errno::current())
                    } else {
                        Ok(n as usize)
                    }
                }
            }
        }
    }

    /// Receives a single datagram into `buf`, returning `(bytes_read, sender_addr)`.
    pub fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr), Errno> {
        #[cfg(target_os = "none")]
        {
            let local = self.local_addr()?;
            match local {
                SocketAddr::V4(_) => {
                    let (n, sin) =
                        ::libc_rs::net::recvfrom_in(self.fd, buf, 0).map_err(|e| Errno(e.0))?;
                    Ok((n, SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::from(sin.ip()), sin.port()))))
                }
                SocketAddr::V6(_) => {
                    let (n, sin6) =
                        ::libc_rs::net::recvfrom_in6(self.fd, buf, 0).map_err(|e| Errno(e.0))?;
                    Ok((
                        n,
                        SocketAddr::V6(SocketAddrV6::new(
                            Ipv6Addr::from(sin6.ip()),
                            sin6.port(),
                            sin6.sin6_flowinfo,
                            sin6.sin6_scope_id,
                        )),
                    ))
                }
            }
        }

        #[cfg(not(target_os = "none"))]
        {
            let mut storage: host_ffi::SockAddrStorage = unsafe { core::mem::zeroed() };
            let mut len = core::mem::size_of::<host_ffi::SockAddrStorage>() as u32;
            let n = unsafe {
                host_ffi::recvfrom(
                    self.fd,
                    buf.as_mut_ptr().cast(),
                    buf.len(),
                    0,
                    core::ptr::from_mut(&mut storage).cast(),
                    core::ptr::from_mut(&mut len),
                )
            };
            if n < 0 {
                return Err(Errno::current());
            }
            let addr = decode_host_storage(&storage)?;
            Ok((n as usize, addr))
        }
    }
}

impl Drop for UdpSocket {
    fn drop(&mut self) {
        close_fd(self.fd);
    }
}

fn create_vsock_socket(nonblocking: bool) -> Result<i32, Errno> {
    #[cfg(target_os = "none")]
    {
        let mut ty = ::libc_rs::net::SOCK_STREAM | ::libc_rs::net::SOCK_CLOEXEC;
        if nonblocking {
            ty |= ::libc_rs::net::SOCK_NONBLOCK;
        }
        ::libc_rs::net::socket(::libc_rs::net::AF_VSOCK, ty, 0).map_err(|e| Errno(e.0))
    }

    #[cfg(all(not(target_os = "none"), target_os = "linux"))]
    {
        let mut ty = host_ffi::SOCK_STREAM | host_ffi::SOCK_CLOEXEC;
        if nonblocking {
            ty |= host_ffi::SOCK_NONBLOCK;
        }
        crate::checked(unsafe { libc::socket(host_ffi::AF_VSOCK, ty, 0) })
    }

    #[cfg(all(not(target_os = "none"), target_os = "macos"))]
    {
        let _ = nonblocking;
        Err(Errno(host_ffi::EAFNOSUPPORT))
    }
}

fn query_vsock_addr(fd: i32, peer: bool) -> Result<VsockAddr, Errno> {
    #[cfg(target_os = "none")]
    {
        let mut vm = ::libc_rs::net::SockAddrVm::default();
        let mut len = core::mem::size_of::<::libc_rs::net::SockAddrVm>() as u32;
        unsafe {
            if peer {
                ::libc_rs::net::getpeername_raw(
                    fd,
                    core::ptr::from_mut(&mut vm).cast::<u8>(),
                    core::ptr::from_mut(&mut len),
                )
                .map_err(|e| Errno(e.0))?;
            } else {
                ::libc_rs::net::getsockname_raw(
                    fd,
                    core::ptr::from_mut(&mut vm).cast::<u8>(),
                    core::ptr::from_mut(&mut len),
                )
                .map_err(|e| Errno(e.0))?;
            }
        }
        Ok(VsockAddr { cid: vm.svm_cid, port: vm.svm_port })
    }

    #[cfg(all(not(target_os = "none"), target_os = "linux"))]
    {
        let mut vm = host_ffi::SockAddrVm::default();
        let mut len = core::mem::size_of::<host_ffi::SockAddrVm>() as u32;
        let rc = unsafe {
            if peer {
                host_ffi::getpeername(
                    fd,
                    core::ptr::from_mut(&mut vm).cast(),
                    core::ptr::from_mut(&mut len),
                )
            } else {
                host_ffi::getsockname(
                    fd,
                    core::ptr::from_mut(&mut vm).cast(),
                    core::ptr::from_mut(&mut len),
                )
            }
        };
        crate::checked(rc)?;
        Ok(VsockAddr { cid: vm.svm_cid, port: vm.svm_port })
    }

    #[cfg(all(not(target_os = "none"), target_os = "macos"))]
    {
        let _ = (fd, peer);
        Err(Errno(host_ffi::EAFNOSUPPORT))
    }
}

/// A Virtio-Vsock (`AF_VSOCK`) stream socket.
#[derive(Debug)]
pub struct VsockStream {
    fd: i32,
}

impl VsockStream {
    /// Connects a blocking vsock stream to `addr`.
    pub fn connect(addr: VsockAddr) -> Result<Self, Errno> {
        Self::connect_inner(addr, false)
    }

    /// Initiates a nonblocking vsock connection to `addr` (`EINPROGRESS` is treated as in-flight success).
    pub fn connect_nonblocking(addr: VsockAddr) -> Result<Self, Errno> {
        Self::connect_inner(addr, true)
    }

    fn connect_inner(addr: VsockAddr, nonblocking: bool) -> Result<Self, Errno> {
        let fd = create_vsock_socket(nonblocking)?;

        #[cfg(target_os = "none")]
        {
            let vm = ::libc_rs::net::SockAddrVm::new(addr.cid, addr.port);
            if let Err(e) = ::libc_rs::net::connect_vsock(fd, &vm) {
                if !(nonblocking && e.0 == ::libc_rs::errno::EINPROGRESS) {
                    close_fd(fd);
                    return Err(Errno(e.0));
                }
            }
            Ok(Self { fd })
        }

        #[cfg(all(not(target_os = "none"), target_os = "linux"))]
        {
            let vm = host_ffi::SockAddrVm {
                svm_family: host_ffi::AF_VSOCK as u16,
                svm_reserved1: 0,
                svm_port: addr.port,
                svm_cid: addr.cid,
                svm_flags: 0,
                svm_zero: [0; 3],
            };
            let rc = unsafe {
                libc::connect(
                    fd,
                    core::ptr::from_ref(&vm).cast(),
                    core::mem::size_of::<host_ffi::SockAddrVm>() as u32,
                )
            };
            if rc < 0 {
                let err = Errno::current();
                if !(nonblocking && err.0 == host_ffi::EINPROGRESS) {
                    close_fd(fd);
                    return Err(err);
                }
            }
            Ok(Self { fd })
        }

        #[cfg(all(not(target_os = "none"), target_os = "macos"))]
        {
            let _ = addr;
            close_fd(fd);
            Err(Errno(host_ffi::EAFNOSUPPORT))
        }
    }

    /// Adopts an existing raw vsock descriptor, taking exclusive ownership.
    ///
    /// # Safety
    ///
    /// `fd` must be a valid open socket file descriptor not owned elsewhere.
    #[must_use]
    pub const unsafe fn from_raw_fd(fd: i32) -> Self {
        Self { fd }
    }

    /// Returns the underlying raw descriptor without transferring ownership.
    #[must_use]
    pub const fn as_raw_fd(&self) -> i32 {
        self.fd
    }

    /// Returns the underlying raw descriptor without transferring ownership.
    #[must_use]
    pub const fn raw(&self) -> i32 {
        self.fd
    }

    /// Consumes the stream and returns the raw descriptor without closing it.
    #[must_use]
    pub const fn into_raw_fd(self) -> i32 {
        let fd = self.fd;
        core::mem::forget(self);
        fd
    }

    /// Sets or clears nonblocking mode (`O_NONBLOCK`) on this socket.
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), Errno> {
        set_fd_nonblocking(self.fd, nonblocking)
    }

    /// Queries and clears any pending `SO_ERROR` on the socket.
    pub fn take_error(&self) -> Result<Option<Errno>, Errno> {
        query_so_error(self.fd)
    }

    /// Returns the local [`VsockAddr`] of this socket.
    pub fn local_addr(&self) -> Result<VsockAddr, Errno> {
        query_vsock_addr(self.fd, false)
    }

    /// Returns the remote peer [`VsockAddr`] of this socket.
    pub fn peer_addr(&self) -> Result<VsockAddr, Errno> {
        query_vsock_addr(self.fd, true)
    }

    /// Reads bytes from the stream into `buf`.
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, Errno> {
        read_fd(self.fd, buf)
    }

    /// Receives bytes from the stream into `buf`.
    pub fn recv(&self, buf: &mut [u8]) -> Result<usize, Errno> {
        read_fd(self.fd, buf)
    }

    /// Writes bytes from `buf` to the stream.
    pub fn write(&self, buf: &[u8]) -> Result<usize, Errno> {
        write_fd(self.fd, buf)
    }

    /// Sends bytes from `buf` on the stream.
    pub fn send(&self, buf: &[u8]) -> Result<usize, Errno> {
        write_fd(self.fd, buf)
    }

    /// Writes all bytes in `buf` to the stream.
    pub fn write_all(&self, buf: &[u8]) -> Result<(), Errno> {
        write_all_fd(self.fd, buf)
    }

    /// Shuts down the read, write, or both halves of this connection.
    pub fn shutdown(&self, how: Shutdown) -> Result<(), Errno> {
        #[cfg(target_os = "none")]
        {
            ::libc_rs::net::shutdown(self.fd, how.to_raw()).map_err(|e| Errno(e.0))
        }
        #[cfg(not(target_os = "none"))]
        {
            crate::checked(unsafe { host_ffi::shutdown(self.fd, how.to_raw()) })?;
            Ok(())
        }
    }
}

impl Drop for VsockStream {
    fn drop(&mut self) {
        close_fd(self.fd);
    }
}

/// A Virtio-Vsock (`AF_VSOCK`) listening socket.
#[derive(Debug)]
pub struct VsockListener {
    fd: i32,
}

impl VsockListener {
    /// Binds a vsock listener to `addr` with a default backlog of `128`.
    pub fn bind(addr: VsockAddr) -> Result<Self, Errno> {
        Self::bind_with_backlog(addr, 128, false)
    }

    /// Binds a nonblocking vsock listener to `addr` with a default backlog of `128`.
    pub fn bind_nonblocking(addr: VsockAddr) -> Result<Self, Errno> {
        Self::bind_with_backlog(addr, 128, true)
    }

    /// Binds a vsock listener to `addr` with explicit `backlog` and `nonblocking` settings.
    pub fn bind_with_backlog(
        addr: VsockAddr,
        backlog: i32,
        nonblocking: bool,
    ) -> Result<Self, Errno> {
        let fd = create_vsock_socket(nonblocking)?;

        #[cfg(target_os = "none")]
        {
            let vm = ::libc_rs::net::SockAddrVm::new(addr.cid, addr.port);
            if let Err(e) = ::libc_rs::net::bind_vsock(fd, &vm)
                .and_then(|()| ::libc_rs::net::listen(fd, backlog))
            {
                close_fd(fd);
                return Err(Errno(e.0));
            }
            Ok(Self { fd })
        }

        #[cfg(all(not(target_os = "none"), target_os = "linux"))]
        {
            let vm = host_ffi::SockAddrVm {
                svm_family: host_ffi::AF_VSOCK as u16,
                svm_reserved1: 0,
                svm_port: addr.port,
                svm_cid: addr.cid,
                svm_flags: 0,
                svm_zero: [0; 3],
            };
            let res = (|| -> Result<(), Errno> {
                crate::checked(unsafe {
                    libc::bind(
                        fd,
                        core::ptr::from_ref(&vm).cast(),
                        core::mem::size_of::<host_ffi::SockAddrVm>() as u32,
                    )
                })?;
                crate::checked(unsafe { libc::listen(fd, backlog) })?;
                Ok(())
            })();
            if let Err(e) = res {
                close_fd(fd);
                return Err(e);
            }
            Ok(Self { fd })
        }

        #[cfg(all(not(target_os = "none"), target_os = "macos"))]
        {
            let _ = (addr, backlog);
            close_fd(fd);
            Err(Errno(host_ffi::EAFNOSUPPORT))
        }
    }

    /// Adopts an existing raw vsock listener descriptor, taking exclusive ownership.
    ///
    /// # Safety
    ///
    /// `fd` must be a valid open socket file descriptor not owned elsewhere.
    #[must_use]
    pub const unsafe fn from_raw_fd(fd: i32) -> Self {
        Self { fd }
    }

    /// Returns the underlying raw descriptor without transferring ownership.
    #[must_use]
    pub const fn as_raw_fd(&self) -> i32 {
        self.fd
    }

    /// Returns the underlying raw descriptor without transferring ownership.
    #[must_use]
    pub const fn raw(&self) -> i32 {
        self.fd
    }

    /// Consumes the listener and returns the raw descriptor without closing it.
    #[must_use]
    pub const fn into_raw_fd(self) -> i32 {
        let fd = self.fd;
        core::mem::forget(self);
        fd
    }

    /// Sets or clears nonblocking mode (`O_NONBLOCK`) on the listener.
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), Errno> {
        set_fd_nonblocking(self.fd, nonblocking)
    }

    /// Returns the local [`VsockAddr`] this listener is bound to.
    pub fn local_addr(&self) -> Result<VsockAddr, Errno> {
        query_vsock_addr(self.fd, false)
    }

    /// Accepts an incoming vsock connection.
    pub fn accept(&self) -> Result<(VsockStream, VsockAddr), Errno> {
        self.accept_inner(false)
    }

    /// Accepts an incoming vsock connection with `O_NONBLOCK` and `FD_CLOEXEC` enabled.
    pub fn accept_nonblocking(&self) -> Result<(VsockStream, VsockAddr), Errno> {
        self.accept_inner(true)
    }

    fn accept_inner(&self, nonblocking: bool) -> Result<(VsockStream, VsockAddr), Errno> {
        #[cfg(target_os = "none")]
        {
            let mut vm = ::libc_rs::net::SockAddrVm::default();
            let mut len = core::mem::size_of::<::libc_rs::net::SockAddrVm>() as u32;
            let mut flags = ::libc_rs::net::SOCK_CLOEXEC;
            if nonblocking {
                flags |= ::libc_rs::net::SOCK_NONBLOCK;
            }
            let client_fd = unsafe {
                ::libc_rs::net::accept4_raw(
                    self.fd,
                    core::ptr::from_mut(&mut vm).cast::<u8>(),
                    core::ptr::from_mut(&mut len),
                    flags,
                )
                .map_err(|e| Errno(e.0))?
            };
            Ok((VsockStream { fd: client_fd }, VsockAddr { cid: vm.svm_cid, port: vm.svm_port }))
        }

        #[cfg(all(not(target_os = "none"), target_os = "linux"))]
        {
            let mut vm = host_ffi::SockAddrVm::default();
            let mut len = core::mem::size_of::<host_ffi::SockAddrVm>() as u32;
            let client_fd = crate::checked(unsafe {
                libc::accept(
                    self.fd,
                    core::ptr::from_mut(&mut vm).cast(),
                    core::ptr::from_mut(&mut len),
                )
            })?;
            if let Err(e) = init_host_socket(client_fd, nonblocking) {
                close_fd(client_fd);
                return Err(e);
            }
            Ok((VsockStream { fd: client_fd }, VsockAddr { cid: vm.svm_cid, port: vm.svm_port }))
        }

        #[cfg(all(not(target_os = "none"), target_os = "macos"))]
        {
            let _ = nonblocking;
            Err(Errno(host_ffi::EAFNOSUPPORT))
        }
    }
}

impl Drop for VsockListener {
    fn drop(&mut self) {
        close_fd(self.fd);
    }
}

#[cfg(test)]
mod net_loopback_tests {
    use super::*;
    use crate::epoll::{Epoll, EpollEvent, EPOLLIN, EPOLLOUT};

    #[test]
    fn tcp_loopback_with_epoll() {
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0));
        let listener = TcpListener::bind_nonblocking(addr).expect("TcpListener::bind");
        let bound = listener.local_addr().expect("listener.local_addr");
        let ep = Epoll::new().expect("Epoll::new");
        ep.add(listener.as_raw_fd(), 1, EPOLLIN).expect("ep.add listener");
        let client = TcpStream::connect_nonblocking(bound).expect("TcpStream::connect");
        ep.add(client.as_raw_fd(), 2, EPOLLOUT).expect("ep.add client");
        let mut events = [EpollEvent { events: 0, token: 0 }; 4];
        let mut accepted = None;
        let mut client_writable = false;
        for _ in 0..20 {
            let n = ep.wait(&mut events, 100).expect("ep.wait");
            for ev in &events[..n] {
                if ev.token == 1 && accepted.is_none() {
                    if let Ok((stream, _)) = listener.accept_nonblocking() {
                        accepted = Some(stream);
                    }
                }
                if ev.token == 2 && (ev.events & EPOLLOUT) != 0 {
                    client_writable = true;
                }
            }
            if accepted.is_some() && client_writable { break; }
        }
        let server = accepted.expect("accepted connection");
        assert!(client_writable);
        assert_eq!(client.write(b"ping").expect("client.write"), 4);
        let mut buf = [0u8; 8];
        let mut read_n = 0usize;
        for _ in 0..20 {
            match server.read(&mut buf) {
                Ok(n) => { read_n = n; break; }
                Err(e) if e.0 == 35 || e.0 == 11 => { let _ = ep.wait(&mut events, 20); }
                Err(e) => panic!("server.read failed: {:?}", e),
            }
        }
        assert_eq!(&buf[..read_n], b"ping");
    }

    #[test]
    fn udp_loopback_sendto_recvfrom() {
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0));
        let rx = UdpSocket::bind(addr).expect("UdpSocket::bind rx");
        let tx = UdpSocket::bind(addr).expect("UdpSocket::bind tx");
        let rx_addr = rx.local_addr().expect("rx.local_addr");
        assert_eq!(tx.send_to(b"hello", rx_addr).expect("tx.send_to"), 5);
        let mut buf = [0u8; 16];
        let (rn, _) = rx.recv_from(&mut buf).expect("rx.recv_from");
        assert_eq!(&buf[..rn], b"hello");
    }
}
