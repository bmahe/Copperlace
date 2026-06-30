from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from copperlace import (
    Copperlace,
    CopperlaceError,
    RuleSet,
    render_file,
    render_file_inferred,
    render_file_structured,
    render_str,
    render_str_inferred,
    render_str_structured,
)
from copperlace._native import native


class CopperlaceTests(unittest.TestCase):
    def test_render_from_config_string(self) -> None:
        output = render_str('name = ["Mia"]\norigin = "{name}"', "origin")

        self.assertEqual(output, "Mia")

    def test_render_from_config_file(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "story.conf"
            path.write_text('name = ["Mia"]\norigin = "{name}"', encoding="utf-8")

            self.assertEqual(render_file(path, "origin"), "Mia")

    def test_render_from_config_string_with_context(self) -> None:
        output = render_str(
            'context { name = "Mia" }\norigin = "Hello {name}"',
            "origin",
            {"name": "Darcy"},
        )

        self.assertEqual(output, "Hello Darcy")

    def test_render_from_config_file_with_context(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "story.conf"
            path.write_text('origin = "Hello {name}"', encoding="utf-8")

            self.assertEqual(render_file(path, "origin", {"name": "Lina"}), "Hello Lina")

    def test_render_structured_from_config_string(self) -> None:
        output = render_str_structured(
            """
            name = ["Mia"]
            origin {
                title = "Hello {name}"
                items = ["one", "two"]
                count = 3
                large = 18446744073709551615
                ratio = 2.5
                active = true
                missing = null
                nested {
                    value = "ok"
                    values = ["three", "four"]
                }
            }
            """,
            "origin",
        )

        self.assertIs(type(output), str)
        self.assertEqual(
            output,
            '{\n\t"active": true,\n\t"count": 3,\n\t"items": [\n\t\t"one",\n\t\t"two"\n\t],\n\t"large": 18446744073709551615,\n\t"missing": null,\n\t"nested": {\n\t\t"value": "ok",\n\t\t"values": [\n\t\t\t"three",\n\t\t\t"four"\n\t\t]\n\t},\n\t"ratio": 2.5,\n\t"title": "Hello Mia"\n}',
        )

    def test_render_structured_from_config_file(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "story.conf"
            path.write_text(
                """
                origin {
                    title = "Hello"
                    items = ["one", "two"]
                }
                """,
                encoding="utf-8",
            )

            self.assertEqual(
                render_file_structured(path, "origin"),
                '{\n\t"items": [\n\t\t"one",\n\t\t"two"\n\t],\n\t"title": "Hello"\n}',
            )

    def test_render_structured_with_context(self) -> None:
        output = render_str_structured(
            """
            context {
                name = "Mia"
            }
            origin {
                greeting = "Hello {name}"
            }
            """,
            "origin",
            {"name": "Lina"},
        )

        self.assertEqual(output, '{\n\t"greeting": "Hello Lina"\n}')

    def test_render_structured_with_builtin_and_custom_processors(self) -> None:
        output = render_str_structured(
            """
            name = "Mia"
            origin {
                builtin = "{name | uppercase}"
                custom = "{name | surround}"
            }
            """,
            "origin",
            processors={"surround": lambda value: f"[{value}]"},
        )

        self.assertEqual(output, '{\n\t"builtin": "MIA",\n\t"custom": "[Mia]"\n}')

    def test_native_structured_json_defaults_to_formatted(self) -> None:
        with RuleSet.from_string('origin { greeting = "Hello Mia" }') as ruleset:
            output = native().ruleset_render_structured_json(ruleset._handle, "origin")

        self.assertEqual(output, '{\n\t"greeting": "Hello Mia"\n}')

    def test_render_inferred_from_config_string(self) -> None:
        text = render_str_inferred(
            """
            text = "Mia"
            choice = ["Lina"]
            origin {
                greeting = "Hello {name}"
            }
            """,
            "text",
        )
        choice = render_str_inferred(
            """
            text = "Mia"
            choice = ["Lina"]
            origin {
                greeting = "Hello {name}"
            }
            """,
            "choice",
        )
        structured = render_str_inferred(
            """
            text = "Mia"
            choice = ["Lina"]
            origin {
                greeting = "Hello {name}"
            }
            """,
            "origin",
            {"name": "Darcy"},
        )

        self.assertEqual(text, "Mia")
        self.assertEqual(choice, "Lina")
        self.assertIs(type(structured), str)
        self.assertEqual(structured, '{\n\t"greeting": "Hello Darcy"\n}')

    def test_render_inferred_from_config_file(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "story.conf"
            path.write_text(
                """
                text = "Mia"
                origin {
                    greeting = "Hello {name}"
                }
                """,
                encoding="utf-8",
            )

            self.assertEqual(render_file_inferred(path, "text"), "Mia")
            self.assertEqual(
                render_file_inferred(path, "origin", {"name": "Lina"}),
                '{\n\t"greeting": "Hello Lina"\n}',
            )

    def test_missing_rule_raises_error(self) -> None:
        with self.assertRaisesRegex(CopperlaceError, "unknown rule"):
            render_str('origin = "{missing}"', "origin")

    def test_structured_render_error_raises_copperlace_error(self) -> None:
        with self.assertRaisesRegex(CopperlaceError, "unknown rule"):
            render_str_structured('origin { value = "{missing}" }', "origin")

    def test_builtin_processor_pipeline(self) -> None:
        output = render_str(
            'name = ["  mIA  "]\norigin = "{name | trim | capitalize}"',
            "origin",
        )

        self.assertEqual(output, "Mia")

    def test_custom_processor(self) -> None:
        output = render_str(
            'name = ["Mia"]\norigin = "{name | surround}"',
            "origin",
            processors={"surround": lambda value: f"'{value}'"},
        )

        self.assertEqual(output, "'Mia'")

    def test_custom_processor_overrides_builtin(self) -> None:
        output = render_str(
            'name = ["Mia"]\norigin = "{name | uppercase}"',
            "origin",
            processors={"uppercase": lambda _value: "custom"},
        )

        self.assertEqual(output, "custom")

    def test_custom_processor_exception_raises_error(self) -> None:
        def fail(_value: str) -> str:
            raise ValueError("not allowed")

        with self.assertRaisesRegex(CopperlaceError, "not allowed"):
            render_str(
                'name = ["Mia"]\norigin = "{name | fail}"',
                "origin",
                processors={"fail": fail},
            )

    def test_custom_processor_rejects_non_string_return(self) -> None:
        with self.assertRaisesRegex(CopperlaceError, "non-string"):
            render_str(
                'name = ["Mia"]\norigin = "{name | bad}"',
                "origin",
                processors={"bad": lambda _value: 1},  # type: ignore[dict-item,return-value]
            )

    def test_builtin_article_processor(self) -> None:
        output = render_str(
            'apple = ["apple"]\nuser = ["user"]\norigin = "{apple | article}/{user | article}"',
            "origin",
        )

        self.assertEqual(output, "an apple/a user")

    def test_builtin_past_tense_processor(self) -> None:
        output = render_str(
            'walk = ["walk"]\nrun = ["run"]\norigin = "{walk | past_tense}/{run | past_tense}"',
            "origin",
        )

        self.assertEqual(output, "walked/ran")

    def test_builtin_pluralize_processor(self) -> None:
        output = render_str(
            'cat = ["cat"]\nperson = ["person"]\norigin = "{cat | pluralize}/{person | pluralize}"',
            "origin",
        )

        self.assertEqual(output, "cats/people")

    def test_builtin_possessive_processor(self) -> None:
        output = render_str(
            'mia = ["Mia"]\njames = ["James"]\norigin = "{mia | possessive}/{james | possessive}"',
            "origin",
        )

        self.assertEqual(output, "Mia's/James'")

    def test_builtin_ordinal_processor(self) -> None:
        output = render_str(
            'one = [1]\neleven = [11]\ntwenty_three = [23]\norigin = "{one | ordinal}/{eleven | ordinal}/{twenty_three | ordinal}"',
            "origin",
        )

        self.assertEqual(output, "1st/11th/23rd")

    def test_builtin_slug_processor(self) -> None:
        output = render_str(
            'title = ["Mia\'s Story"]\norigin = "{title | slug}"',
            "origin",
        )

        self.assertEqual(output, "mias-story")

    def test_weighted_choice(self) -> None:
        output = render_str(
            'origin = [{ value = "common", weight = 0 }, { value = "rare", weight = 2.5 }]',
            "origin",
        )

        self.assertEqual(output, "rare")

    def test_repeated_renders_on_one_ruleset(self) -> None:
        ruleset = RuleSet.from_string('name = ["Mia"]\norigin = "{name}"')
        try:
            self.assertEqual(ruleset.render("origin"), "Mia")
            self.assertEqual(ruleset.render("origin"), "Mia")
        finally:
            ruleset.close()

    def test_ruleset_renders_with_context(self) -> None:
        with RuleSet.from_string(
            'context { name = "Mia" }\nnext = "Darcy"\norigin = "{name}{% name:=next %}"'
        ) as ruleset:
            self.assertEqual(ruleset.render("origin", {"name": "Lina"}), "Lina")
            self.assertEqual(ruleset.render("origin", {"name": "Lina"}), "Lina")

    def test_ruleset_renders_with_custom_processor(self) -> None:
        with RuleSet.from_string(
            'name = ["Mia"]\norigin = "{name | surround}"',
            {"surround": lambda value: f"[{value}]"},
        ) as ruleset:
            self.assertEqual(ruleset.render("origin"), "[Mia]")
            self.assertEqual(ruleset.render("origin"), "[Mia]")

    def test_ruleset_renders_structured(self) -> None:
        with RuleSet.from_string(
            """
            name = ["Mia"]
            plain = "{name}"
            origin {
                title = "{name}"
                tags = ["generated", "{name | slug}"]
            }
            """
        ) as ruleset:
            self.assertEqual(
                ruleset.render_structured("origin"),
                '{\n\t"tags": [\n\t\t"generated",\n\t\t"mia"\n\t],\n\t"title": "Mia"\n}',
            )
            self.assertEqual(ruleset.render("plain"), "Mia")

    def test_ruleset_renders_inferred(self) -> None:
        with RuleSet.from_string(
            """
            text = "Mia"
            origin {
                greeting = "Hello {name}"
            }
            """
        ) as ruleset:
            self.assertEqual(ruleset.render_inferred("text"), "Mia")
            self.assertEqual(
                ruleset.render_inferred("origin", {"name": "Darcy"}),
                '{\n\t"greeting": "Hello Darcy"\n}',
            )

    def test_repeated_renders_on_one_copperlace_instance(self) -> None:
        copperlace = Copperlace.from_string(
            'name = ["Mia"]\npet = ["owl"]\norigin = "{name}"\ncompanion = "{name} and {pet}"'
        )
        try:
            self.assertEqual(copperlace.render("origin"), "Mia")
            self.assertEqual(copperlace.render("companion"), "Mia and owl")
            self.assertEqual(copperlace.render("origin"), "Mia")
        finally:
            copperlace.close()

    def test_copperlace_renders_with_context(self) -> None:
        with Copperlace.from_string('origin = "{name}"') as copperlace:
            self.assertEqual(copperlace.render("origin", {"name": "Mia"}), "Mia")

    def test_copperlace_renders_structured_with_context(self) -> None:
        with Copperlace.from_string('origin { greeting = "Hello {name}" }') as copperlace:
            self.assertEqual(
                copperlace.render_structured("origin", {"name": "Mia"}),
                '{\n\t"greeting": "Hello Mia"\n}',
            )

    def test_copperlace_renders_inferred_with_context(self) -> None:
        with Copperlace.from_string('origin { greeting = "Hello {name}" }') as copperlace:
            self.assertEqual(
                copperlace.render_inferred("origin", {"name": "Mia"}),
                '{\n\t"greeting": "Hello Mia"\n}',
            )

    def test_context_rejects_non_string_key(self) -> None:
        with RuleSet.from_string('origin = "{name}"') as ruleset:
            with self.assertRaisesRegex(TypeError, "context keys"):
                ruleset.render("origin", {1: "Mia"})  # type: ignore[dict-item]

    def test_context_rejects_non_string_value(self) -> None:
        with RuleSet.from_string('origin = "{name}"') as ruleset:
            with self.assertRaisesRegex(TypeError, "context values"):
                ruleset.render("origin", {"name": 1})  # type: ignore[dict-item]

    def test_structured_context_rejects_non_string_key(self) -> None:
        with RuleSet.from_string('origin { greeting = "Hello {name}" }') as ruleset:
            with self.assertRaisesRegex(TypeError, "context keys"):
                ruleset.render_structured("origin", {1: "Mia"})  # type: ignore[dict-item]

    def test_structured_context_rejects_non_string_value(self) -> None:
        with RuleSet.from_string('origin { greeting = "Hello {name}" }') as ruleset:
            with self.assertRaisesRegex(TypeError, "context values"):
                ruleset.render_structured("origin", {"name": 1})  # type: ignore[dict-item]

    def test_processors_reject_non_string_name(self) -> None:
        with self.assertRaisesRegex(TypeError, "processor names"):
            RuleSet.from_string(
                'origin = "{name | custom}"',
                {1: lambda value: value},  # type: ignore[dict-item]
            )

    def test_processors_reject_non_callable_value(self) -> None:
        with self.assertRaisesRegex(TypeError, "callable"):
            RuleSet.from_string(
                'origin = "{name | custom}"',
                {"custom": "nope"},  # type: ignore[dict-item]
            )

    def test_copperlace_loads_from_file(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "story.conf"
            path.write_text('name = ["Mia"]\norigin = "{name}"', encoding="utf-8")

            with Copperlace.from_file(path) as copperlace:
                self.assertEqual(copperlace.render("origin"), "Mia")
                self.assertEqual(copperlace.render("origin"), "Mia")

    def test_context_manager_closes_ruleset(self) -> None:
        with RuleSet.from_string('name = ["Mia"]\norigin = "{name}"') as ruleset:
            self.assertEqual(ruleset.render("origin"), "Mia")

        with self.assertRaisesRegex(CopperlaceError, "closed"):
            ruleset.render("origin")

        with self.assertRaisesRegex(CopperlaceError, "closed"):
            ruleset.render_structured("origin")

        with self.assertRaisesRegex(CopperlaceError, "closed"):
            ruleset.render_inferred("origin")

    def test_context_manager_closes_copperlace(self) -> None:
        with Copperlace.from_string('name = ["Mia"]\norigin = "{name}"') as copperlace:
            self.assertEqual(copperlace.render("origin"), "Mia")

        with self.assertRaisesRegex(CopperlaceError, "closed"):
            copperlace.render("origin")

        with self.assertRaisesRegex(CopperlaceError, "closed"):
            copperlace.render_structured("origin")

        with self.assertRaisesRegex(CopperlaceError, "closed"):
            copperlace.render_inferred("origin")

    def test_explicit_close_is_idempotent(self) -> None:
        ruleset = RuleSet.from_string('name = ["Mia"]\norigin = "{name}"')

        ruleset.close()
        ruleset.close()

        with self.assertRaisesRegex(CopperlaceError, "closed"):
            ruleset.render("origin")

    def test_copperlace_close_is_idempotent(self) -> None:
        copperlace = Copperlace.from_string('name = ["Mia"]\norigin = "{name}"')

        copperlace.close()
        copperlace.close()

        with self.assertRaisesRegex(CopperlaceError, "closed"):
            copperlace.render("origin")


if __name__ == "__main__":
    unittest.main()
