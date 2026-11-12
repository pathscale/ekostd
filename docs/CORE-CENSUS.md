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

At the start (runs 1 to 9). Leaves: `bitflags`, `bumpalo`, `ekostd`, `foldhash`, `memchr`, `spin`.
Above them: `hashbrown`, `object`, `ps-rusty_alloc`, then `linker-utils`, `libwild`, `wild`.
From run 10 the graph is eight packages; see that section.

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

Run 9, same day: `ps-rusty_alloc` compiles (56 to 0). It is ours and stays in every graph, so what it needs was
added: the address-only pointer operations (`expose_provenance`, `with_addr`, `map_addr`, `without_provenance`,
`with_exposed_provenance`), `mem::offset_of!` with its intrinsic, the `asm!` and `global_asm!` macros,
`AtomicI32`, `AtomicI64`, `fetch_max`, `is_multiple_of`, `next_multiple_of`, `checked_next_multiple_of`,
`checked_shl`, `overflowing_add`, `clamp`, `unsigned_abs`, ASCII case helpers, `Iterator::sum`,
`IntoIterator` for `&[T; N]`, a const `Cell::into_inner`, and `str::parse` (which brought `from_str_radix` back).
Only `object` still fails: 150.

Rule for every later addition: it must be needed by a crate that stays in the graph, and when that crate leaves,
the addition leaves with it.

## Run 10, 2026-11-13: the whole ps-wild graph checks

After ps-wild dropped `object` (and with it `memchr`), the graph is `ekostd`, `foldhash`, `spin`, `hashbrown`,
`ps-rusty_alloc`, `linker-utils`, `libwild`, `wild`. `cargo check -p wild` against `eko-core` with an empty
sysroot now exits 0 for all eight. Run 10 is a check; run 11 below builds and links.

| Step | linker-utils | libwild | wild |
|---|---|---|---|
| start | 85 | not reached | not reached |
| slice searching, `Option::filter`, comparison and default impls; six one-off uses rewritten in the linker | 0 | 2963 | not reached |
| `Send` and `Sync` for `Vec` and `Box` | 0 | 549 | not reached |
| the batch below; about thirty one-off uses rewritten in the linker | 0 | 15 (borrow check) | not reached |
| the drop-check and `size_of_val` corrections below | 0 | 0 | 2 |
| `global_allocator`, `mem::Alignment` | 0 | 0 | 0 |

Added, all needed by `linker-utils` or `libwild`, which are ours and stay in the graph:

- Slices: `windows`, `chunks`, `chunks_exact`, `chunks_exact_mut`, `binary_search_by_key`, `partition_point`,
  `sort_by_key`, `sort_unstable_by_key`, `strip_prefix`, `strip_suffix`, `reverse`, `split_off_mut`,
  `split_off_first_mut`; `Default` for `&[T]` and `&mut [T]`; comparison of a slice with an array;
  `IntoIterator` for `&mut [T; N]`; array `as_slice`; `Default` for arrays.
- `Option`: `filter`, `is_none_or`, `unwrap_or_default`, `map_or_else`, `get_or_insert`, `get_or_insert_with`,
  `replace`, `as_deref`, `as_deref_mut`, `flatten`, `transpose`. `Result`: `unwrap_or_default`, `map_or`,
  `map_or_else`, `is_ok_and`. `bool::then`, `bool::then_some`.
- Iterators: `min`, `max`, `max_by_key`, `min_by_key`, `try_fold`, `try_for_each`, `flat_map`, `flatten`,
  `peekable`, `take_while`; collecting into `Result` and into `Option`.
- Strings: `core::str::pattern::Pattern` for a string, a character and a test on characters; over it
  `starts_with`, `ends_with`, `strip_prefix`, `strip_suffix`, `find`, `contains`, `split`, `split_inclusive`,
  `split_once`, `trim_end_matches`; `chars` (both directions), `lines`; `Debug` and `Display` for `Utf8Error`.
  In `eko-alloc`: `String::from_utf8_lossy`, `str::to_ascii_lowercase`, `join` on slices of strings,
  `String` compared with `str`.
- Numbers: `swap_bytes` on the signed types, `to_be_bytes`, `from_be_bytes`; narrowing `TryFrom` between signed
  types and from unsigned to signed; the operators with a reference on either side; on the non-zero types
  the comparison, hashing and formatting traits, `MIN`, `saturating_add`, `TryFrom` and `From`.
- Trait impls: `PartialEq` and `Eq` for `Result`, `PhantomData`, `Range`, `Infallible` and three-element tuples;
  `Debug` for two- and three-element tuples, `Range`, `Infallible`, `!` and the atomics; `Hash` for pairs;
  `Clone` for `Range`; `Range::contains`, `RangeInclusive::contains`; `Default` and `into_inner` on the atomics.
- Macros: the `test` and `global_allocator` attributes, the `#[default]` helper of `derive(Default)`.
- `eko-alloc`: `Borrow` and `From<&str>` for `Box`; `Hash` and `Borrow` for `Cow`; `VecDeque` from a `Vec` or
  an array; `Extend<&T>` and `AsMut<Vec<T>>` for `Vec`.

Corrections to what was already there, found by this run:

- `Vec` and `Box` were not `Send` or `Sync`.
- The `Drop` impls of `Vec`, its `IntoIter` and `Box` lacked `#[may_dangle]`, so a container of borrows kept
  the borrow alive until the container was dropped. `Vec` now carries `PhantomData<T>` to say it owns its
  elements.
- `mem::size_of_val` and `mem::align_of_val` were the raw intrinsics (`unsafe`, taking a pointer). They take a
  reference and are safe; the intrinsics are in `intrinsics`.
- `Option::unwrap` and `expect` carried a const-trait bound that made them unusable in a constant from a crate
  without that feature.
- `str::split` is back, for any pattern.

Not added; the linker was changed to do without: `pow`, `is_negative`, `first_chunk`,
`CStr::from_bytes_until_nul`, `CString`, `core::num::NonZero`, `iter::repeat_n`, `fmt::from_fn`,
`DebugTuple::finish_non_exhaustive`, `match_indices`, `rsplit`, `splitn`, array patterns, `trim_ascii`,
`Vec::drain`, `BTreeMap::into_values`, `IntoIterator` for `Result`, `Fn` for `Box<dyn Fn>`.

Cost: `eko-core` and `eko-alloc` grew by about 1,450 lines to about 9,500.

Open, found while doing this:

- `sort_by` is an insertion sort and `sort_unstable_by` calls it: quadratic. `VecDeque` is a `Vec`, so
  `pop_front` is linear. Both are correct and both are too slow for a linker's symbol tables; they need real
  implementations before a linker built on `eko-core` is measured.
- `core::arch::aarch64` still carries what only `memchr` called; prune to what `hashbrown` uses.
- `mem::Alignment` and `Layout::from_size_alignment_unchecked` exist because this rustc's
  `global_allocator` expansion names them. Another compiler's expansion may not.

## Run 11, 2026-11-13: it builds, links and runs

`cargo build -p wild` with the same flags exits 0: rustc generated code for all eight packages on `eko-core`,
`eko-compiler-builtins` and `eko-alloc`, and the system linker produced a `wild` executable. One change was
needed: `eko-alloc` declares the allocator entry points with `rustc_std_internal_symbol`, the mangling under
which the `global_allocator` expansion defines them.

That `wild` then linked `libwild/tests/fixtures/macho/rust-trivial` (`cc --ld-path=<wild>`), and the result ran
and returned 42 as the fixture says. Planted defect: the same link through `/usr/bin/false` fails, so the
`--ld-path` linker is the one that ran. This is one fixture on one host, debug profile; the linker's test
suite has not been run on an `eko-core` build.

## Run 12, 2026-11-13: ekod on `eko-core`

ekod admits, analyses and verifies `eko-core` and reaches its backend; it publishes no object yet. The blockers
are compiler work and are listed in `ekopathrs/docs/TODO.md`. Two things left `eko-core` because ekod refuses
them and nothing in the graph uses them: `Rem` and `RemAssign` for `f32` and `f64`, and shifts by a 128-bit
amount. The census still exits 0 without them. `Default` for the atomics takes its zero as a macro argument.

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
