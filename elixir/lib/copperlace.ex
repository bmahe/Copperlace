defmodule Copperlace do
  @moduledoc """
  Elixir wrapper for the Copperlace procedural text renderer.

  `Copperlace` compiles a configuration once into a native ruleset handle and
  renders named rules from it repeatedly. It binds the shared Copperlace C ABI
  through Native Implemented Functions (NIFs).

  ## Example

      {:ok, copperlace} = Copperlace.from_string(~s(name = ["Mia"]\\norigin = "Hello {name}"))
      {:ok, "Hello Mia"} = Copperlace.render(copperlace, "origin")

  The native handle is released by `close/1` or automatically when the BEAM
  garbage-collects the resource. Call `close/1` for long-lived renderers.

  Custom Elixir processor callbacks are not yet supported; this release uses
  the builtin processor registry only.
  """

  alias Copperlace.{Error, Nif}

  defstruct [:handle]

  @type t :: %__MODULE__{handle: reference()}

  @type context :: %{String.t() => String.t()} | %{}

  @type render_option :: {:max_recursion_depth, non_neg_integer()}
  @type structured_option ::
          {:max_recursion_depth, non_neg_integer()} | {:format_json, boolean()}

  @doc """
  Compiles a configuration string into a renderer.

  Custom processors are not supported in this release. Passing `:processors`
  raises `ArgumentError`.
  """
  @spec from_string(String.t(), keyword()) :: {:ok, t()} | {:error, Error.t()}
  def from_string(config, opts \\ []) when is_binary(config) and is_list(opts) do
    ensure_no_processors!(opts)

    case Nif.from_string_raw(config) do
      {:ok, handle} -> {:ok, %__MODULE__{handle: handle}}
      {:error, status, message} -> {:error, to_error(status, message)}
    end
  end

  @doc """
  Compiles a configuration file into a renderer.

  Custom processors are not supported in this release. Passing `:processors`
  raises `ArgumentError`.
  """
  @spec from_file(Path.t(), keyword()) :: {:ok, t()} | {:error, Error.t()}
  def from_file(path, opts \\ []) when is_list(opts) do
    ensure_no_processors!(opts)
    path = to_string(path)

    case Nif.from_file_raw(path) do
      {:ok, handle} -> {:ok, %__MODULE__{handle: handle}}
      {:error, status, message} -> {:error, to_error(status, message)}
    end
  end

  @doc """
  Renders a named rule as text.

  `context` is a map of initial string bindings for this render only.
  Each render uses a fresh context.

  ## Options

    * `:max_recursion_depth` (default `0`) - recursive re-entries allowed.
  """
  @spec render(t(), String.t(), context(), [render_option()]) ::
          {:ok, String.t()} | {:error, Error.t()}
  def render(copperlace, rule, context \\ %{}, opts \\ [])

  def render(%__MODULE__{handle: handle}, rule, context, opts) when is_binary(rule) do
    max_recursion = max_recursion_depth(opts)

    with :ok <- validate_context(context) do
      case Nif.render_raw(handle, rule, context, max_recursion) do
        {:ok, output} -> {:ok, output}
        {:error, status, message} -> {:error, to_error(status, message)}
      end
    end
  end

  @doc """
  Renders a named rule, returning formatted JSON for object-valued rules.

  String-valued and list-valued rules render as normal text. Object-valued
  rules return formatted JSON.
  """
  @spec render_inferred(t(), String.t(), context(), [render_option()]) ::
          {:ok, String.t()} | {:error, Error.t()}
  def render_inferred(copperlace, rule, context \\ %{}, opts \\ [])

  def render_inferred(%__MODULE__{handle: handle}, rule, context, opts) when is_binary(rule) do
    max_recursion = max_recursion_depth(opts)

    with :ok <- validate_context(context) do
      case Nif.render_inferred_raw(handle, rule, context, max_recursion) do
        {:ok, output} -> {:ok, output}
        {:error, status, message} -> {:error, to_error(status, message)}
      end
    end
  end

  @doc """
  Renders a named structured rule as JSON text.

  ## Options

    * `:format_json` (default `true`) - return formatted JSON with tabs; `false`
      returns compact JSON.
    * `:max_recursion_depth` (default `0`).
  """
  @spec render_structured(t(), String.t(), context(), [structured_option()]) ::
          {:ok, String.t()} | {:error, Error.t()}
  def render_structured(copperlace, rule, context \\ %{}, opts \\ [])

  def render_structured(%__MODULE__{handle: handle}, rule, context, opts) when is_binary(rule) do
    max_recursion = max_recursion_depth(opts)
    format_json = Keyword.get(opts, :format_json, true)

    with :ok <- validate_context(context) do
      case Nif.render_structured_raw(handle, rule, context, max_recursion, format_json) do
        {:ok, output} -> {:ok, output}
        {:error, status, message} -> {:error, to_error(status, message)}
      end
    end
  end

  @doc """
  Renders a named rule as text, raising `Copperlace.Error` on failure.
  """
  @spec render!(t(), String.t(), context(), [render_option()]) :: String.t()
  def render!(copperlace, rule, context \\ %{}, opts \\ []) do
    case render(copperlace, rule, context, opts) do
      {:ok, output} -> output
      {:error, error} -> raise error
    end
  end

  @doc """
  Renders a named rule as inferred text or JSON, raising `Copperlace.Error` on failure.
  """
  @spec render_inferred!(t(), String.t(), context(), [render_option()]) :: String.t()
  def render_inferred!(copperlace, rule, context \\ %{}, opts \\ []) do
    case render_inferred(copperlace, rule, context, opts) do
      {:ok, output} -> output
      {:error, error} -> raise error
    end
  end

  @doc """
  Renders a named structured rule as JSON text, raising `Copperlace.Error` on failure.
  """
  @spec render_structured!(t(), String.t(), context(), [structured_option()]) :: String.t()
  def render_structured!(copperlace, rule, context \\ %{}, opts \\ []) do
    case render_structured(copperlace, rule, context, opts) do
      {:ok, output} -> output
      {:error, error} -> raise error
    end
  end

  @doc """
  Releases the native ruleset handle. Calling `close/1` more than once is safe.
  Rendering after close returns an error from the native layer.
  """
  @spec close(t()) :: :ok
  def close(%__MODULE__{handle: handle}) do
    Nif.close_raw(handle)
    :ok
  end

  defp max_recursion_depth(opts) do
    case Keyword.get(opts, :max_recursion_depth, 0) do
      n when is_integer(n) and n >= 0 ->
        n

      other ->
        raise ArgumentError,
              "max_recursion_depth must be a non-negative integer, got: #{inspect(other)}"
    end
  end

  defp validate_context(context) when is_map(context) do
    Enum.reduce_while(context, :ok, fn {key, value}, _acc ->
      cond do
        not is_binary(key) ->
          {:halt,
           {:error, %Error{status: :invalid_argument, message: "context keys must be strings"}}}

        not is_binary(value) ->
          {:halt,
           {:error, %Error{status: :invalid_argument, message: "context values must be strings"}}}

        true ->
          {:cont, :ok}
      end
    end)
    |> case do
      :ok -> :ok
      {:error, _} = e -> e
    end
  end

  defp validate_context(_other) do
    {:error, %Error{status: :invalid_argument, message: "context must be a map"}}
  end

  defp ensure_no_processors!(opts) do
    if Keyword.has_key?(opts, :processors) do
      raise ArgumentError,
            "custom processors are not supported in this Copperlace release; " <>
              "the builtin processor registry is used"
    end

    :ok
  end

  defp to_error(status, message) do
    %Error{status: status, message: message}
  end
end
