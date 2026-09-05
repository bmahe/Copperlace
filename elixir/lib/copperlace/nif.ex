defmodule Copperlace.Nif do
  @moduledoc false

  # Thin NIF binding to the Copperlace C ABI. The NIF shared object is loaded
  # once when this module is first referenced. The native Copperlace library
  # itself is resolved by the NIF at load time (see c_src/copperlace_nif.c).

  @on_load :load_nif

  def load_nif do
    nif = :filename.join(:code.priv_dir(:copperlace), "copperlace_nif")
    # Pass the priv directory as load_info so the NIF can resolve the
    # precompiled native library under <priv>/native/ regardless of the
    # consumer's working directory.
    priv_dir = :code.priv_dir(:copperlace)

    case :erlang.load_nif(nif, priv_dir) do
      :ok ->
        :ok

      {:error, reason} ->
        # Loading the NIF object itself failed (missing build, wrong ABI).
        # Keep the module loadable so callers get a clear error from the
        # stubs instead of a module-load failure.
        IO.puts(:stderr, "copperlace: failed to load NIF #{nif}: #{inspect(reason)}")
        :ok
    end
  end

  @doc false
  def from_string_raw(_config), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def from_file_raw(_path), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def render_raw(_handle, _rule, _context, _max_recursion_depth),
    do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def render_inferred_raw(_handle, _rule, _context, _max_recursion_depth),
    do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def render_structured_raw(_handle, _rule, _context, _max_recursion_depth, _format_json),
    do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def close_raw(_handle), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def loaded, do: :erlang.nif_error(:nif_not_loaded)
end
