# Copperlace (Elixir)

Elixir wrapper for [Copperlace](https://github.com/bmahe/Copperlace), the
procedural text renderer. This package binds the shared Copperlace C ABI to
the BEAM through Native Implemented Functions (NIFs).

## Install

Add `copperlace` to your `mix.exs` dependencies:

```elixir
def deps do
  [
    {:copperlace, "~> 0.3"}
  ]
end
```

The Hex package includes the Elixir NIF shim and uses `elixir_make`'s
precompiler to download a precompiled archive containing the NIF shim and the
platform native library at install time. No C toolchain is needed when the
precompiled archive is available. If the archive is unavailable for the current
platform or Erlang/OTP version, `elixir_make` compiles the NIF shim from
source and the native library is resolved at runtime in this order:

1. the `COPPERLACE_LIBRARY_PATH` environment variable, when set and pointing
   to an existing file;
2. the packaged native library under `priv/native/`;
3. the local Rust build output at `../rust-core/target/release/`.

For source-tree development, build the native library first:

```sh
make rust-build
```

## Usage

```elixir
config = ~s(name = ["Mia"]\norigin = "Hello {name}")
{:ok, copperlace} = Copperlace.from_string(config)
{:ok, "Hello Mia"} = Copperlace.render(copperlace, "origin")
```

`render/3` accepts an optional context map and options:

```elixir
{:ok, _} = Copperlace.render(copperlace, "origin", %{"name" => "Darcy"})
{:ok, _} = Copperlace.render(copperlace, "origin", %{}, max_recursion_depth: 2)
```

Inferred and structured rendering are also available:

```elixir
{:ok, _} = Copperlace.render_inferred(copperlace, "origin")
{:ok, _} = Copperlace.render_structured(copperlace, "origin")
```

`render!/3`, `render_inferred!/3`, and `render_structured!/3` raise
`Copperlace.Error` on failure.

Release the native handle explicitly with `Copperlace.close/1`. Handles are
also released automatically when the BEAM garbage-collects the resource, but
explicit closing is recommended for long-lived renderers.

## Custom processors

This first release uses the builtin processor registry only (`article`,
`possessive`, `pluralize`, `sentence`, `slug`, `uppercase`, and the rest).
Custom Elixir processor callbacks are not yet supported; they will be added in
a follow-up. The CLI also uses builtin processors only.

## License

Apache License, Version 2.0. See `LICENSE` in the repository root.
