# Repository Guidelines

## Project Structure & Module Organization

Copperlace is split into a Rust core plus language wrappers. `rust-core/` contains the renderer library, C ABI, CLI, and WASM exports; source lives in `rust-core/src/`, with integration tests in `rust-core/tests/`. `docs/` holds the AsciiDoc behavior and configuration specifications. `python/` contains the Python wheel wrapper and `ctypes` bindings. `java/` contains the Java FFM wrapper under package `dev.mahe.copperlace`. `js/` contains generated JS/WASM package output when built. `scripts/` contains packaging and release smoke-test helpers. `.github/workflows/` contains Pages, package-check, and release automation.

## Build, Test, and Development Commands

- `make help`: lists the root Makefile workflows.
- `make check`: runs Rust formatting checks plus Rust, Python, and Java tests.
- `make rust-build` / `make rust-cli`: builds the Rust library, native library, and CLI, or runs the sample CLI render command.
- `make cli-archive`: builds a current-platform CLI and native-library release archive.
- `make python-wheel`: builds a platform wheel and bundles the Rust native library.
- `make js-package` / `make js-web`: builds bundler or direct-browser JS/WASM packages into `js/pkg/`.
- `make java-package`: builds the Java API JAR plus the current-platform native classifier JAR.
- `make package`: builds current-platform CLI/native, Python, JS/WASM, and Java distributable artifacts.
- `make release-check`: verifies package version consistency across Rust, Python, and Java metadata.
- `make clean`: removes Rust, Python, JS/WASM, Java, and release archive build outputs.
- Prefer `podman` over `docker` for containerized development and release commands.

## Coding Style & Naming Conventions

Rust code must pass `cargo fmt --check`; keep tests outside `rust-core/src/`. Python uses small typed modules and standard-library `unittest`. Java uses Maven layout, Java FFM APIs, and package `dev.mahe.copperlace`. Keep Maven dependency and plugin versions in properties. Packaging helpers in `scripts/` should be small, typed Python scripts using only the standard library unless a repo toolchain dependency already exists. Code defensively, including explicit precondition checks and using Java's `final` modifier wherever practical for locals, parameters, fields, and classes. Prefer descriptive names that match the renderer domain: rules, nodes, bindings, render contexts, processors, choices, and release artifacts.

## Testing Guidelines

Add behavior coverage in the wrapper closest to the change. Tests must live in separate test files, never colocated with production source code; `make check` enforces this for Rust source files. Rust renderer behavior belongs in `rust-core/tests/`; Python wrapper behavior in `python/tests/`; Java FFM behavior in `java/src/test/`. Packaging behavior should be covered by the relevant `scripts/smoke_*` helper and, when possible, the GitHub Actions package-check workflow. Keep tests direct and focused on observable rendering, binding, configuration, packaging, or wrapper behavior. Do not commit generated artifacts such as `target/`, `python/build/`, `python/dist/`, `*.egg-info`, `__pycache__/`, `java/target/`, `js/pkg/`, or local release archives.

## Packaging Notes

Python wheels, Java native classifier JARs, and CLI archives are platform-specific because they include the Rust dynamic library. Java runtime loading checks `COPPERLACE_LIBRARY_PATH` first, then packaged native resources, then local Rust build output for source-tree development. Python wheels bundle the native library under `copperlace/native/`. JS/WASM packages are generated artifacts from `wasm-pack` and must not be committed.

The first-class release targets are `linux-x86_64`, `linux-aarch64`, `macos-x86_64`, `macos-aarch64`, and `windows-x86_64`. Release tags use `v<version>` and publish GitHub Release assets through `.github/workflows/release.yml`. Use `make release-check` before tagging, and use the relevant smoke scripts for installed-artifact validation.

The project is licensed under Apache-2.0. Keep package license metadata aligned with the root `LICENSE`.

## Commit & Pull Request Guidelines

Use short imperative commit subjects matching project history, such as `Add article processor`, `Move tests out of source`, or `Rename Java package coordinates`. Small focused changes can use a subject-only commit. Add a commit body when the subject alone does not explain intent, behavior, API impact, packaging changes, native-library effects, or cross-language wrapper implications. Commit bodies should state the purpose of the change, its user or maintenance impact, and why that impact matters; include relevant verification commands when useful. Do not use the body merely to list edited files.

Example subject-only commit:

```text
Add article processor
```

Example commit with body:

```text
Add reusable Copperlace API

Expose a load-once renderer in Rust, Python, and Java so callers can render
multiple rules without recompiling config each time. This reduces repeated
render overhead for applications that reuse the same configuration.

Verified with make check.
```

Pull requests should summarize behavior changes, list verification commands run, and call out native-library, packaging, or cross-language API impacts.
