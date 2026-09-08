# ekostd

Janky stuff to help you reduce the deps you need on `std`.

The package is `ekostd`; the crate is `eko`. Those differ because `eko` was taken on crates.io in
2019 by an unrelated project, and `[lib] name` is what you write:

```rust
use eko::file::File;
use eko::thread::Mutex;
```

## What it is

The operating system, wrapped once, over `libc`: paths, files, directories, environment, the
clock, the process, sockets, threads, locks, channels, memory maps, and the print macros.

**A `no_std` crate wrapping `libc` is still `no_std`.** The point is not to avoid the platform, it
is to avoid linking Rust's standard library, which a bootstrapping or self-hosting stage would
otherwise have to carry. It does not have to carry `libc`: the platform already has it. On macOS
this is not even a compromise, because `std` itself goes through libSystem - Apple does not
provide a stable syscall ABI.

## Where it came from

It was written for [EKOPathRS](https://github.com/pathscale/ekopathrs), a pure-Rust compiler with
no LLVM, to get `std` out of a binary that links sixty-odd `rustc_*` crates. It is published
separately because its consumers are not the compiler.

## Features

`nightly` turns on `dropck_eyepatch` for the `#[may_dangle]` on `thread::Mutex`'s destructor.
Exactly one consumer needs it - rustc's `'tcx` origin-point pattern, where a `Mutex` inside a type
otherwise makes that type drop-significant. **Off by default**, so a stable consumer can use this
crate at all. A plain `Drop` impl covers everyone else.

## What it is not

Not a portability layer. It targets Unix, and macOS first. There is no Windows here and no
attempt to hide which platform you are on: a caller that wants `errno` gets `errno`, as a number
it can match, rather than a string that has lost which error it was.
