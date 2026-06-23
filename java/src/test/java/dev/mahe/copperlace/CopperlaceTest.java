package dev.mahe.copperlace;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import org.junit.jupiter.api.Test;

final class CopperlaceTest {
    @Test
    void rendersFromString() {
        assertEquals(
                "Mia",
                Copperlace.renderHoconString("""
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

            assertEquals("Mia", Copperlace.renderHoconFile(config, "origin"));
        } finally {
            Files.deleteIfExists(config);
        }
    }

    @Test
    void missingRuleRaisesException() {
        final CopperlaceException exception = assertThrows(
                CopperlaceException.class,
                () -> Copperlace.renderHoconString("""
                        origin = "{missing}"
                        """, "origin"));

        assertTrue(exception.getMessage().contains("unknown rule"));
    }

    @Test
    void rendersBuiltinProcessorPipeline() {
        assertEquals(
                "Mia",
                Copperlace.renderHoconString("""
                        name = ["  mIA  "]
                        origin = "{name | trim | capitalize}"
                        """, "origin"));
    }

    @Test
    void rendersBuiltinArticleProcessor() {
        assertEquals(
                "an apple/a user",
                Copperlace.renderHoconString("""
                        apple = ["apple"]
                        user = ["user"]
                        origin = "{apple | article}/{user | article}"
                        """, "origin"));
    }

    @Test
    void rendersBuiltinPastTenseProcessor() {
        assertEquals(
                "walked/ran",
                Copperlace.renderHoconString("""
                        walk = ["walk"]
                        run = ["run"]
                        origin = "{walk | past_tense}/{run | past_tense}"
                        """, "origin"));
    }

    @Test
    void rendersBuiltinPluralizeProcessor() {
        assertEquals(
                "cats/people",
                Copperlace.renderHoconString("""
                        cat = ["cat"]
                        person = ["person"]
                        origin = "{cat | pluralize}/{person | pluralize}"
                        """, "origin"));
    }

    @Test
    void rendersBuiltinPossessiveProcessor() {
        assertEquals(
                "Mia's/James'",
                Copperlace.renderHoconString("""
                        mia = ["Mia"]
                        james = ["James"]
                        origin = "{mia | possessive}/{james | possessive}"
                        """, "origin"));
    }

    @Test
    void rendersBuiltinOrdinalProcessor() {
        assertEquals(
                "1st/11th/23rd",
                Copperlace.renderHoconString("""
                        one = [1]
                        eleven = [11]
                        twenty_three = [23]
                        origin = "{one | ordinal}/{eleven | ordinal}/{twenty_three | ordinal}"
                        """, "origin"));
    }

    @Test
    void rejectsBlankConfig() {
        final IllegalArgumentException exception =
                assertThrows(
                        IllegalArgumentException.class,
                        () -> Copperlace.renderHoconString(" ", "origin"));

        assertTrue(exception.getMessage().contains("config"));
    }

    @Test
    void rejectsBlankRule() {
        final IllegalArgumentException exception =
                assertThrows(
                        IllegalArgumentException.class,
                        () -> Copperlace.renderHoconString("name = [\"Mia\"]", " "));

        assertTrue(exception.getMessage().contains("rule"));
    }

    @Test
    void rejectsBlankPath() {
        final IllegalArgumentException exception =
                assertThrows(IllegalArgumentException.class, () -> Copperlace.renderHoconFile(" ", "origin"));

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
