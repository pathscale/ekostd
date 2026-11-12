# bootstrap

This folder holds the source for the three language units and the lists that say which units stage1 builds.

Rule: stage1 builds exactly what stage0 builds. The unit set is the Cargo unit graph of `ekod`, not a list written by hand. Nothing built by rustc is linked into a stage1 output.

## The two files

`sources.json` is the hand-written list. It has five units. It shows how far stage1 has got. It is not what stage1 is. It also supplies the three language units (`core`, `compiler_builtins`, `alloc`) that the generated list starts with.

`units-from-cargo.ts` writes the full list from Cargo. It is the only way to get the real unit set. Do not extend `sources.json` by hand.

## Generate the full list

Pass a Cargo manifest and a file to write. Cargo runs offline, for the target in `sources.json` (aarch64-apple-darwin), with `RUSTUP_TOOLCHAIN=stable`. This is the stage0 producer, so rustc is allowed here and nowhere else.

    bun bootstrap/units-from-cargo.ts --manifest /Users/revenge/code/ekopathrs/ekod/Cargo.toml --out /tmp/ekod-sources.json

The file uses the same schema as `sources.json` (`ekostd.bootstrap/1`).

- The three language units come first, copied from `sources.json`.
- Then one unit per library target that is linked into the binary. Normal dependencies are followed. Units are listed dependency first.
- Every unit lists `core`, `alloc` and `compiler_builtins` first, then its Cargo dependencies, by key. `build.ts` requires `compiler_builtins`.
- Roots are absolute paths.
- A key is the crate name. If one crate appears in two versions, both keys end with the version, for example `syn_1_0_109`.
- `features` holds the features Cargo enabled. `build.ts` passes them as `--cfg feature="x"`.
- `extern_names` appears only when a dependency is renamed in Cargo.
- A unit with a build script has `build_script` with the path to it. `build.ts` does not run build scripts.
- `host_units` lists what is not linked into the binary: proc-macro packages (kind `proc-macro`), packages reached through build dependencies (kind `build-dependency`), and what those need in turn (kind `host-dependency`). A host unit may also be a linked unit. These need a host program to run, so they are listed apart and `build.ts` ignores them.

The command stops with an error when a package has no library target, when two units would get the same key, or when a key is not a valid identifier.

## Use the list

    bun bootstrap/build.ts --sources /tmp/ekod-sources.json --out /absolute/fresh/dir --run

Without `--sources`, `build.ts` reads `sources.json` as before.

## Names

A stage names the producer. Stage0 is cargo with stable rustc, stage1 is built by the stage0 compiler.

The second run of the same compiler is called `pass2`, never `stage2`. It proves the output is repeatable. It is not a stage.
