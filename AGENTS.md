# Repository Guidelines

## Project Structure & Module Organization

Copperlace is split into a Rust core plus language wrappers. `rust-core/` contains the renderer library, C ABI, and CLI; source lives in `rust-core/src/`, with integration tests in `rust-core/tests/`. `docs/` holds the AsciiDoc behavior and configuration specifications. `python/` contains the Python wheel wrapper and `ctypes` bindings. `java/` contains the Java FFM wrapper under package `dev.mahe.copperlace`.

## Build, Test, and Development Commands

- `cd rust-core && cargo build --release`: builds the CLI, Rust library, and native dynamic library.
- `cd rust-core && cargo test`: runs Rust integration tests for renderer, CLI, nodes, and FFI.
- `cd rust-core && cargo run --bin copperlace -- render --config example.conf --rule origin`: runs the CLI locally.
- `PYTHONPATH=python python -m unittest discover -s python/tests`: runs Python wrapper tests against the local native library.
- `cd python && python -m build --wheel`: builds a platform wheel and bundles the Rust native library.
- `cd java && mvn -q test`: runs Java FFM tests; tests build the Rust native library first.

## Coding Style & Naming Conventions

Rust code must pass `cargo fmt --check`; keep tests outside `rust-core/src/`. Python uses small typed modules and standard-library `unittest`. Java uses Maven layout, Java FFM APIs, and package `dev.mahe.copperlace`. Prefer descriptive names that match the renderer domain: rules, nodes, bindings, and render contexts.

## Testing Guidelines

Add behavior coverage in the wrapper closest to the change. Rust renderer behavior belongs in `rust-core/tests/`; Python wrapper behavior in `python/tests/`; Java FFM behavior in `java/src/test/`. Keep tests direct and focused on observable rendering, binding, configuration, or wrapper behavior. Do not commit generated artifacts such as `target/`, `python/build/`, `python/dist/`, `*.egg-info`, or `java/target/`.

## Commit & Pull Request Guidelines

Use short imperative commit subjects matching project history, such as `Add Python wheel wrapper`, `Move tests out of source`, or `Rename Java package coordinates`. Pull requests should summarize behavior changes, list verification commands run, and call out native-library, packaging, or cross-language API impacts.
