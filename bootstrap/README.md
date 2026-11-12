# bootstrap

Source for the three language units: `core/`, `alloc/` and `compiler_builtins/`. Each is a bare
`src/` directory. `core` is a `#![no_core]` subset, enough for the in-repo bootstrap units and
not yet for third-party crates.

There is no build harness here. The script that compiled a hand-made unit list with rustc and
then with ekod (`build.ts`, with `units-from-cargo.ts` and `sources.json`) was removed on
2026-11-12: it resolved features wrongly, wrote paths ekod refuses, and prepended these three
units to every crate by hand. Units come from Cargo through
`ekopathrs/tools/cargo-eko-source-driver`. How a build is given its `core` is the open design in
[../docs/CORE-AS-A-DEPENDENCY.md](../docs/CORE-AS-A-DEPENDENCY.md).
