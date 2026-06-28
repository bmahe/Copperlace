package dev.mahe.copperlace;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.Test;

final class CopperlaceTest {
    @Test
    void rendersFromString() {
        assertEquals(
                "Mia",
                Copperlace.renderString("""
                        name = ["Mia"]
                        origin = "{name}"
                        """, "origin"));
    }

    @Test
    void rendersFromFile() throws IOException {
        final Path config = Files.createTempFile("copperlace", ".conf");
        try {
            Files.writeString(config, """
                    name = ["Mia"]
                    origin = "{name}"
                    """);

            assertEquals("Mia", Copperlace.renderFile(config, "origin"));
        } finally {
            Files.deleteIfExists(config);
        }
    }

    @Test
    void rendersFromStringWithContext() {
        assertEquals(
                "Hello Darcy",
                Copperlace.renderString("""
                        context {
                            name = "Mia"
                        }
                        origin = "Hello {name}"
                        """, "origin", Map.of("name", "Darcy")));
    }

    @Test
    void rendersFromFileWithContext() throws IOException {
        final Path config = Files.createTempFile("copperlace", ".conf");
        try {
            Files.writeString(config, """
                    origin = "Hello {name}"
                    """);

            assertEquals("Hello Lina", Copperlace.renderFile(config, "origin", Map.of("name", "Lina")));
        } finally {
            Files.deleteIfExists(config);
        }
    }

    @Test
    void missingRuleRaisesException() {
        final CopperlaceException exception = assertThrows(
                CopperlaceException.class,
                () -> Copperlace.renderString("""
                        origin = "{missing}"
                        """, "origin"));

        assertTrue(exception.getMessage().contains("unknown rule"));
    }

    @Test
    void rendersBuiltinProcessorPipeline() {
        assertEquals(
                "Mia",
                Copperlace.renderString("""
                        name = ["  mIA  "]
                        origin = "{name | trim | capitalize}"
                        """, "origin"));
    }

    @Test
    void rendersCustomProcessor() {
        assertEquals(
                "'Mia'",
                Copperlace.renderStringWithProcessors(
                        """
                        name = ["Mia"]
                        origin = "{name | surround}"
                        """,
                        "origin",
                        Map.of("surround", value -> "'" + value + "'")));
    }

    @Test
    void customProcessorOverridesBuiltinProcessor() {
        assertEquals(
                "custom",
                Copperlace.renderStringWithProcessors(
                        """
                        name = ["Mia"]
                        origin = "{name | uppercase}"
                        """,
                        "origin",
                        Map.of("uppercase", value -> "custom")));
    }

    @Test
    void customProcessorExceptionRaisesCopperlaceException() {
        final CopperlaceException exception = assertThrows(
                CopperlaceException.class,
                () -> Copperlace.renderStringWithProcessors(
                        """
                        name = ["Mia"]
                        origin = "{name | fail}"
                        """,
                        "origin",
                        Map.of("fail", value -> {
                            throw new IllegalStateException("not allowed");
                        })));

        assertTrue(exception.getMessage().contains("not allowed"));
    }

    @Test
    void customProcessorNullReturnRaisesCopperlaceException() {
        final CopperlaceException exception = assertThrows(
                CopperlaceException.class,
                () -> Copperlace.renderStringWithProcessors(
                        """
                        name = ["Mia"]
                        origin = "{name | missing_return}"
                        """,
                        "origin",
                        Map.of("missing_return", value -> null)));

        assertTrue(exception.getMessage().contains("returned null"));
    }

    @Test
    void rendersBuiltinArticleProcessor() {
        assertEquals(
                "an apple/a user",
                Copperlace.renderString("""
                        apple = ["apple"]
                        user = ["user"]
                        origin = "{apple | article}/{user | article}"
                        """, "origin"));
    }

    @Test
    void rendersBuiltinPastTenseProcessor() {
        assertEquals(
                "walked/ran",
                Copperlace.renderString("""
                        walk = ["walk"]
                        run = ["run"]
                        origin = "{walk | past_tense}/{run | past_tense}"
                        """, "origin"));
    }

    @Test
    void rendersBuiltinPluralizeProcessor() {
        assertEquals(
                "cats/people",
                Copperlace.renderString("""
                        cat = ["cat"]
                        person = ["person"]
                        origin = "{cat | pluralize}/{person | pluralize}"
                        """, "origin"));
    }

    @Test
    void rendersBuiltinPossessiveProcessor() {
        assertEquals(
                "Mia's/James'",
                Copperlace.renderString("""
                        mia = ["Mia"]
                        james = ["James"]
                        origin = "{mia | possessive}/{james | possessive}"
                        """, "origin"));
    }

    @Test
    void rendersBuiltinOrdinalProcessor() {
        assertEquals(
                "1st/11th/23rd",
                Copperlace.renderString("""
                        one = [1]
                        eleven = [11]
                        twenty_three = [23]
                        origin = "{one | ordinal}/{eleven | ordinal}/{twenty_three | ordinal}"
                        """, "origin"));
    }

    @Test
    void rendersBuiltinSlugProcessor() {
        assertEquals(
                "mias-story",
                Copperlace.renderString("""
                        title = ["Mia's Story"]
                        origin = "{title | slug}"
                        """, "origin"));
    }

    @Test
    void rendersWeightedChoice() {
        assertEquals(
                "rare",
                Copperlace.renderString("""
                        origin = [
                            { value = "common", weight = 0 },
                            { value = "rare", weight = 2.5 }
                        ]
                        """, "origin"));
    }

    @Test
    void rejectsBlankConfig() {
        final IllegalArgumentException exception =
                assertThrows(
                        IllegalArgumentException.class,
                        () -> Copperlace.renderString(" ", "origin"));

        assertTrue(exception.getMessage().contains("config"));
    }

    @Test
    void rejectsBlankRule() {
        final IllegalArgumentException exception =
                assertThrows(
                        IllegalArgumentException.class,
                        () -> Copperlace.renderString("name = [\"Mia\"]", " "));

        assertTrue(exception.getMessage().contains("rule"));
    }

    @Test
    void rejectsBlankPath() {
        final IllegalArgumentException exception =
                assertThrows(IllegalArgumentException.class, () -> Copperlace.renderFile(" ", "origin"));

        assertTrue(exception.getMessage().contains("path"));
    }

    @Test
    void rulesetRejectsBlankConfig() {
        final IllegalArgumentException exception =
                assertThrows(IllegalArgumentException.class, () -> RuleSet.fromString(" "));

        assertTrue(exception.getMessage().contains("config"));
    }

    @Test
    void rulesetRejectsBlankRule() {
        try (final RuleSet rules = RuleSet.fromString("""
                name = ["Mia"]
                origin = "{name}"
                """)) {
            final IllegalArgumentException exception =
                    assertThrows(IllegalArgumentException.class, () -> rules.render(" "));

            assertTrue(exception.getMessage().contains("rule"));
        }
    }

    @Test
    void rendersRepeatedlyFromOneRuleSet() {
        try (final RuleSet rules = RuleSet.fromString("""
                name = ["Mia"]
                origin = "{name}"
                """)) {
            assertEquals("Mia", rules.render("origin"));
            assertEquals("Mia", rules.render("origin"));
        }
    }

    @Test
    void rulesetRendersWithContext() {
        try (final RuleSet rules = RuleSet.fromString("""
                context {
                    name = "Mia"
                }
                next = "Darcy"
                origin = "{name}{% name:=next %}"
                """)) {
            assertEquals("Lina", rules.render("origin", Map.of("name", "Lina")));
            assertEquals("Lina", rules.render("origin", Map.of("name", "Lina")));
        }
    }

    @Test
    void rulesetRendersWithCustomProcessor() {
        try (final RuleSet rules = RuleSet.fromStringWithProcessors(
                """
                name = ["Mia"]
                origin = "{name | surround}"
                """,
                Map.of("surround", value -> "[" + value + "]"))) {
            assertEquals("[Mia]", rules.render("origin"));
            assertEquals("[Mia]", rules.render("origin"));
        }
    }

    @Test
    void rendersRepeatedlyFromOneCopperlaceInstance() {
        try (final Copperlace copperlace = Copperlace.fromString("""
                name = ["Mia"]
                pet = ["owl"]
                origin = "{name}"
                companion = "{name} and {pet}"
                """)) {
            assertEquals("Mia", copperlace.render("origin"));
            assertEquals("Mia and owl", copperlace.render("companion"));
            assertEquals("Mia", copperlace.render("origin"));
        }
    }

    @Test
    void copperlaceRendersWithContext() {
        try (final Copperlace copperlace = Copperlace.fromString("""
                origin = "{name}"
                """)) {
            assertEquals("Mia", copperlace.render("origin", Map.of("name", "Mia")));
        }
    }

    @Test
    void renderWithContextRejectsNullContext() {
        try (final RuleSet rules = RuleSet.fromString("""
                origin = "{name}"
                """)) {
            assertThrows(NullPointerException.class, () -> rules.render("origin", null));
        }
    }

    @Test
    void renderWithContextRejectsNullContextKey() {
        final Map<String, String> context = new HashMap<>();
        context.put(null, "Mia");

        try (final RuleSet rules = RuleSet.fromString("""
                origin = "{name}"
                """)) {
            assertThrows(NullPointerException.class, () -> rules.render("origin", context));
        }
    }

    @Test
    void renderWithContextRejectsNullContextValue() {
        final Map<String, String> context = new HashMap<>();
        context.put("name", null);

        try (final RuleSet rules = RuleSet.fromString("""
                origin = "{name}"
                """)) {
            assertThrows(NullPointerException.class, () -> rules.render("origin", context));
        }
    }

    @Test
    void customProcessorWorksWithInitialContext() {
        assertEquals(
                "[Mia]",
                Copperlace.renderStringWithProcessors(
                        """
                        origin = "{name | surround}"
                        """,
                        "origin",
                        Map.of("name", "Mia"),
                        Map.of("surround", value -> "[" + value + "]")));
    }

    @Test
    void customProcessorsRejectNullMap() {
        assertThrows(
                NullPointerException.class,
                () -> RuleSet.fromStringWithProcessors(
                        """
                        origin = "Mia"
                        """,
                        null));
    }

    @Test
    void customProcessorsRejectNullProcessorName() {
        final Map<String, CopperlaceProcessor> processors = new HashMap<>();
        processors.put(null, value -> value);

        assertThrows(
                NullPointerException.class,
                () -> RuleSet.fromStringWithProcessors(
                        """
                        origin = "Mia"
                        """,
                        processors));
    }

    @Test
    void customProcessorsRejectNullProcessor() {
        final Map<String, CopperlaceProcessor> processors = new HashMap<>();
        processors.put("custom", null);

        assertThrows(
                NullPointerException.class,
                () -> RuleSet.fromStringWithProcessors(
                        """
                        origin = "Mia"
                        """,
                        processors));
    }

    @Test
    void copperlaceLoadsFromFile() throws IOException {
        final Path config = Files.createTempFile("copperlace", ".conf");
        try {
            Files.writeString(config, """
                    name = ["Mia"]
                    origin = "{name}"
                    """);

            try (final Copperlace copperlace = Copperlace.fromFile(config)) {
                assertEquals("Mia", copperlace.render("origin"));
                assertEquals("Mia", copperlace.render("origin"));
            }
        } finally {
            Files.deleteIfExists(config);
        }
    }

    @Test
    void copperlaceRejectsBlankConfig() {
        final IllegalArgumentException exception =
                assertThrows(IllegalArgumentException.class, () -> Copperlace.fromString(" "));

        assertTrue(exception.getMessage().contains("config"));
    }

    @Test
    void copperlaceRejectsBlankPath() {
        final IllegalArgumentException exception =
                assertThrows(IllegalArgumentException.class, () -> Copperlace.fromFile(" "));

        assertTrue(exception.getMessage().contains("path"));
    }

    @Test
    void copperlaceRejectsBlankRule() {
        try (final Copperlace copperlace = Copperlace.fromString("""
                name = ["Mia"]
                origin = "{name}"
                """)) {
            final IllegalArgumentException exception =
                    assertThrows(IllegalArgumentException.class, () -> copperlace.render(" "));

            assertTrue(exception.getMessage().contains("rule"));
        }
    }

    @Test
    void closeIsIdempotentAndRenderFailsAfterClose() {
        final RuleSet rules = RuleSet.fromString("""
                name = ["Mia"]
                origin = "{name}"
                """);

        rules.close();
        rules.close();

        final CopperlaceException exception =
                assertThrows(CopperlaceException.class, () -> rules.render("origin"));
        assertTrue(exception.getMessage().contains("closed"));
    }

    @Test
    void copperlaceCloseIsIdempotentAndRenderFailsAfterClose() {
        final Copperlace copperlace = Copperlace.fromString("""
                name = ["Mia"]
                origin = "{name}"
                """);

        copperlace.close();
        copperlace.close();

        final CopperlaceException exception =
                assertThrows(CopperlaceException.class, () -> copperlace.render("origin"));
        assertTrue(exception.getMessage().contains("closed"));
    }

    @Test
    void usesClassifierScopedNativeResourcePath() {
        assertEquals(
                "dev/mahe/copperlace/native/linux-x86_64/libcopperlace.so",
                NativeLibrary.packagedResourcePath("linux-x86_64", "libcopperlace.so"));
    }

    @Test
    void detectsHostNativeLibraryName() {
        final String os = System.getProperty("os.name").toLowerCase();
        final String expected;
        if (os.contains("win")) {
            expected = "copperlace.dll";
        } else if (os.contains("mac") || os.contains("darwin")) {
            expected = "libcopperlace.dylib";
        } else {
            expected = "libcopperlace.so";
        }

        assertEquals(expected, NativeLibrary.nativeLibraryName());
    }
}
