# Census: what `eko-core` lacks for ps-wild

Decision 1 in [CORE-AS-A-DEPENDENCY.md](CORE-AS-A-DEPENDENCY.md): grow `bootstrap/core` until ps-wild's graph
compiles. This file is the running count.

## How it is measured

rustc, with an empty sysroot, so it cannot supply its own `core`:

1. `cargo build` of `bootstrap/alloc` (pulls `eko-core` and `eko-compiler-builtins`) with
   `RUSTC_BOOTSTRAP=1 RUSTFLAGS="--sysroot <empty dir> -Cpanic=abort"`, target `aarch64-apple-darwin`.
2. In `~/code/ps-wild`: `cargo check -p wild --keep-going` with the same flags plus `-L <step 1 deps dir>`.

Start each run from clean target directories: a leftover `.rmeta` beside the `.rlib` makes rustc report
"multiple candidates for `core`" and every prelude name then fails, which is noise, not a finding.

rustc stops each crate at its first failing phase, so the count for a crate can rise after a fix exposes the next
phase. Crates above the leaves are not reached until the leaves pass.

## Graph

Leaves (reached): `bitflags`, `bumpalo`, `ekostd`, `foldhash`, `memchr`, `spin`.
Not reached yet: `hashbrown`, `object`, `ps-rusty_alloc`, `linker-utils`, `libwild`, `wild`.

## Count

| Run | foldhash | spin | bitflags | bumpalo | memchr | ekostd | total |
|---|---|---|---|---|---|---|---|
| 1, 2026-11-12, before any change | 7 | 11 | 14 | 24 | 45 | 48 | 149 |
| 2, after batch 1 | 6 | 5 | 14 | 23 | 30 | 34 | 112 |
| 3, after batch 2 | 2 | 4 | 14 | 8 | 27 | 25 | 80 |
| 4, after batch 3 | passes | 1 | 2 | 6 | 25 | 22 | 56, plus `hashbrown` reached: 37 |

Batch 1 added: `mem::zeroed`, `mem::transmute`, `MaybeUninit::{zeroed, as_ptr, as_mut_ptr}`,
`ptr::{read_unaligned, write_unaligned}` and the pointer method, `wrapping_shl`, `wrapping_shr`,
`UnsafeCell::get_mut`, and the legacy integer modules (`core::usize::MAX`).

Batch 2 added: `Formatter::{debug_struct, debug_tuple}` with their builders; `Debug` for `NonNull`, `Cell`,
`PhantomData` and `Layout`; `Pointer` for `NonNull`; `AsRef<[T]>` and `AsMut<[T]>` for arrays; `AsRef<[u8]>` for
`str`; `TryFrom<&[T]>` for arrays.

Batch 3 added: `from_str_radix` with `ParseIntError`; `saturating_mul`, `abs`, `abs_diff`, `rotate_left`,
`rotate_right`; shifts by any integer type; `Option::take`, `Cell::take`, `Result::{unwrap_err, transpose}`;
`MaybeUninit::assume_init_read`; `get_mut` on the integer atomics; pointer `offset_from`; `alloc::LayoutErr`;
the `CStr` lang item on the existing type; and `eko-alloc` now re-exports `Layout`.

After run 4 `foldhash` compiles, which lets rustc reach `hashbrown` for the first time.

## Open after run 2, largest first

(Not yet re-sorted after batch 2; the trait-impl and formatter groups below are mostly closed.)

- Missing trait impls, about 34: `Debug` and `Pointer` for `NonNull`, `Cell`, `PhantomData`, `Layout`;
  `AsRef<[u8]>` for arrays, `str` and `Vec`; `TryFrom<&[u8]>` for arrays; `Hash` for `Vec<u8>`.
- `from_str_radix` on the integer types, 12.
- `core::arch` and the NEON vector types (`uint8x16_t`), 10. `memchr` only; it has a scalar fallback if the
  `aarch64` path is not selected, to be checked before writing any of `core::arch`.
- The `CStr` lang item, 5.
- `Formatter::{debug_struct, debug_tuple}`, 5.
- Single methods: `Option::take` bounds, `str::split`, pointer `offset_from`, `Result::unwrap_err`,
  `Option::transpose`, `saturating_mul`, `rotate_right`, `div_ceil`, `abs`, `abs_diff`,
  `MaybeUninit::assume_init_read`, `alloc::LayoutErr`.
- Three integer type mismatches on shifts (`foldhash`), where our shift impls are narrower than Rust's.
