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

## Run 13, 2026-11-16: ekod on an empty sysroot, `eko-core` and `eko-alloc` only

The survey builds `eko-core`, `eko-alloc` and `eko-compiler-builtins` with `--sysroot <empty> -Cpanic=abort`, then
`cargo check --keep-going --bin ekod` with the three as the only `core`, `alloc` and `compiler_builtins`.
Every item below was added on its own and only as far as a user needed it. Nothing is a stub: no dummy bodies, no
`unimplemented!`. Rounds are taken from `summary.txt` of each run.

| round | units started | units failed | errors | commit |
|---|---|---|---|---|
| R10 | 40 | 4 | 178 | 4ddd770 |
| R11 (Pin, Zip, BTreeSet, str, Vec::splice, Option/Result/Cell/Ordering) | 40 | 3 | 25 | |
| R12 (Unpin, array Clone, Vec/Cow equality, `ascii`, Pattern for `&&str`) | 45 | 6 | 88 | |
| R13 (CString, Box Fn/FnMut, String replace_range, NonZero const, floats, Step) | 48 | 4 | 68 | |
| R14 (BTreeSet range/union, Extend<&T>, From<T> for Option) | 48 | 2 | 56 | f3f79b9 |

Source per item. **written** means written here against the existing eko-core primitives; **copied** means taken
from `ps-rust-stdlib`; **relax** means taken from `relax/libs/relax/compiler/std/core` (Rust syntax). Where the
Rust version sits on a chain, the item was rewritten smaller and is marked *chain-forced*. Everything below is
written; nothing was copied or taken from relax in these rounds, because each item stands on existing primitives
and is shorter written than imported.

- `Pin<&mut T>::get_mut/get_unchecked_mut/into_ref` by value (the old ones took `&mut self`); `Unpin` for raw
  pointers, `PhantomData`, `Box`, `Rc`, `Arc`, `Weak`; `Clone for [T; N]` for `T: Clone` (it was `Copy` only).
- `Zip` `size_hint`, `ExactSizeIterator`, `DoubleEndedIterator`; `Take` `DoubleEndedIterator`;
  `Iterator::{ne, reduce, partition, max_by}`; `Step` for `u128`/`i128`; `Range: Default`.
- `Option::{as_slice, inspect}`, `From<T> for Option<T>`, `Result::{inspect, inspect_err, expect_err}`,
  `Cell: Default`, `Ordering::{reverse, is_eq, is_ne, is_lt, is_gt, is_le, is_ge}`, `f32/f64::total_cmp` (the Rust
  algorithm), `From<bool>` for `f32`/`f64`.
- *chain-forced*: `NonZero::new` is a plain `const fn` that reads the bytes. Rust relies on a niche attribute, and a
  `[const]` trait bound would force every caller to enable `const_trait_impl`.
- `str::{matches, rsplit_once, rsplit, splitn, escape_default}`, `Pattern for &&str`, `core::ascii::escape_default`.
- *eager*: `Vec::splice` and `BTreeSet::{difference, union, intersection}` build their result at once and return a
  `vec::IntoIter`. Rust defers the splice to the drop of the iterator; the difference shows only in when the vector
  changes.
- `Vec::dedup_by`; `Vec` equality with `Vec<U>`, `[U]`, `&[U]`, `&mut [U]`, `[U; N]`, `&[U; N]`; `Cow<[T]>` equality
  with `Vec<U>` and `&[U]`; `[T]: PartialEq<Vec<U>>`; `Rc::try_unwrap`; `Arc::into_inner` (the Rust algorithm,
  `fetch_sub` first); `String::{try_reserve, insert_str, replace_range}`; `From<String>` and `From<&str>` for
  `Vec<u8>`; `alloc::str::replace`; `VecDeque::try_reserve`.
- `BTreeSet` without the `T: Ord` struct bound (non-ordering methods now in `impl<T>`), `is_subset`, `range`,
  `From<[T; N]>`, `Hash`, `Extend<&T>`; `BTreeMap` `Hash` and `From<[(K, V); N]>`.
- `Box<F>: Fn/FnMut` (it had `FnOnce` only).
- *chain-forced*: `alloc::ffi::{CString, NulError}` hold a `Box<[u8]>` with the NUL, over the existing
  `core::ffi::CStr`: `new`, `from_vec_unchecked`, `as_bytes`, `as_bytes_with_nul`, `as_c_str`, `into_bytes`, `Deref`,
  `Debug`.
- Rounds R1 to R9: `fmt::Options` and template options; `Debug` for `UnsafeCell`, `MaybeUninit`, `slice::Iter`,
  `Enumerate`, `RefCell`, `Ref`, `Fuse`, tuples 4 to 8; generic `NonZero<T>`; `Saturating`; `Wrapping` `/` and `%`; the
  lang items for async lowering (`ResumeTy`, `get_context`, `IntoFuture`, `AsyncDrop`, `Poll`, `Context`);
  `task::Wake`; the `Vec`, `Box`, `Arc`, `String`, `str`, `slice`, `BinaryHeap`, `BTreeMap::range` groups; the NEON
  intrinsics `memchr` uses; `include!`.

### Not added

- `char::escape_debug` (relax-lex `tokens.rs:806`). It needs the Unicode grapheme-extend and printable tables, which is
  a chain, so it stays open. The caller can use `escape_default` (different for non-ASCII), or the tables can be
  generated into eko-core as a separate decision.
- `core::net`. `serde_core` is its only user, and only ekostd's optional `serde` feature pulls serde in.
- Dropped by the owner: `sort_by_cached_key`, `is_sorted_by`, bounds-pair `SliceIndex`, `shrink_to`, `align_to`,
  `Formatter::precision`, `IntoIter` Clone/Debug, `Drain::as_slice`, the crc32 intrinsics.
- Deferred, dependency may be removed: indexmap, crc32fast, generic-array, hybrid-array, typenum, tinyvec.
- Caller-side, not library: `envyrs/src/os.rs` submodules use `libc::` while only the parent has
  `use eko::libc_lite as libc` (55 errors on macOS and Linux aarch64, where the `libc` crate is not a dependency).

### ekostd with `--workspace --all-features`

The only error is `serde_core` on `core::net` (E0432). `serde` is not in ekod's `Cargo.lock`. Recommended: drop
ekostd's `serde` feature, and have `frontend-private` and `relax-facts` (feature `facts-json`) serialise a path
through its string form. That is a caller change and was not made here.

## Run 14, 2026-11-16: serde removed, relax-base, relax-front and relax-types

Numbers are from `summary.txt` of the runs made in this round. Survey (A), `check --workspace --all-features` in ekostd:
exit 0, 1 unit started, 0 failed, after the `serde` feature and dependency were removed.

| round | units started | units failed | errors | what changed |
|---|---|---|---|---|
| S1 | 50 | 2 | 4 | start (relax-base 3, machine 1; escape_debug and envyrs fixed before) |
| S2 | 50 | 1 | 36 | GlobalAlloc re-export, str::trim_start_matches: relax-base opens up |
| S3 | 51 | 1 | 58 | relax-base items, escape_debug, NEON: relax-front opens up |
| S4 | 45 | 2 | 24 | relax-front items: relax-types opens up |
| S5 | 49 | 3 | 4 | relax-types items |
| S6 | 51 | 2 | 2 | String::pop/as_bytes, RangeInclusive Hash |
| S7 | 51 | 2 | 2 | no change: re-run, same two errors |

Commits: ekostd 4160d58, 1e37bcd; envyrs 9ce4b0b.

### Routes
- **serde.** Removed ekostd's `serde` feature, the optional dependency and the three impls in `src/path.rs`. Nothing in
  frontend-relax serialises a `Path` or `PathBuf` (every `Serialize` struct was checked for a path field: the path types
  there are only `&Path` arguments), so no replacement serde code was written. The `features = ["serde"]` and
  `ekostd/serde` entries in `frontend-private` and `relax-facts` were deleted; FRONT owns the rest of serde there.
- **`char::escape_debug` (relax-lex `tokens.rs:806`).** The call renders a doc comment as the `#[doc = "..."]` literal
  that a proc macro sees, so it is user-visible: route (a) would have changed token text for non-ASCII. The relax std
  tables are not usable (they were generated for Unicode 18, and `GRAPHEME_EXTEND_PRINTABLE` is empty because the
  extenders were subtracted from `PRINTABLE`, which is right for `char` but wrong for `str`). Taken: **rust**, the
  stdlib's own algorithm (`escape_debug_ext`: control, private use, whitespace, `Grapheme_Extend`,
  `Default_Ignorable`, `Cf`, unassigned) over the stdlib's own tables (Unicode 17), copied into
  `bootstrap/core/src/unicode.rs` (`grapheme_extend`, `cf`, `cn_planes_0_3`, `default_ignorable_code_point`,
  `skip_search`). The compile-time table checks and the `assume` hints were dropped; the data is untouched. Output is
  identical to rustc's for every character. FRONT saw the error clear in S2 and later.
- **envyrs.** `use super::libc;` (same `cfg`) in the five submodules of `src/os.rs`.

### Items added (source: rust = copied from ps-rust-stdlib, written = written here)
- **rust:** the `unicode.rs` tables and their search (`grapheme_extend`, `cf`, `cn_planes_0_3`,
  `default_ignorable_code_point`, `alphabetic`, `n`, `uppercase` with `bitset_search`, `conversions::{to_upper, to_lower}`
  with their L1 tables, as one module); `char::{escape_debug, to_uppercase, to_lowercase, is_alphabetic, is_alphanumeric,
  is_uppercase}` and `str::to_uppercase` over them; `Duration::from_secs_f64` (the `try_from_secs!` algorithm for
  `f64`, same rounding and panic messages).
- **written:** `char::{is_digit, is_ascii_hexdigit, to_ascii_uppercase, to_ascii_lowercase}`; `Cell::as_ptr`,
  `Atomic{U8,U16,U32,U64,Usize,Isize,I32,I64}::as_ptr`; `OnceCell` (`new`, `get`, `set`, `get_or_init` with the reentrancy
  panic); `Clone for RefCell`; `mem::discriminant`/`Discriminant`; `Iterator for RangeFrom`; `Flatten` back iterator;
  `Iterator::map_while`; `IntoIterator for Result` and for `&Box<[T]>`; `Iterator for Box<I>`; `Option::{insert,
  get_or_insert_default}`; `Result::as_deref`; `ControlFlow::{is_break, is_continue}` and its `Try`/`FromResidual`;
  `AtomicBool::get_mut`; `Formatter::debug_tuple_field{4,5}_finish`; `<[T]>::split`; `Concat` and `<[T]>::concat`;
  `<[u8]>::trim_ascii{,_start,_end}`; `RangeInclusive: Hash`; the NEON `vsubq_u8`, `vcleq_u8`, `vbicq_u8`, `vmull_p64`
  (bitwise carry-less multiply) and `p64`/`p128` aliases, and `vextq_u8` is now `vextq_u8::<N>(a, b)` as in
  `core::arch` (its only user is ps-simd-scan); alloc: `GlobalAlloc` re-export in `alloc::alloc`,
  `str::trim_start_matches` (core), `Arc::{increment_strong_count, unwrap_or_clone}`, `FromIterator<T> for Arc<[T]>`,
  `FromIterator<Cow<str>> for String`, `Cow<str>: PartialEq<&str>` and `<str>`, `Vec::dedup_by_key`, `VecDeque::retain`,
  `String::{pop, as_bytes}`.
- Every one of these was called by a unit in the graph; the `rust` ones were also compared with the relax std, which
  was rejected for the Unicode data above.

### Debts, state after S7
- `Vec::splice` and `BTreeSet::{difference, union, intersection}` are still eager. Not changed: no unit added this round
  calls them, and the earlier rounds did not show a caller that can see the difference; a faithful version needs a
  lazy iterator type per method and was not written.
- `core::any::type_name` still returns `"<type>"`. A real one needs the compiler: an intrinsic
  (`core::intrinsics::type_name::<T>()`, a `rustc_intrinsic` that rustc evaluates to the type's path string) wired
  into `type_name`. It is not data we can write in the library.
- `Result::into_iter` returns `iter::Once<T>` and `Option`'s the same, where Rust names `result::IntoIter`; the sequence
  is the same.
- The `escape_debug` for `str` (`str::escape_debug`, which does not escape grapheme extenders after the first character) is
  not added: nothing in the graph calls it.
- frontend-relax tracks `Cargo.lock` and `link-probe/Cargo.lock` in git; not touched, flagged.

### First remaining blocker (S7)
Two errors, both from in-flight unwinding removal by another agent, not library items:
`ekopathrs/crates/door/src/server.rs:186` calls `contained(..)` which no longer exists (E0425), and
`frontend-relax/src/lib.rs:36` imports `relax_eko_bridge::rustc_compat::unwind_janky`, which is not there (E0432).

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

## 2026-11-16, agent FRONT: items for the frontend-relax workspace (check exit 0 after these)

All taken from the Rust standard library semantics; each is what a frontend unit calls.
- `char::is_lowercase`, `is_cased`, `is_case_ignorable` (made `is_numeric` public); tables `unicode::{lowercase, lt, case_ignorable}` copied verbatim from `unicode_data.rs` (`lowercase::lookup` is not `const`, as `bitset_search` here is not).
- `str::to_lowercase` with the final-sigma rule (`map_uppercase_sigma`), `str::rmatch_indices` (returns `Rev<MatchIndices>`).
- `char::eq_ignore_ascii_case`, `char::is_ascii_punctuation`.
- `String::insert`, `String::remove`.
- `[T]::sort_by_cached_key` (alloc), `Windows: DoubleEndedIterator`.
- `DoubleEndedIterator` for `str::{CharIndices, SplitWhitespace, Lines, Split, SplitInclusive}`: `Split` and `SplitInclusive` gained an `end` field.
- `Ord`/`PartialOrd`/`PartialEq`/`Eq` for tuples up to arity 12 (was 8).

## 2026-11-16, agent FRONT: items for the ekod graph (ekod check exit 0 after these)
- `fmt::Debug` for tuples up to arity 12 (was 8).
- `Arc::make_mut` on `Arc<[T]>` as well as `Arc<T>`: bound is now the trait `CloneToArc` (Rust: `CloneToUninit`).
- `sync::Weak::ptr_eq`, `VecDeque::remove`, `CStr::to_string_lossy` (needs `#[rustc_has_incoherent_inherent_impls]` on `CStr`), `str::rsplitn`.
- `String::drain`: the removed characters are taken out when it is called and the iterator owns them (Rust removes on drop; differs only if the `Drain` is leaked).
