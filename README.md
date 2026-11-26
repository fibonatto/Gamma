# gamma

Lean 4.36.0 checker for macOS Apple Silicon (`aarch64-apple-darwin`). Rust owns project discovery, import graph construction, caching, CLI and output. Lean owns parsing, elaboration, environment inspection and diagnostics.

## Important implementation boundary

This repository is an initial native-FFI integration, not yet a self-contained distributable. `build.rs` invokes Lake **only during compilation** to build `LeanChecker` as a shared library. At runtime Rust does not spawn `lean`, `lake`, or `gamma`; it calls the Lean backend through the C ABI.

The current macOS build links `libLeanChecker.dylib` and `libleanshared.dylib` dynamically. Therefore the Lean shared runtime must remain available at the recorded rpath. For redistribution, the next packaging step is to copy the matching Lean dylibs and Lean library tree into an application bundle and rewrite their install names/rpaths. This is not yet a single-file executable. The FFI ABI is documented by Lean as an internal, potentially unstable interface.

The checker does not compile or rebuild imported project modules. It consumes the project's existing `.olean` artifacts through Lean's search path. This is deliberate for the checker phase: compiling source dependencies without invoking the Lean CLI requires adding a native compilation pipeline, which is a separate feature and must not be simulated by touching timestamps.

## Requirements

- Apple Silicon macOS
- Rust toolchain
- Lean/elan installation pinned to `leanprover/lean4:v4.36.0`
- `lake` available at build time

## Build

```sh
cd gamma-lean
cargo build --release --target aarch64-apple-darwin
```

If Lean is installed outside the default elan location, set `LEAN_SYSROOT` to the Lean toolchain sysroot. `LAKE` can override the Lake executable path.

## Run

```sh
./target/aarch64-apple-darwin/release/gamma path/to/File.lean
```

At runtime, `LEAN_PATH` is assembled from the project's `.lake/build/lib/lean`, local `src`/`lean` roots, package build directories and the Lean standard library. Existing `LEAN_PATH` entries are retained.

## Cache

The JSON cache is stored under `.gamma/cache.json`. Its key includes source contents for the reachable local import graph, search paths and the pinned Lean version. It does not use second-resolution mtimes. Delete `.gamma` to force a fresh inspection.

## Known next engineering steps

1. Compile and exercise the FFI against the exact v4.36.0 Apple Silicon distribution. This source environment does not contain Lean, Lake or a Rust toolchain, so the native build has not been executed here.
2. Correct any API/ABI drift revealed by that build before tagging a release.
3. Bundle Lean shared libraries and the standard library into a relocatable macOS app layout.
4. Add a native Lean module-compilation pipeline if `gamma` is expected to produce `.olean` files rather than check against existing artifacts.
5. Expand project resolution to read Lake package configuration and dependency source roots rather than relying on conventional roots.
6. Add integration tests with real projects, including Mathlib, module cycles, missing `.olean` files, `sorry`, parser errors and imported declarations.
