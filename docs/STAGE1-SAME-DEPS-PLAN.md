# Plan: stage1 builds exactly what stage0 builds

Owner rule, 2026-11-12: stage0 and stage1 have exactly the same dependencies. What rustc builds for `ekod` and `ekors`
at stage0 is what `ekod` builds at stage1. Nothing rustc-built is linked into a stage1 output. Nothing here is built yet.

## 1. Where we are (measured 2026-11-12)

- `ekod`'s real build is 198 packages: 32 local, 166 from registries. Stage1 has only ever been given five
  (`ekostd/bootstrap/sources.json`). It builds two of them and is refused on the third.
- ekostd as a whole has never been built by `ekod` from source. The October witnesses (`ekopathrs/scripts/ekostd-selfhost-direct-*.ts`)
  compiled single modules against a rustc-built sysroot holding `libcore`, `liballoc` and `liblibc`. That sysroot is gone and forbidden.
- Compiling a crate root directly, with no source graph, gives "can't find crate for `core`", exit 1, no objects
  (checked with the installed pair on all three bootstrap crates). A report that this mode passes does not reproduce.
- Wave g1 gives `ekod` the same Cargo unit graph rustc builds, and counts what passes. Its numbers replace the estimates in section 3.

## 2. libc-lite, a module inside ekostd (ruled, do first)

ekostd is a wrapper around the system C library. It builds no C library.

- `src/libc_lite/`: declarations only (`extern "C"` functions, `#[repr(C)]` types, constants) for the 173 items ekostd
  uses, split in four files so four lanes can write them at once without touching the same file:
  files (48 items), threads and time (43), processes and signals (49), memory and sockets (33).
- macOS on ARM64 first: it is the stage1 target and the only one a parity test can check on this machine.
  On Linux ekostd keeps the `libc` crate until the same test can run there.
- Each of the 16 source files gets one line, `use crate::libc_lite as libc;`, so no call site changes.
- Proof: a parity test under rustc compares every constant, every struct size, alignment and field offset, and every
  function signature against the `libc` crate. The crate stays only as a test dependency.
- Then rebuild `ekod` and `ekors` and rerun the sweep.

## 3. What else `ekod` cannot compile, and what to do with each kind

From the 166 registry packages in `ekod`'s build. Exact membership per kind comes from wave g1's refusal list.

| Kind | Examples in `ekod`'s build | Why `ekod` cannot build it | Proposal |
|---|---|---|---|
| C declarations | `libc` | third-party, build script | libc-lite in ekostd (section 2) |
| C or assembly compiled by a build script | `blake3` (now pure Rust), `cc` | no C allowed | done for `blake3`; `cc` then drops out |
| Build scripts that only probe the compiler | `serde`, `num-traits`, `indexmap`, `lock_api`, `slab`, `anyhow`, `thiserror`, `semver`, `libm`, `portable-atomic`, `crossbeam-utils`, `getrandom` | `ekod` cannot run a host program | record each script's output once at stage0 (flags only) and feed the same record to stage1; both stages then use identical inputs |
| Build scripts that generate source | `typenum`, `target-lexicon` | same | same record, including the generated files; or drop the crate |
| Proc macros | `serde_derive`, `thiserror-impl`, `derive_more`, `darling`, `rkyv_derive`, `bytecheck_derive`, `tracing-attributes`, `async-trait`, `data_bucket_derive`, `ribbit-derive`, `smart-default`, `derive-where`, `derive_setters` and `syn`, `quote`, `proc-macro2` under them | a proc macro is a host program run inside the compiler | in our own crates, replace convenience derives with written code (`derive_more`, `thiserror`, `smart-default`, `derive_setters`, `async-trait`); for the rest, owner decision in section 6 |
| Small utilities ekostd can own | `cfg-if`, `scopeguard`, `either`, `lazy_static`, `spin`, `fnv`, `foldhash`, `rustc-hash`, `itoa`, `fastrand`, `crc32fast`, `no_std_time`, `log` | nothing stops `ekod` except that each needs a `core`; they are simply outside code | move the few lines we use into ekostd modules and drop the crate, one at a time, biggest fan-in first |
| Large outside crates we really use | `hashbrown`, `indexmap`, `smallvec`, `object`, `serde_json`, `rkyv`, the `unicode-*` set, `rustc_apfloat`, `futures*` | need `core` handed to them; some need the two kinds above | keep; they compile once every unit gets a `core` (ekostd `docs/CORE-AS-A-DEPENDENCY.md`) |

Every third-party unit also needs a `core`. Today `ekod` gives none to a crate that does not declare one. That is one
fix in the Cargo driver, and it unblocks every row above.

## 4. Move general code from ekopathrs to ekostd

So that pathdb, Karen, ps-wild and the compiler depend on one base.

| In ekopathrs today | What it is | Move? |
|---|---|---|
| `crates/clock` | monotonic nanosecond counter, no operating system | yes, to an ekostd module |
| `crates/perf` | timing accumulated in place and reported as a table | yes |
| `crates/door` | counted transport and ordered documents, no application vocabulary | yes |
| `crates/doacross`, `doacross-bridge` | ordering of tasks without blocking workers | yes if nothing in it names the compiler; check first |
| `crates/import`, `origin`, `wire`, `compiler-protocol` | the compiler itself | no |
| `vendor-patches/`, `scripts/build-sysroot.sh`, `sysroot-probe.sh`, `refresh-toolchain.sh`, `no-std-linker.sh` and its two checks | sysroot-era leftovers | delete |

Already in ekostd: bootstrap `core`, `alloc`, `compiler_builtins` (bare folders, no `Cargo.toml`) and the panic runtime.
They become packages as in `docs/CORE-AS-A-DEPENDENCY.md`.

## 5. Order, as one flat wave

All of these touch different files, so they run at once:
1. libc-lite: four lanes, parity test, rebuild.
2. The Cargo driver hands a `core` to every unit.
3. The bootstrap unit list is generated from Cargo, not written by hand; `sources.json` and the name `stage2-diff` go.
4. Record build-script outputs at stage0 and replay them at stage1.
5. Move `clock`, `perf`, `door` into ekostd.
6. Replace convenience derives in our own crates.
7. Delete the sysroot-era leftovers.
The measure after the wave: g1's count of stage0 units that stage1 builds.

## 6. Decisions for the owner

1. Proc macros that stay (`serde_derive`, `rkyv_derive`, `data_bucket_derive` and the like): let `ekod` run them as host
   programs built by the stage0 pair, or expand them once at stage0 and give stage1 the expanded source? The first is the
   real fix and the larger job. The second is quick but stage1 would not be building from the same source text.
2. Linux for libc-lite: wait for a machine where the parity test can run, or declare it now unverified?
3. `doacross`: move it, if it turns out to be general?
