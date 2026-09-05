defmodule Copperlace.Precompiler do
  @moduledoc false

  # Implements the ElixirMake.Precompiler behaviour so that `elixir_make`
  # downloads a precompiled archive containing the NIF shim and the bundled
  # Copperlace native library at install time.
  #
  # Precompiled archives are built against OTP 25 (NIF API 2.16) and are
  # backward-compatible with OTP 26+ (NIF API 2.17) via elixir_make's NIF
  # version fallback.
  #
  # Archive structure (extracted to priv/):
  #   copperlace_nif.so / .dll / .dylib   (precompiled NIF shim)
  #   native/libcopperlace.so / ...       (the Copperlace renderer)
  #
  # When the precompiled archive is unavailable for the current target,
  # elixir_make falls back to compiling the NIF shim from source. The
  # renderer is then resolved at runtime via COPPERLACE_LIBRARY_PATH or
  # a local rust-core build.
  @supported_targets [
    "x86_64-linux-gnu",
    "aarch64-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-windows-msvc"
  ]

  def all_supported_targets(:fetch), do: @supported_targets
  def all_supported_targets(:compile), do: current_target_list()

  def current_target do
    case :os.type() do
      {:win32, _} ->
        arch =
          System.get_env("PROCESSOR_ARCHITECTURE", "")
          |> String.downcase()
          |> String.trim()
          |> normalize_arch_name()

        {:ok, "#{arch}-windows-msvc"}

      {:unix, :darwin} ->
        arch = normalize_arch(:erlang.system_info(:system_architecture))
        {:ok, "#{arch}-apple-darwin"}

      {:unix, _} ->
        sys = to_string(:erlang.system_info(:system_architecture))
        parts = String.split(sys, "-", trim: true)

        arch = normalize_arch_name(List.first(parts) || "")

        cond do
          arch == "" ->
            {:error, "cannot determine current target from: #{sys}"}

          Enum.any?(parts, &String.starts_with?(&1, "linux")) ->
            {:ok, "#{arch}-linux-gnu"}

          Enum.any?(parts, &String.starts_with?(&1, "gnu")) ->
            {:ok, "#{arch}-linux-gnu"}

          true ->
            {:ok, sys}
        end
    end
  end

  def build_native(args) do
    ElixirMake.Precompiler.mix_compile(args)
  end

  def precompile(args, _target) do
    ElixirMake.Precompiler.mix_compile(args)
    :ok
  end

  def post_precompile_target(_target), do: :ok

  def post_precompile, do: :ok

  def unavailable_target(_target), do: :compile

  defp current_target_list do
    case current_target() do
      {:ok, target} -> [target]
      {:error, _} -> []
    end
  end

  defp normalize_arch(system_architecture) when is_list(system_architecture) do
    system_architecture
    |> to_string()
    |> String.split("-", parts: 2)
    |> hd()
    |> normalize_arch_name()
  end

  defp normalize_arch_name(arch) do
    case String.downcase(arch) do
      a when a in ["amd64", "x86_64"] -> "x86_64"
      a when a in ["aarch64", "arm64"] -> "aarch64"
      other -> other
    end
  end
end
