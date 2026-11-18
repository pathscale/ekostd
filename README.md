# ekostd

Janky stuff to help you reduce the deps you need on `std`.

The package is `ekostd`; the crate is `eko`. Those differ because `eko` was taken on crates.io in
2019 by an unrelated project, and `[lib] name` is what you write:

```rust
use eko::file::File;
use eko::thread::Mutex;
```

## What it is

One OS API for applications - paths, files, directories, polling, environment, the clock, the
process, signals, sockets, threads, locks, channels, math, memory maps, and the print macros -
with two interchangeable backends:

- `libc_lite` for macOS and Linux (`aarch64` and `x86_64`), checked at compile time for exact layout
  and signature parity against the platform `libc` without taking `libc` as a runtime dependency.
- [`libc-rs`](https://crates.io/crates/libc-rs) for `unikernel-rs` (`target_os = "none"`), enabled
  with the `libc-rs` feature, routing threads, futex locks, heap, math, and syscalls through pure
  Rust with zero C code.

Code above `eko` does not change when the backend changes.

**A `no_std` crate over an OS substrate is still `no_std`.** The point is not to avoid the
platform, it is to avoid linking Rust's standard library, which a bootstrapping or self-hosting
stage would otherwise have to carry. On macOS and Linux the platform already provides `libc` (on
macOS `std` itself goes through `libSystem` - Apple does not provide a stable syscall ABI). On
`unikernel-rs` there is no C library at all, and `libc-rs` supplies the substrate directly.

## Backends

| Target | Backend | Feature |
| --- | --- | --- |
| `aarch64-apple-darwin` | `libc_lite` (`libSystem`) | *(default)* |
| `aarch64-unknown-linux-{gnu,musl}` | `libc_lite` (platform `libc`) | *(default)* |
| `x86_64-unknown-linux-{gnu,musl}` | `libc_lite` (platform `libc`) | *(default)* |
| `aarch64-unknown-none` (`unikernel-rs`) | `libc-rs` | `libc-rs` |

## Where it came from

It was written for [EKOPathRS](https://github.com/pathscale/ekopathrs), a pure-Rust compiler with
no LLVM, to get `std` out of a binary that links sixty-odd `rustc_*` crates. It is published
separately because its consumers are not the compiler.

## Features

`libc-rs` switches the backend from `libc_lite` to [`libc-rs`](https://crates.io/crates/libc-rs)
(`^0.2.1`) for `unikernel-rs` (`target_os = "none"`, `aarch64-unknown-none`), replacing C library
declarations, `pthreads`, and `-lm` with pure-Rust syscall, futex, TLS, allocator, and math
implementations. **Off by default.**

`nightly` turns on `dropck_eyepatch` for the `#[may_dangle]` on `thread::Mutex`'s destructor.
Exactly one consumer needs it - rustc's `'tcx` origin-point pattern, where a `Mutex` inside a type
otherwise makes that type drop-significant. **Off by default**, so a stable consumer can use this
crate at all. A plain `Drop` impl covers everyone else.

## What it is not

One API across macOS, Linux, and `unikernel-rs` - no Windows. And it does not hide platform
details: a caller that wants `errno` gets `errno`, as a number it can match, rather than a string
that has lost which error it was.
