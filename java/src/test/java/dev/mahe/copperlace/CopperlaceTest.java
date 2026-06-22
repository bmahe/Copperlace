package dev.mahe.copperlace;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import org.junit.jupiter.api.Test;

class CopperlaceTest {
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
        Path config = Files.createTempFile("copperlace", ".conf");
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
        CopperlaceException exception = assertThrows(
                CopperlaceException.class,
                () -> Copperlace.renderHoconString("""
                        origin = "{missing}"
                        """, "origin"));

        assertTrue(exception.getMessage().contains("unknown rule"));
    }

    @Test
    void rendersRepeatedlyFromOneRuleSet() {
        try (RuleSet rules = RuleSet.fromString("""
                name = ["Mia"]
                origin = "{name}"
                """)) {
            assertEquals("Mia", rules.render("origin"));
            assertEquals("Mia", rules.render("origin"));
        }
    }

    @Test
    void closeIsIdempotentAndRenderFailsAfterClose() {
        RuleSet rules = RuleSet.fromString("""
                name = ["Mia"]
                origin = "{name}"
                """);

        rules.close();
        rules.close();

        CopperlaceException exception =
                assertThrows(CopperlaceException.class, () -> rules.render("origin"));
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
        String os = System.getProperty("os.name").toLowerCase();
        String expected;
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
