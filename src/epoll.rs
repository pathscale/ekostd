//! Cross-platform readiness poller (`epoll` on Linux/`libc-rs`, `kqueue` on macOS) and wakeup descriptor.

#[cfg(not(target_os = "none"))]
use crate::libc_lite as libc;
use crate::Errno;

/// The associated descriptor is available for read operations.
pub const EPOLLIN: u32 = 0x001;
/// The associated descriptor is available for write operations.
pub const EPOLLOUT: u32 = 0x004;
/// Error condition happened on the associated descriptor.
pub const EPOLLERR: u32 = 0x008;
/// Hang up happened on the associated descriptor.
pub const EPOLLHUP: u32 = 0x010;
/// Stream socket peer closed connection, or shut down writing half of connection.
pub const EPOLLRDHUP: u32 = 0x2000;
/// Sets one-shot behavior for the associated descriptor.
pub const EPOLLONESHOT: u32 = 1 << 30;
/// Sets edge-triggered behavior for the associated descriptor.
pub const EPOLLET: u32 = 1 << 31;

/// A readiness notification returned by [`Epoll::wait`].
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EpollEvent {
    /// Readiness event bitmask (`EPOLLIN`, `EPOLLOUT`, `EPOLLERR`, `EPOLLHUP`, `EPOLLRDHUP`).
    pub events: u32,
    /// Caller-supplied opaque token registered with the descriptor.
    pub token: u64,
}

impl EpollEvent {
    /// Constructs a new [`EpollEvent`].
    #[must_use]
    pub const fn new(events: u32, token: u64) -> Self {
        Self { events, token }
    }

    /// Returns `true` if `EPOLLIN` is set.
    #[must_use]
    pub const fn is_readable(&self) -> bool {
        (self.events & EPOLLIN) != 0
    }

    /// Returns `true` if `EPOLLOUT` is set.
    #[must_use]
    pub const fn is_writable(&self) -> bool {
        (self.events & EPOLLOUT) != 0
    }

    /// Returns `true` if `EPOLLERR` is set.
    #[must_use]
    pub const fn is_error(&self) -> bool {
        (self.events & EPOLLERR) != 0
    }

    /// Returns `true` if `EPOLLHUP` or `EPOLLRDHUP` is set.
    #[must_use]
    pub const fn is_hangup(&self) -> bool {
        (self.events & (EPOLLHUP | EPOLLRDHUP)) != 0
    }
}

#[cfg(all(not(target_os = "none"), target_os = "linux"))]
mod linux_ffi {
    use core::ffi::c_int;

    pub const EPOLL_CLOEXEC: c_int = 0x80000;
    pub const EPOLL_CTL_ADD: c_int = 1;
    pub const EPOLL_CTL_DEL: c_int = 2;
    pub const EPOLL_CTL_MOD: c_int = 3;

    pub const EFD_NONBLOCK: c_int = 0x800;
    pub const EFD_CLOEXEC: c_int = 0x80000;

    #[cfg(target_arch = "x86_64")]
    #[repr(C, packed)]
    #[derive(Clone, Copy, Default)]
    pub struct SysEpollEvent {
        pub events: u32,
        pub data: u64,
    }

    #[cfg(not(target_arch = "x86_64"))]
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct SysEpollEvent {
        pub events: u32,
        pub _pad: u32,
        pub data: u64,
    }

    unsafe extern "C" {
        pub fn epoll_create1(flags: c_int) -> c_int;
        pub fn epoll_ctl(epfd: c_int, op: c_int, fd: c_int, event: *mut SysEpollEvent) -> c_int;
        pub fn epoll_wait(
            epfd: c_int,
            events: *mut SysEpollEvent,
            maxevents: c_int,
            timeout: c_int,
        ) -> c_int;
        pub fn eventfd(initval: core::ffi::c_uint, flags: c_int) -> c_int;
    }
}

#[cfg(all(not(target_os = "none"), target_os = "macos"))]
mod macos_ffi {
    use core::ffi::{c_int, c_long, c_void};

    pub const EVFILT_READ: i16 = -1;
    pub const EVFILT_WRITE: i16 = -2;

    pub const EV_ADD: u16 = 0x0001;
    pub const EV_DELETE: u16 = 0x0002;
    pub const EV_ENABLE: u16 = 0x0004;
    pub const EV_ONESHOT: u16 = 0x0010;
    pub const EV_CLEAR: u16 = 0x0020;
    pub const EV_ERROR: u16 = 0x4000;
    pub const EV_EOF: u16 = 0x8000;

    #[repr(C, packed(4))]
    #[derive(Clone, Copy)]
    pub struct Kevent {
        pub ident: usize,
        pub filter: i16,
        pub flags: u16,
        pub fflags: u32,
        pub data: isize,
        pub udata: *mut c_void,
    }

    impl Kevent {
        pub const ZERO: Self = Self {
            ident: 0,
            filter: 0,
            flags: 0,
            fflags: 0,
            data: 0,
            udata: core::ptr::null_mut(),
        };
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct Timespec {
        pub tv_sec: c_long,
        pub tv_nsec: c_long,
    }

    unsafe extern "C" {
        pub fn kqueue() -> c_int;
        pub fn kevent(
            kq: c_int,
            changelist: *const Kevent,
            nchanges: c_int,
            eventlist: *mut Kevent,
            nevents: c_int,
            timeout: *const Timespec,
        ) -> c_int;
    }
}

/// A cross-platform readiness poller.
#[derive(Debug)]
pub struct Epoll {
    fd: i32,
}

impl Epoll {
    /// Creates a new readiness poller with close-on-exec enabled.
    pub fn new() -> Result<Self, Errno> {
        #[cfg(target_os = "none")]
        {
            let fd = ::libc_rs::poll::epoll_create1(::libc_rs::poll::EPOLL_CLOEXEC)
                .map_err(|e| Errno(e.0))?;
            Ok(Self { fd })
        }

        #[cfg(all(not(target_os = "none"), target_os = "linux"))]
        {
            let fd = crate::checked(unsafe { linux_ffi::epoll_create1(linux_ffi::EPOLL_CLOEXEC) })?;
            Ok(Self { fd })
        }

        #[cfg(all(not(target_os = "none"), target_os = "macos"))]
        {
            let fd = crate::checked(unsafe { macos_ffi::kqueue() })?;
            let _ = unsafe { libc::fcntl3(fd, libc::F_SETFD, libc::FD_CLOEXEC) };
            Ok(Self { fd })
        }
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

    /// Registers `fd` with `token` and `events` mask.
    pub fn add(&self, fd: i32, token: u64, events: u32) -> Result<(), Errno> {
        #[cfg(target_os = "none")]
        {
            let ev = ::libc_rs::poll::EpollEvent { events, _pad: 0, data: token };
            ::libc_rs::poll::epoll_ctl(self.fd, ::libc_rs::poll::EPOLL_CTL_ADD, fd, Some(&ev))
                .map_err(|e| Errno(e.0))
        }

        #[cfg(all(not(target_os = "none"), target_os = "linux"))]
        {
            let mut ev = linux_ffi::SysEpollEvent {
                events,
                #[cfg(not(target_arch = "x86_64"))]
                _pad: 0,
                data: token,
            };
            crate::checked(unsafe {
                linux_ffi::epoll_ctl(self.fd, linux_ffi::EPOLL_CTL_ADD, fd, &mut ev)
            })?;
            Ok(())
        }

        #[cfg(all(not(target_os = "none"), target_os = "macos"))]
        {
            self.apply_kqueue(fd, token, events, true)
        }
    }

    /// Modifies the registration of `fd` to use `token` and `events` mask.
    pub fn modify(&self, fd: i32, token: u64, events: u32) -> Result<(), Errno> {
        #[cfg(target_os = "none")]
        {
            let ev = ::libc_rs::poll::EpollEvent { events, _pad: 0, data: token };
            ::libc_rs::poll::epoll_ctl(self.fd, ::libc_rs::poll::EPOLL_CTL_MOD, fd, Some(&ev))
                .map_err(|e| Errno(e.0))
        }

        #[cfg(all(not(target_os = "none"), target_os = "linux"))]
        {
            let mut ev = linux_ffi::SysEpollEvent {
                events,
                #[cfg(not(target_arch = "x86_64"))]
                _pad: 0,
                data: token,
            };
            crate::checked(unsafe {
                linux_ffi::epoll_ctl(self.fd, linux_ffi::EPOLL_CTL_MOD, fd, &mut ev)
            })?;
            Ok(())
        }

        #[cfg(all(not(target_os = "none"), target_os = "macos"))]
        {
            self.apply_kqueue(fd, token, events, false)
        }
    }

    /// Removes `fd` from the poller's interest set.
    pub fn delete(&self, fd: i32) -> Result<(), Errno> {
        #[cfg(target_os = "none")]
        {
            ::libc_rs::poll::epoll_ctl(self.fd, ::libc_rs::poll::EPOLL_CTL_DEL, fd, None)
                .map_err(|e| Errno(e.0))
        }

        #[cfg(all(not(target_os = "none"), target_os = "linux"))]
        {
            crate::checked(unsafe {
                linux_ffi::epoll_ctl(self.fd, linux_ffi::EPOLL_CTL_DEL, fd, core::ptr::null_mut())
            })?;
            Ok(())
        }

        #[cfg(all(not(target_os = "none"), target_os = "macos"))]
        {
            self.delete_kqueue(fd)
        }
    }

    /// Alias for [`Self::delete`].
    pub fn remove(&self, fd: i32) -> Result<(), Errno> {
        self.delete(fd)
    }

    #[cfg(all(not(target_os = "none"), target_os = "macos"))]
    fn apply_kqueue(&self, fd: i32, token: u64, events: u32, is_add: bool) -> Result<(), Errno> {
        if fd < 0 {
            return Err(Errno(libc::EBADF));
        }
        let mut base_flags = macos_ffi::EV_ADD | macos_ffi::EV_ENABLE;
        if (events & EPOLLET) != 0 {
            base_flags |= macos_ffi::EV_CLEAR;
        }
        if (events & EPOLLONESHOT) != 0 {
            base_flags |= macos_ffi::EV_ONESHOT;
        }

        let want_read = (events & EPOLLIN) != 0;
        let want_write = (events & EPOLLOUT) != 0;

        if !want_read && !want_write {
            let probe = macos_ffi::Kevent {
                ident: fd as usize,
                filter: macos_ffi::EVFILT_READ,
                flags: base_flags,
                fflags: 0,
                data: 0,
                udata: token as usize as *mut core::ffi::c_void,
            };
            self.kevent_change(&probe)?;
            let _ = self.delete_kqueue(fd);
            return Ok(());
        }

        let changes = [
            macos_ffi::Kevent {
                ident: fd as usize,
                filter: macos_ffi::EVFILT_READ,
                flags: if want_read { base_flags } else { macos_ffi::EV_DELETE },
                fflags: 0,
                data: 0,
                udata: token as usize as *mut core::ffi::c_void,
            },
            macos_ffi::Kevent {
                ident: fd as usize,
                filter: macos_ffi::EVFILT_WRITE,
                flags: if want_write { base_flags } else { macos_ffi::EV_DELETE },
                fflags: 0,
                data: 0,
                udata: token as usize as *mut core::ffi::c_void,
            },
        ];

        for change in &changes {
            if is_add && (change.flags & macos_ffi::EV_DELETE) != 0 {
                continue;
            }
            if let Err(err) = self.kevent_change(change) {
                if (change.flags & macos_ffi::EV_DELETE) != 0 && err.0 == libc::ENOENT {
                    continue;
                }
                return Err(err);
            }
        }
        Ok(())
    }

    #[cfg(all(not(target_os = "none"), target_os = "macos"))]
    fn delete_kqueue(&self, fd: i32) -> Result<(), Errno> {
        if fd < 0 {
            return Err(Errno(libc::EBADF));
        }
        let changes = [
            macos_ffi::Kevent {
                ident: fd as usize,
                filter: macos_ffi::EVFILT_READ,
                flags: macos_ffi::EV_DELETE,
                fflags: 0,
                data: 0,
                udata: core::ptr::null_mut(),
            },
            macos_ffi::Kevent {
                ident: fd as usize,
                filter: macos_ffi::EVFILT_WRITE,
                flags: macos_ffi::EV_DELETE,
                fflags: 0,
                data: 0,
                udata: core::ptr::null_mut(),
            },
        ];
        let mut any_ok = false;
        let mut last_err = Errno(libc::ENOENT);
        for change in &changes {
            match self.kevent_change(change) {
                Ok(()) => any_ok = true,
                Err(e) => {
                    if e.0 != libc::ENOENT {
                        return Err(e);
                    }
                    last_err = e;
                }
            }
        }
        if any_ok || last_err.0 == libc::ENOENT {
            Ok(())
        } else {
            Err(last_err)
        }
    }

    #[cfg(all(not(target_os = "none"), target_os = "macos"))]
    fn kevent_change(&self, change: &macos_ffi::Kevent) -> Result<(), Errno> {
        let rc = unsafe {
            macos_ffi::kevent(
                self.fd,
                core::ptr::from_ref(change),
                1,
                core::ptr::null_mut(),
                0,
                core::ptr::null(),
            )
        };
        crate::checked(rc)?;
        Ok(())
    }

    /// Waits for events into `events` for up to `timeout_ms` milliseconds (`< 0` waits indefinitely).
    pub fn wait(&self, events: &mut [EpollEvent], timeout_ms: i32) -> Result<usize, Errno> {
        if events.is_empty() {
            return Ok(0);
        }

        #[cfg(target_os = "none")]
        {
            let mut raw_events = [::libc_rs::poll::EpollEvent { events: 0, _pad: 0, data: 0 }; 64];
            let max_len = events.len().min(raw_events.len());
            let mut ts = ::libc_rs::time::Timespec::default();
            let timeout_opt = if timeout_ms < 0 {
                None
            } else {
                let ms = i64::from(timeout_ms);
                ts.tv_sec = ms / 1000;
                ts.tv_nsec = (ms % 1000) * 1_000_000;
                Some(&ts)
            };
            let n = ::libc_rs::poll::epoll_wait(self.fd, &mut raw_events[..max_len], timeout_opt)
                .map_err(|e| Errno(e.0))?;
            let mut i = 0usize;
            while i < n {
                events[i] = EpollEvent { events: raw_events[i].events, token: raw_events[i].data };
                i += 1;
            }
            Ok(n)
        }

        #[cfg(all(not(target_os = "none"), target_os = "linux"))]
        {
            let mut raw_events = [linux_ffi::SysEpollEvent {
                events: 0,
                #[cfg(not(target_arch = "x86_64"))]
                _pad: 0,
                data: 0,
            }; 64];
            let max_len = events.len().min(raw_events.len());
            let n = crate::checked(unsafe {
                linux_ffi::epoll_wait(
                    self.fd,
                    raw_events.as_mut_ptr(),
                    max_len as core::ffi::c_int,
                    timeout_ms,
                )
            })? as usize;
            let mut i = 0usize;
            while i < n {
                let ev_mask = raw_events[i].events;
                let ev_tok = raw_events[i].data;
                events[i] = EpollEvent { events: ev_mask, token: ev_tok };
                i += 1;
            }
            Ok(n)
        }

        #[cfg(all(not(target_os = "none"), target_os = "macos"))]
        {
            let mut raw_events: [macos_ffi::Kevent; 64] = [macos_ffi::Kevent::ZERO; 64];
            let max_len = events.len().min(raw_events.len());
            let ts = macos_ffi::Timespec {
                tv_sec: (i64::from(timeout_ms.max(0)) / 1000) as core::ffi::c_long,
                tv_nsec: ((i64::from(timeout_ms.max(0)) % 1000) * 1_000_000) as core::ffi::c_long,
            };
            let ts_ptr = if timeout_ms < 0 { core::ptr::null() } else { core::ptr::from_ref(&ts) };
            let n = crate::checked(unsafe {
                macos_ffi::kevent(
                    self.fd,
                    core::ptr::null(),
                    0,
                    raw_events.as_mut_ptr(),
                    max_len as core::ffi::c_int,
                    ts_ptr,
                )
            })? as usize;

            let mut out_len = 0usize;
            let mut i = 0usize;
            while i < n {
                let kev = &raw_events[i];
                let token = kev.udata as usize as u64;
                let mut mask = 0u32;
                if kev.filter == macos_ffi::EVFILT_READ {
                    mask |= EPOLLIN;
                    if (kev.flags & macos_ffi::EV_EOF) != 0 {
                        mask |= EPOLLRDHUP | EPOLLHUP;
                    }
                } else if kev.filter == macos_ffi::EVFILT_WRITE {
                    mask |= EPOLLOUT;
                    if (kev.flags & macos_ffi::EV_EOF) != 0 {
                        mask |= EPOLLHUP;
                    }
                }
                if (kev.flags & macos_ffi::EV_ERROR) != 0 {
                    mask |= EPOLLERR;
                }

                // Coalesce if an earlier event in this batch has the same token.
                let mut merged = false;
                let mut j = 0usize;
                while j < out_len {
                    if events[j].token == token {
                        events[j].events |= mask;
                        merged = true;
                        break;
                    }
                    j += 1;
                }
                if !merged && out_len < events.len() {
                    events[out_len] = EpollEvent { events: mask, token };
                    out_len += 1;
                }
                i += 1;
            }
            Ok(out_len)
        }
    }
}

impl Drop for Epoll {
    fn drop(&mut self) {
        #[cfg(target_os = "none")]
        let _ = ::libc_rs::io::close(self.fd);
        #[cfg(not(target_os = "none"))]
        unsafe {
            libc::close(self.fd);
        }
    }
}

/// A nonblocking descriptor for waking a thread blocked in [`Epoll::wait`].
///
/// Backed by `eventfd` on Linux and `libc-rs`, and by a nonblocking `pipe` on macOS.
#[derive(Debug)]
pub struct WakerFd {
    read_fd: i32,
    #[cfg(all(not(target_os = "none"), target_os = "macos"))]
    write_fd: i32,
}

impl WakerFd {
    /// Creates a new nonblocking close-on-exec wakeup descriptor.
    pub fn new() -> Result<Self, Errno> {
        #[cfg(target_os = "none")]
        {
            let fd = ::libc_rs::poll::eventfd(
                0,
                ::libc_rs::poll::EFD_CLOEXEC | ::libc_rs::poll::EFD_NONBLOCK,
            )
            .map_err(|e| Errno(e.0))?;
            Ok(Self { read_fd: fd })
        }

        #[cfg(all(not(target_os = "none"), target_os = "linux"))]
        {
            let fd = crate::checked(unsafe {
                linux_ffi::eventfd(0, linux_ffi::EFD_CLOEXEC | linux_ffi::EFD_NONBLOCK)
            })?;
            Ok(Self { read_fd: fd })
        }

        #[cfg(all(not(target_os = "none"), target_os = "macos"))]
        {
            let mut fds = [0i32; 2];
            crate::checked(unsafe { libc::pipe(fds.as_mut_ptr()) })?;
            let read_fd = fds[0];
            let write_fd = fds[1];
            let setup = (|| -> Result<(), Errno> {
                crate::checked(unsafe { libc::fcntl3(read_fd, libc::F_SETFD, libc::FD_CLOEXEC) })?;
                crate::checked(unsafe { libc::fcntl3(write_fd, libc::F_SETFD, libc::FD_CLOEXEC) })?;
                let rfl = crate::checked(unsafe { libc::fcntl2(read_fd, libc::F_GETFL) })?;
                crate::checked(unsafe {
                    libc::fcntl3(read_fd, libc::F_SETFL, rfl | libc::O_NONBLOCK)
                })?;
                let wfl = crate::checked(unsafe { libc::fcntl2(write_fd, libc::F_GETFL) })?;
                crate::checked(unsafe {
                    libc::fcntl3(write_fd, libc::F_SETFL, wfl | libc::O_NONBLOCK)
                })?;
                Ok(())
            })();
            if let Err(e) = setup {
                unsafe {
                    libc::close(read_fd);
                    libc::close(write_fd);
                }
                return Err(e);
            }
            Ok(Self { read_fd, write_fd })
        }
    }

    /// Returns the raw descriptor to register with [`Epoll`] for `EPOLLIN` readiness.
    #[must_use]
    pub const fn as_raw_fd(&self) -> i32 {
        self.read_fd
    }

    /// Returns the raw descriptor to register with [`Epoll`] for `EPOLLIN` readiness.
    #[must_use]
    pub const fn raw(&self) -> i32 {
        self.read_fd
    }

    /// Signals the wakeup descriptor so any thread blocked in [`Epoll::wait`] wakes up.
    pub fn wake(&self) -> Result<(), Errno> {
        #[cfg(target_os = "none")]
        {
            let val: u64 = 1;
            match ::libc_rs::io::write(self.read_fd, &val.to_ne_bytes()) {
                Ok(_) => Ok(()),
                Err(e) if e.would_block() => Ok(()),
                Err(e) => Err(Errno(e.0)),
            }
        }

        #[cfg(all(not(target_os = "none"), target_os = "linux"))]
        {
            let val: u64 = 1;
            let bytes = val.to_ne_bytes();
            let n = unsafe { libc::write(self.read_fd, bytes.as_ptr().cast(), bytes.len()) };
            if n < 0 {
                let err = Errno::current();
                if err.would_block() {
                    return Ok(());
                }
                return Err(err);
            }
            Ok(())
        }

        #[cfg(all(not(target_os = "none"), target_os = "macos"))]
        {
            let byte = [1u8];
            let n = unsafe { libc::write(self.write_fd, byte.as_ptr().cast(), 1) };
            if n < 0 {
                let err = Errno::current();
                if err.would_block() || err.interrupted() {
                    return Ok(());
                }
                return Err(err);
            }
            Ok(())
        }
    }

    /// Alias for [`Self::wake`].
    pub fn notify(&self) -> Result<(), Errno> {
        self.wake()
    }

    /// Drains pending wakeup notifications from the descriptor until `EAGAIN`/`EWOULDBLOCK`.
    pub fn drain(&self) -> Result<(), Errno> {
        #[cfg(target_os = "none")]
        {
            let mut buf = [0u8; 8];
            loop {
                match ::libc_rs::io::read(self.read_fd, &mut buf) {
                    Ok(0) => return Ok(()),
                    Ok(_) => continue,
                    Err(e) if e.interrupted() => continue,
                    Err(e) if e.would_block() => return Ok(()),
                    Err(e) => return Err(Errno(e.0)),
                }
            }
        }

        #[cfg(not(target_os = "none"))]
        {
            #[cfg(target_os = "linux")]
            let mut buf = [0u8; 8];
            #[cfg(target_os = "macos")]
            let mut buf = [0u8; 64];
            loop {
                let n = unsafe { libc::read(self.read_fd, buf.as_mut_ptr().cast(), buf.len()) };
                if n > 0 {
                    continue;
                }
                if n == 0 {
                    return Ok(());
                }
                let err = Errno::current();
                if err.interrupted() {
                    continue;
                }
                if err.would_block() {
                    return Ok(());
                }
                return Err(err);
            }
        }
    }
}

impl Drop for WakerFd {
    fn drop(&mut self) {
        #[cfg(target_os = "none")]
        let _ = ::libc_rs::io::close(self.read_fd);

        #[cfg(all(not(target_os = "none"), target_os = "linux"))]
        unsafe {
            libc::close(self.read_fd);
        }

        #[cfg(all(not(target_os = "none"), target_os = "macos"))]
        unsafe {
            libc::close(self.read_fd);
            libc::close(self.write_fd);
        }
    }
}

/// Alias for [`WakerFd`].
pub type EventFd = WakerFd;

/// Alias for [`WakerFd`].
pub type WakePipe = WakerFd;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoll_and_waker_fd_roundtrip() {
        let ep = Epoll::new().expect("Epoll::new");
        let waker = WakerFd::new().expect("WakerFd::new");
        ep.add(waker.as_raw_fd(), 42, EPOLLIN).expect("ep.add");
        let mut events = [EpollEvent { events: 0, token: 0 }; 4];
        assert_eq!(ep.wait(&mut events, 0).expect("ep.wait(0)"), 0);
        waker.wake().expect("waker.wake");
        let n = ep.wait(&mut events, 100).expect("ep.wait(100)");
        assert_eq!(n, 1);
        assert_eq!(events[0].token, 42);
        assert_ne!(events[0].events & EPOLLIN, 0);
        waker.drain().expect("waker.drain");
        assert_eq!(ep.wait(&mut events, 0).expect("ep.wait(0) after drain"), 0);
        ep.delete(waker.as_raw_fd()).expect("ep.delete");
    }
}
