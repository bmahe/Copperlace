defmodule CopperlaceTest do
  use ExUnit.Case, async: true

  alias Copperlace.{Error, Nif}

  @moduletag :requires_native

  @hello_config ~s(name = ["Mia"]\norigin = "Hello {name}")

  describe "from_string/2 and render/4" do
    test "compiles a config string and renders a rule" do
      {:ok, copperlace} = Copperlace.from_string(@hello_config)
      assert {:ok, "Hello Mia"} = Copperlace.render(copperlace, "origin")
    end

    test "render!/4 returns the string directly" do
      {:ok, copperlace} = Copperlace.from_string(@hello_config)
      assert "Hello Mia" = Copperlace.render!(copperlace, "origin")
    end

    test "renders repeatedly from one loaded config" do
      {:ok, copperlace} = Copperlace.from_string(@hello_config)
      assert {:ok, "Hello Mia"} = Copperlace.render(copperlace, "origin")
      assert {:ok, "Hello Mia"} = Copperlace.render(copperlace, "origin")
    end
  end

  describe "from_file/2" do
    test "compiles a config file and renders a rule" do
      tmp = Path.join(System.tmp_dir!(), "copperlace_file_#{:rand.uniform(1_000_000)}.conf")
      File.write!(tmp, @hello_config)

      try do
        {:ok, copperlace} = Copperlace.from_file(tmp)
        assert {:ok, "Hello Mia"} = Copperlace.render(copperlace, "origin")
      after
        File.rm(tmp)
      end
    end
  end

  describe "render with context" do
    test "initial context overrides defaults" do
      config = ~s(context { name = "Mia" }\norigin = "Hello {name}")
      {:ok, copperlace} = Copperlace.from_string(config)
      assert {:ok, "Hello Darcy"} = Copperlace.render(copperlace, "origin", %{"name" => "Darcy"})
    end

    test "empty context is allowed" do
      {:ok, copperlace} = Copperlace.from_string(@hello_config)
      assert {:ok, "Hello Mia"} = Copperlace.render(copperlace, "origin", %{})
    end

    test "non-string context value returns an error" do
      {:ok, copperlace} = Copperlace.from_string(@hello_config)

      assert {:error, %Error{status: :invalid_argument}} =
               Copperlace.render(copperlace, "origin", %{"name" => 123})
    end

    test "non-map context returns an error" do
      {:ok, copperlace} = Copperlace.from_string(@hello_config)

      assert {:error, %Error{status: :invalid_argument}} =
               Copperlace.render(copperlace, "origin", [{"name", "Mia"}])
    end
  end

  describe "builtin processors" do
    test "article processor chooses a or an" do
      config = ~s(item = ["owl"]\norigin = "You found {item | article}.")
      {:ok, copperlace} = Copperlace.from_string(config)
      assert {:ok, "You found an owl."} = Copperlace.render(copperlace, "origin")
    end

    test "uppercase processor" do
      config = ~s(name = ["mia"]\norigin = "{name | uppercase}")
      {:ok, copperlace} = Copperlace.from_string(config)
      assert {:ok, "MIA"} = Copperlace.render(copperlace, "origin")
    end

    test "slug processor" do
      config = ~s(item = ["Ancient Key"]\norigin = "{item | slug}")
      {:ok, copperlace} = Copperlace.from_string(config)
      assert {:ok, "ancient-key"} = Copperlace.render(copperlace, "origin")
    end
  end

  describe "render_inferred/4" do
    test "returns text for string-valued rules" do
      {:ok, copperlace} = Copperlace.from_string(@hello_config)
      assert {:ok, "Hello Mia"} = Copperlace.render_inferred(copperlace, "origin")
    end

    test "returns formatted JSON for object-valued rules" do
      config = ~s(name = ["Mia"]\norigin {\n  name = "{name}"\n  greeting = "Hello {name}"\n})
      {:ok, copperlace} = Copperlace.from_string(config)

      assert {:ok, json} = Copperlace.render_inferred(copperlace, "origin")
      assert String.contains?(json, "\"name\": \"Mia\"")
      assert String.contains?(json, "\"greeting\": \"Hello Mia\"")
    end
  end

  describe "render_structured/4" do
    setup do
      config = ~s(name = ["Mia"]\norigin {\n  name = "{name}"\n  greeting = "Hello {name}"\n})
      {:ok, copperlace} = Copperlace.from_string(config)
      %{copperlace: copperlace}
    end

    test "returns formatted JSON by default", %{copperlace: copperlace} do
      assert {:ok, json} = Copperlace.render_structured(copperlace, "origin")
      assert String.contains?(json, "\"name\": \"Mia\"")
      assert String.contains?(json, "\t")
    end

    test "returns compact JSON when format_json is false", %{copperlace: copperlace} do
      assert {:ok, json} =
               Copperlace.render_structured(copperlace, "origin", %{}, format_json: false)

      assert String.contains?(json, "\"name\":\"Mia\"")
      refute String.contains?(json, "\t")
    end

    test "structured!/4 raises on a non-structured rule" do
      {:ok, copperlace} = Copperlace.from_string(@hello_config)

      assert_raise Error, fn ->
        Copperlace.render_structured!(copperlace, "origin")
      end
    end
  end

  describe "errors" do
    test "unknown rule returns a render error" do
      {:ok, copperlace} = Copperlace.from_string(@hello_config)

      assert {:error, %Error{status: :render_error}} =
               Copperlace.render(copperlace, "missing-rule")
    end

    test "invalid config returns a parse error" do
      assert {:error, %Error{status: :parse_error}} =
               Copperlace.from_string("this is not = valid = config {{{")
    end

    test "render!/4 raises Copperlace.Error on failure" do
      {:ok, copperlace} = Copperlace.from_string(@hello_config)

      assert_raise Error, fn ->
        Copperlace.render!(copperlace, "missing-rule")
      end
    end
  end

  describe "close/1" do
    test "renders after close return an error" do
      {:ok, copperlace} = Copperlace.from_string(@hello_config)
      assert :ok = Copperlace.close(copperlace)

      assert {:error, %Error{}} = Copperlace.render(copperlace, "origin")
    end

    test "close is idempotent" do
      {:ok, copperlace} = Copperlace.from_string(@hello_config)
      assert :ok = Copperlace.close(copperlace)
      assert :ok = Copperlace.close(copperlace)
    end
  end

  describe "custom processors" do
    test "passing :processors raises ArgumentError" do
      assert_raise ArgumentError, fn ->
        Copperlace.from_string(@hello_config, processors: %{"shout" => &String.upcase/1})
      end
    end
  end

  describe "max recursion depth" do
    test "circular reference with depth 0 returns an error" do
      config = ~s(echo = "{echo}"\norigin = "{echo}")
      {:ok, copperlace} = Copperlace.from_string(config)

      assert {:error, %Error{status: :render_error}} =
               Copperlace.render(copperlace, "origin", %{}, max_recursion_depth: 0)
    end

    test "limited recursion returns ok output" do
      config = ~s(echo = "{echo}"\norigin = "{echo}")
      {:ok, copperlace} = Copperlace.from_string(config)

      assert {:ok, _} = Copperlace.render(copperlace, "origin", %{}, max_recursion_depth: 2)
    end
  end

  describe "native library resolution" do
    test "Copperlace.Native exposes the platform library name" do
      name = Copperlace.Native.library_name()
      assert name in ["libcopperlace.so", "libcopperlace.dylib", "copperlace.dll"]
    end

    test "Copperlace.Native.find_library/0 returns a path when the native lib is loaded" do
      assert Nif.loaded()
      assert is_binary(Copperlace.Native.find_library())
    end
  end
end
