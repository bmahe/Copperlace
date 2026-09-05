nif_loaded? =
  try do
    Copperlace.Nif.loaded() == true
  rescue
    _ -> false
  catch
    :error, _ -> false
  end

exclude = if nif_loaded?, do: [], else: [:requires_native]

ExUnit.start(exclude: exclude)
