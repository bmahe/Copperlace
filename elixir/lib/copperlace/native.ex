defmodule Copperlace.Native do
  @moduledoc false

  # Elixir-side mirror of the native library resolution performed by the NIF
  # (c_src/copperlace_nif.c) and of python/copperlace/_native.py:find_library.
  # Used by the packaging step to stage the native library under priv/native/
  # and by runtime diagnostics. The NIF performs its own resolution at load
  # time; this module is the source of truth for the candidate order.

  @doc "Returns the platform native library file name."
  @spec library_name() :: String.t()
  def library_name do
    case :os.type() do
      {:win32, _} -> "copperlace.dll"
      {:unix, :darwin} -> "libcopperlace.dylib"
      {:unix, _} -> "libcopperlace.so"
    end
  end

  @doc "Returns the first existing candidate path, or nil."
  @spec find_library() :: String.t() | nil
  def find_library do
    name = library_name()

    candidates(name)
    |> Enum.find(&File.exists?/1)
  end

  @doc "Returns the candidate paths in resolution order."
  @spec candidates(String.t()) :: [String.t()]
  def candidates(name) do
    override = System.get_env("COPPERLACE_LIBRARY_PATH")

    packaged =
      case :code.priv_dir(:copperlace) do
        {:error, _} -> ["priv/native/#{name}", "native/#{name}"]
        dir -> [Path.join([dir, "native", name])]
      end

    source = [
      "../rust-core/target/release/#{name}",
      "../../rust-core/target/release/#{name}",
      "rust-core/target/release/#{name}"
    ]

    override_list = if override && override != "", do: [override], else: []
    override_list ++ packaged ++ source
  end
end
