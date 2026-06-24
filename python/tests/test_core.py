from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from copperlace import Copperlace, CopperlaceError, RuleSet, render_hocon_file, render_hocon_str


class CopperlaceTests(unittest.TestCase):
    def test_render_from_config_string(self) -> None:
        output = render_hocon_str('name = ["Mia"]\norigin = "{name}"', "origin")

        self.assertEqual(output, "Mia")

    def test_render_from_config_file(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "story.conf"
            path.write_text('name = ["Mia"]\norigin = "{name}"', encoding="utf-8")

            self.assertEqual(render_hocon_file(path, "origin"), "Mia")

    def test_render_from_config_string_with_context(self) -> None:
        output = render_hocon_str(
            'context { name = "Mia" }\norigin = "Hello {name}"',
            "origin",
            {"name": "Darcy"},
        )

        self.assertEqual(output, "Hello Darcy")

    def test_render_from_config_file_with_context(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "story.conf"
            path.write_text('origin = "Hello {name}"', encoding="utf-8")

            self.assertEqual(render_hocon_file(path, "origin", {"name": "Lina"}), "Hello Lina")

    def test_missing_rule_raises_error(self) -> None:
        with self.assertRaisesRegex(CopperlaceError, "unknown rule"):
            render_hocon_str('origin = "{missing}"', "origin")

    def test_builtin_processor_pipeline(self) -> None:
        output = render_hocon_str(
            'name = ["  mIA  "]\norigin = "{name | trim | capitalize}"',
            "origin",
        )

        self.assertEqual(output, "Mia")

    def test_custom_processor(self) -> None:
        output = render_hocon_str(
            'name = ["Mia"]\norigin = "{name | surround}"',
            "origin",
            processors={"surround": lambda value: f"'{value}'"},
        )

        self.assertEqual(output, "'Mia'")

    def test_custom_processor_overrides_builtin(self) -> None:
        output = render_hocon_str(
            'name = ["Mia"]\norigin = "{name | uppercase}"',
            "origin",
            processors={"uppercase": lambda _value: "custom"},
        )

        self.assertEqual(output, "custom")

    def test_custom_processor_exception_raises_error(self) -> None:
        def fail(_value: str) -> str:
            raise ValueError("not allowed")

        with self.assertRaisesRegex(CopperlaceError, "not allowed"):
            render_hocon_str(
                'name = ["Mia"]\norigin = "{name | fail}"',
                "origin",
                processors={"fail": fail},
            )

    def test_custom_processor_rejects_non_string_return(self) -> None:
        with self.assertRaisesRegex(CopperlaceError, "non-string"):
            render_hocon_str(
                'name = ["Mia"]\norigin = "{name | bad}"',
                "origin",
                processors={"bad": lambda _value: 1},  # type: ignore[dict-item,return-value]
            )

    def test_builtin_article_processor(self) -> None:
        output = render_hocon_str(
            'apple = ["apple"]\nuser = ["user"]\norigin = "{apple | article}/{user | article}"',
            "origin",
        )

        self.assertEqual(output, "an apple/a user")

    def test_builtin_past_tense_processor(self) -> None:
        output = render_hocon_str(
            'walk = ["walk"]\nrun = ["run"]\norigin = "{walk | past_tense}/{run | past_tense}"',
            "origin",
        )

        self.assertEqual(output, "walked/ran")

    def test_builtin_pluralize_processor(self) -> None:
        output = render_hocon_str(
            'cat = ["cat"]\nperson = ["person"]\norigin = "{cat | pluralize}/{person | pluralize}"',
            "origin",
        )

        self.assertEqual(output, "cats/people")

    def test_builtin_possessive_processor(self) -> None:
        output = render_hocon_str(
            'mia = ["Mia"]\njames = ["James"]\norigin = "{mia | possessive}/{james | possessive}"',
            "origin",
        )

        self.assertEqual(output, "Mia's/James'")

    def test_builtin_ordinal_processor(self) -> None:
        output = render_hocon_str(
            'one = [1]\neleven = [11]\ntwenty_three = [23]\norigin = "{one | ordinal}/{eleven | ordinal}/{twenty_three | ordinal}"',
            "origin",
        )

        self.assertEqual(output, "1st/11th/23rd")

    def test_builtin_slug_processor(self) -> None:
        output = render_hocon_str(
            'title = ["Mia\'s Story"]\norigin = "{title | slug}"',
            "origin",
        )

        self.assertEqual(output, "mias-story")

    def test_weighted_choice(self) -> None:
        output = render_hocon_str(
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
            'context { name = "Mia" }\nnext = "Darcy"\norigin = "{name}{name:=next}"'
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

    def test_context_rejects_non_string_key(self) -> None:
        with RuleSet.from_string('origin = "{name}"') as ruleset:
            with self.assertRaisesRegex(TypeError, "context keys"):
                ruleset.render("origin", {1: "Mia"})  # type: ignore[dict-item]

    def test_context_rejects_non_string_value(self) -> None:
        with RuleSet.from_string('origin = "{name}"') as ruleset:
            with self.assertRaisesRegex(TypeError, "context values"):
                ruleset.render("origin", {"name": 1})  # type: ignore[dict-item]

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

    def test_context_manager_closes_copperlace(self) -> None:
        with Copperlace.from_string('name = ["Mia"]\norigin = "{name}"') as copperlace:
            self.assertEqual(copperlace.render("origin"), "Mia")

        with self.assertRaisesRegex(CopperlaceError, "closed"):
            copperlace.render("origin")

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
