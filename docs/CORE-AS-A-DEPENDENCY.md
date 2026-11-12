# Proposal: `core` is an ordinary dependency, for rustc and for EKO

Status: proposal, 2026-11-12. Nothing here is built yet. Owner decisions needed are in section 6.

## 1. What is true today

- `bootstrap/core`, `bootstrap/alloc` and `bootstrap/compiler_builtins` are bare `src/` directories in this repository.
  They have no `Cargo.toml` and are published nowhere. Only `bootstrap/build.ts` knows them, through `bootstrap/sources.json`.
- The in-house binaries do not use them. `ekod`, `ekors`, `pathdb`, `karen` and `ps-wild` are built by cargo with stable
  rustc, and rustc silently supplies its own prebuilt `core` from its sysroot. Nothing in any manifest says so.
- The bootstrap `core` is about 5,800 lines, a minimal subset. It is enough for the five bootstrap units
  (`core`, `alloc`, `compiler_builtins`, `frontend_panic_runtime`, `eko_debug`). It is not enough for a third-party crate.
- EKO has the same gap from the other side. Asked to build Karen (wave c, 2026-11-12), the stage0 pair refused every
  third-party unit with "source graph has no admitted direct dependency alias `core`": nothing gives `adler2` or `memchr`
  a `core`, because they do not declare one. Crates that need `std` are refused with "std (`ps-rust-stdlib`) is disabled in stage0".

## 2. The rule

A program's `core` is a versioned package named in the build, resolved by cargo like any dependency, and built from source.
No compiler supplies one by default. Nobody asks where it is.

## 3. Packages

Publish three packages from this repository, on the same registry the family already uses for `ekostd`:

| Package | Library name | Source |
|---|---|---|
| `eko-core` | `core` | `bootstrap/core` |
| `eko-alloc` | `alloc` | `bootstrap/alloc` |
| `eko-compiler-builtins` | `compiler_builtins` | `bootstrap/compiler_builtins` |

The package names differ from the library names because a registry will not take `core`, `alloc` or `std` as package
names (to be confirmed at first publish). Each gets a `Cargo.toml`; `sources.json` is then generated from cargo
metadata instead of being written by hand.

## 4. How a build gets it

The hard fact: a crate that does not declare `core` still uses it. rustc and EKO both have to hand one `core` to every
unit in the build, including third-party ones, and it must be the same one for all of them. A per-crate dependency line
cannot do that alone. So the dependency is declared once per workspace and the build driver applies it to every unit.

- **Declared once.** The workspace names the packages and versions, as ordinary dependencies of one small member
  (or `[workspace.dependencies]`). Cargo resolves and fetches them like anything else.
- **With EKO (stage1 and later).** The Cargo driver (`ekopathrs/tools/cargo-eko-source-driver`) adds those packages as the
  language roots of every unit's source graph. This is the fix for the refusal in section 1 and needs no flag.
- **With rustc (stage0).** One build driver, the existing Cargo VIP `--build-driver` boundary, does three things:
  builds the three packages first; runs the real build with an explicit `--target` so host tools (build scripts,
  proc macros) keep the host compiler's own library and only target units are affected; and gives every target unit
  the built `core` and an otherwise empty library root, so rustc's own `core` cannot be picked up. It needs
  `RUSTC_BOOTSTRAP=1`, as `bootstrap/build.ts` already uses.
- **The default.** The driver refuses to build a target unit if rustc's own `core` would be used. Opting back in is
  an explicit, visible flag.

Unproven and to be tested before anything is committed to: that cargo and rustc accept one injected `core` for
third-party units without also building a second copy for the crates that name it.

## 5. The real size of this: the bootstrap `core` is a subset

`ekod` links the frontend (sixty-odd rustc crates), and Karen, `pathdb` and `ps-wild` pull in third-party crates
(`serde`, `gimli`, `object`, `hashbrown`). All of them use far more of `core` than 5,800 lines provide. Today they
compile only because rustc lends them its full `core`. Turning the default off before the package is complete breaks
every one of them.

So the first step is a measurement, not a switch: compile each binary's units against the bootstrap `core` with no
default `core`, and count what is missing, per item and per crate. That number is the size of the job and its order.

## 6. Decisions for the owner

1. **Which source becomes the complete `core` for the compiler family?** Grow the ekostd bootstrap `core` until the
   census in section 5 is empty (keeps the rule that the compiler never touches the Rust standard library), or use the
   `core` inside the `ps-rust-stdlib` fork (complete today, but it is the standard library fork, reserved so far for
   user programs). Recommendation: grow the ekostd one, driven by the census, starting with `ps-wild` (already `no_std`).
2. **Registry:** crates.io or the private registry `ekostd`'s consumers already use.
3. **Karen and `pathdb` use `std`.** They cannot adopt this until they are `std`-free or the standard library fork is
   available as the ordinary dependency planned for user programs.

## 7. Order of work

1. Census (section 5): one wave, read-only, four binaries.
2. `Cargo.toml` for the three packages; `sources.json` generated.
3. EKO driver adds them as language roots to every unit; rerun the Karen build attempt.
4. rustc stage0 driver with the refusal-by-default, proven on `ps-wild` first.
5. Publish.
