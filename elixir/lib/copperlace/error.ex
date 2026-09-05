defmodule Copperlace.Error do
  @moduledoc """
  Exception raised by Copperlace when parsing, compiling, or rendering fails.

  The `:status` field maps the C ABI status code to an atom:

    * `:invalid_argument` - invalid arguments passed to the native call.
    * `:parse_error` - the configuration could not be parsed or compiled.
    * `:render_error` - a render failed, such as an unknown rule or processor error.
    * `:native_not_loaded` - the Copperlace native library was not found at load time.
  """

  @enforce_keys [:status, :message]
  defexception [:status, :message]

  @type status ::
          :ok | :invalid_argument | :parse_error | :render_error | :native_not_loaded | :unknown
  @type t :: %__MODULE__{status: status(), message: String.t()}

  @impl true
  def message(%__MODULE__{message: message}), do: message
end
