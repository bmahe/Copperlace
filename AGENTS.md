# Repository Guidelines

## Project Structure & Module Organization

Copperlace is split into a Rust core plus language wrappers. `rust-core/` contains the renderer library, C ABI, and CLI; source lives in `rust-core/src/`, with integration tests in `rust-core/tests/`. `docs/` holds the AsciiDoc behavior and configuration specifications. `python/` contains the Python wheel wrapper and `ctypes` bindings. `java/` contains the Java FFM wrapper under package `dev.mahe.copperlace`.

## Build, Test, and Development Commands

- `make help`: lists the root Makefile workflows.
- `make check`: runs Rust formatting checks plus Rust, Python, and Java tests.
- `make rust-build` / `make rust-cli`: builds the Rust library, native library, and CLI, or runs the sample CLI render command.
- `make python-wheel`: builds a platform wheel and bundles the Rust native library.
- `make java-package`: builds the Java API JAR plus the current-platform native classifier JAR.
- `make clean`: removes Rust, Python, and Java build outputs.

## Coding Style & Naming Conventions

Rust code must pass `cargo fmt --check`; keep tests outside `rust-core/src/`. Python uses small typed modules and standard-library `unittest`. Java uses Maven layout, Java FFM APIs, and package `dev.mahe.copperlace`. Keep Maven dependency and plugin versions in properties. Code defensively, including explicit precondition checks and using Java's `final` modifier wherever practical for locals, parameters, fields, and classes. Prefer descriptive names that match the renderer domain: rules, nodes, bindings, and render contexts.

## Testing Guidelines

Add behavior coverage in the wrapper closest to the change. Rust renderer behavior belongs in `rust-core/tests/`; Python wrapper behavior in `python/tests/`; Java FFM behavior in `java/src/test/`. Keep tests direct and focused on observable rendering, binding, configuration, packaging, or wrapper behavior. Do not commit generated artifacts such as `target/`, `python/build/`, `python/dist/`, `*.egg-info`, `__pycache__/`, or `java/target/`.

## Packaging Notes

Python wheels and Java native classifier JARs are platform-specific because they include the Rust dynamic library. Java runtime loading checks `COPPERLACE_LIBRARY_PATH` first, then packaged native resources, then local Rust build output for source-tree development. Use `make package` to validate both wrapper packaging paths together.

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
