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

    def test_missing_rule_raises_error(self) -> None:
        with self.assertRaisesRegex(CopperlaceError, "unknown rule"):
            render_hocon_str('origin = "{missing}"', "origin")

    def test_builtin_processor_pipeline(self) -> None:
        output = render_hocon_str(
            'name = ["  mIA  "]\norigin = "{name | trim | capitalize}"',
            "origin",
        )

        self.assertEqual(output, "Mia")

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

    def test_repeated_renders_on_one_ruleset(self) -> None:
        ruleset = RuleSet.from_string('name = ["Mia"]\norigin = "{name}"')
        try:
            self.assertEqual(ruleset.render("origin"), "Mia")
            self.assertEqual(ruleset.render("origin"), "Mia")
        finally:
            ruleset.close()

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
