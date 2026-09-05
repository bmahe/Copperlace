# Load the precompiler module at config time so elixir_make (which compiles
# before the project's Elixir modules) can call its callbacks. The file lives
# at the project root, not in lib/, so the Elixir compiler does not recompile it.
Code.require_file("precompiler.ex", __DIR__)

defmodule Copperlace.MixProject do
  use Mix.Project

  @version "0.3.1"
  @source_url "https://github.com/bmahe/Copperlace"
  @homepage_url "https://bmahe.github.io/Copperlace/"

  def project do
    [
      app: :copperlace,
      version: @version,
      elixir: "~> 1.15",
      start_permanent: Mix.env() == :prod,
      compilers: [:elixir_make] ++ Mix.compilers(),
      make_cwd: "c_src",
      make_clean: ["clean"],
      # Precompiled NIF + native library download (see Copperlace.Precompiler).
      # Dev/test always compile from source; the published Hex package
      # (MIX_ENV=prod) downloads precompiled archives from GitHub Releases.
      make_force_build: Mix.env() != :prod,
      make_precompiler: {:nif, Copperlace.Precompiler},
      make_precompiler_filename: "copperlace_nif",
      make_precompiler_priv_paths: ["copperlace_nif.*", "native"],
      make_precompiler_url:
        "https://github.com/bmahe/Copperlace/releases/download/v#{@version}/@{artefact_filename}",
      # Precompiled archives are built against OTP 26 (NIF API 2.17).
      # OTP 27+ (NIF 2.18) falls back to 2.17 via elixir_make's version fallback.
      # OTP 25 and earlier compile the NIF shim from source.
      make_precompiler_nif_versions: [versions: ["2.17"]],
      deps: deps(),
      package: package(),
      description: description(),
      name: "Copperlace",
      source_url: @source_url,
      homepage_url: @homepage_url,
      docs: docs()
    ]
  end

  def application do
    []
  end

  defp deps do
    [
      {:elixir_make, "~> 0.8", runtime: false},
      {:ex_doc, "~> 0.34", only: :dev, runtime: false}
    ]
  end

  defp package do
    [
      name: "copperlace",
      licenses: ["Apache-2.0"],
      links: %{
        "GitHub" => @source_url,
        "Homepage" => @homepage_url,
        "Docs" => "https://bmahe.github.io/Copperlace/"
      },
      # The NIF shim ships as C source in c_src/ and compiles on install via
      # elixir_make; the compiled priv/copperlace_nif.* is not packaged. The
      # Copperlace native library is resolved at runtime via dlopen, or
      # downloaded as a precompiled archive when checksum.exs is present.
      files: ~w(lib c_src precompiler.ex checksum.exs LICENSE mix.exs README.md .formatter.exs)
    ]
  end

  defp description do
    "Elixir NIF wrapper for the Copperlace procedural text renderer, " <>
      "binding the shared Copperlace C ABI via Native Implemented Functions."
  end

  defp docs do
    [
      main: "Copperlace",
      source_ref: "v#{@version}",
      source_url: @source_url
    ]
  end
end
