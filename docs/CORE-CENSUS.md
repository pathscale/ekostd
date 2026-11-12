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

Leaves: `bitflags`, `bumpalo`, `ekostd`, `foldhash`, `memchr`, `spin`.
Above them: `hashbrown`, `object`, `ps-rusty_alloc`, then `linker-utils`, `libwild`, `wild`.

## Count

| Run | foldhash | spin | bitflags | bumpalo | memchr | ekostd | total |
|---|---|---|---|---|---|---|---|
| 1, 2026-11-12, before any change | 7 | 11 | 14 | 24 | 45 | 48 | 149 |
| 2, after batch 1 | 6 | 5 | 14 | 23 | 30 | 34 | 112 |
| 3, after batch 2 | 2 | 4 | 14 | 8 | 27 | 25 | 80 |
| 4, after batch 3 | passes | 1 | 2 | 6 | 25 | 22 | 56, plus `hashbrown` reached: 37 |
| 5, after batch 4 | passes | passes | passes | 3 | 22 | 20 | 45, plus `hashbrown`: 5 |
| 6, after batch 5 | passes | passes | passes | passes | 22 | passes | 22, plus `hashbrown`: 1 and `ps-rusty_alloc` reached: 56 |
| 7, after batch 6 | passes | passes | passes | passes | passes | passes | leaves done; `hashbrown` passes; `ps-rusty_alloc` 56; `object` reached: 150 |

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

Batch 4 added: `unwrap_unchecked` on `Option` and `Result`; `core::array` with array `map`; `PartialEq` and `Eq`
for `Layout`; `swap_bytes`, `div_ceil`; `leading_zeros` and `trailing_zeros` on the non-zero types;
`Formatter::pad` (no width or fill); `BuildHasher::hash_one`; `From<bool>` for the integers; signed-to-unsigned
`TryFrom`; `Iterator::step_by`; `ptr::swap_nonoverlapping`; pointer `align_offset`; `atomic::spin_loop_hint`;
`str::split` on a single character; and `handle_alloc_error` in `eko-alloc`.

After run 5 `foldhash`, `spin` and `bitflags` compile.

Batch 5 added: `Box` in `eko-alloc` as the `owned_box` language item, so `*boxed` moves out and the compiler drops
the contents; `RefCell` with `Ref` and `RefMut`; `AsRef` and `AsMut` through references (the old impls were
reflexive, which is not what Rust has); array `IntoIterator` by value and `array::from_fn`; `Hash` for `Vec`;
`Cow` conversions per type instead of a blanket; `c_long` as 64 bits on 64-bit targets; `const_format_args!`
in the panic macros, a const `Arguments::new`, and a bare `assert!` that is one literal, so assertions work in
constants and `const fn`.

After run 6 five of the six leaves compile, `ekostd` among them. What remains on `memchr` and `hashbrown` is
`core::arch` (NEON) only.

Batch 6 added `core::arch::aarch64`: the six NEON vector types and twenty operations that `memchr` and
`hashbrown` call, written lane by lane over plain arrays with no vector language items, so any compiler that can
compile a loop can compile them.

**Milestone 1 reached at run 7: all six leaf crates compile against `eko-core` with an empty sysroot.**
`hashbrown` compiles too. Open: `ps-rusty_alloc` (56) and `object` (150); `linker-utils`, `libwild` and `wild`
are not reached yet.

## Direction change, 2026-11-12: keep `core` minimal, shrink the graph instead

Owner direction: do not grow `core` to cover third-party crates; best effort to limit and prune.

- ps-wild dropped `bitflags` and `bumpalo` (in-house, ps-wild C12). What only they needed was then removed from
  `eko-core`: `from_str_radix` with `ParseIntError`, `str::split`, `abs_diff`, `Result::unwrap_err` and
  `alloc::LayoutErr`. Run 8 confirms `ekostd`, `foldhash`, `spin`, `memchr` and `hashbrown` still compile.
- `object` (150 open) is not to be chased in `core`. It leaves ps-wild's default graph with ELF behind a feature,
  and `memchr` leaves with it; then `core::arch` shrinks to what `hashbrown` alone calls.
- `ps-rusty_alloc` (56 open) is ours: change it to use less first, add to `core` only what is unavoidable.

Rule for every later addition: it must be needed by a crate that stays in the graph, and when that crate leaves,
the addition leaves with it.

## Open after run 2, largest first

(Historical: everything in this list is closed as of run 7.)

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
