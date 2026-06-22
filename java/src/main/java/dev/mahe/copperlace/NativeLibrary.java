package dev.mahe.copperlace;

import java.io.IOException;
import java.io.InputStream;
import java.lang.foreign.Arena;
import java.lang.foreign.FunctionDescriptor;
import java.lang.foreign.Linker;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.SymbolLookup;
import java.lang.foreign.ValueLayout;
import java.lang.invoke.MethodHandle;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.Locale;
import java.util.Optional;

final class NativeLibrary {
    static final NativeLibrary INSTANCE = new NativeLibrary();

    private static final int COPPERLACE_OK = 0;
    private static final int COPPERLACE_PARSE_ERROR = 2;
    private static final int COPPERLACE_RENDER_ERROR = 3;

    private final Arena libraryArena = Arena.ofShared();
    private final MethodHandle rulesetFromFile;
    private final MethodHandle rulesetFromString;
    private final MethodHandle rulesetRender;
    private final MethodHandle rulesetFree;
    private final MethodHandle stringFree;

    private NativeLibrary() {
        Linker linker = Linker.nativeLinker();
        SymbolLookup lookup = SymbolLookup.libraryLookup(findLibrary(), libraryArena);
        rulesetFromFile = downcall(
                linker,
                lookup,
                "copperlace_ruleset_from_file",
                FunctionDescriptor.of(
                        ValueLayout.JAVA_INT,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS));
        rulesetFromString = downcall(
                linker,
                lookup,
                "copperlace_ruleset_from_string",
                FunctionDescriptor.of(
                        ValueLayout.JAVA_INT,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS));
        rulesetRender = downcall(
                linker,
                lookup,
                "copperlace_ruleset_render",
                FunctionDescriptor.of(
                        ValueLayout.JAVA_INT,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS));
        rulesetFree = downcall(
                linker,
                lookup,
                "copperlace_ruleset_free",
                FunctionDescriptor.ofVoid(ValueLayout.ADDRESS));
        stringFree = downcall(
                linker,
                lookup,
                "copperlace_string_free",
                FunctionDescriptor.ofVoid(ValueLayout.ADDRESS));
    }

    MemorySegment rulesetFromString(String config) {
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment outHandle = arena.allocate(ValueLayout.ADDRESS);
            MemorySegment outError = arena.allocate(ValueLayout.ADDRESS);
            MemorySegment configString = arena.allocateFrom(config);

            int status = (int) rulesetFromString.invokeExact(configString, outHandle, outError);
            checkStatus(status, outError);
            return outHandle.get(ValueLayout.ADDRESS, 0);
        } catch (CopperlaceException exception) {
            throw exception;
        } catch (Throwable throwable) {
            throw new CopperlaceException("Failed to create Copperlace ruleset", throwable);
        }
    }

    MemorySegment rulesetFromFile(Path path) {
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment outHandle = arena.allocate(ValueLayout.ADDRESS);
            MemorySegment outError = arena.allocate(ValueLayout.ADDRESS);
            MemorySegment pathString = arena.allocateFrom(path.toString());

            int status = (int) rulesetFromFile.invokeExact(pathString, outHandle, outError);
            checkStatus(status, outError);
            return outHandle.get(ValueLayout.ADDRESS, 0);
        } catch (CopperlaceException exception) {
            throw exception;
        } catch (Throwable throwable) {
            throw new CopperlaceException("Failed to create Copperlace ruleset", throwable);
        }
    }

    String render(MemorySegment handle, String rule) {
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment outString = arena.allocate(ValueLayout.ADDRESS);
            MemorySegment outError = arena.allocate(ValueLayout.ADDRESS);
            MemorySegment ruleString = arena.allocateFrom(rule);

            int status = (int) rulesetRender.invokeExact(handle, ruleString, outString, outError);
            checkStatus(status, outError);

            MemorySegment nativeString = outString.get(ValueLayout.ADDRESS, 0);
            try {
                return readNativeString(nativeString);
            } finally {
                stringFree(nativeString);
            }
        } catch (CopperlaceException exception) {
            throw exception;
        } catch (Throwable throwable) {
            throw new CopperlaceException("Failed to render Copperlace rule", throwable);
        }
    }

    void rulesetFree(MemorySegment handle) {
        try {
            rulesetFree.invokeExact(handle);
        } catch (Throwable throwable) {
            throw new CopperlaceException("Failed to free Copperlace ruleset", throwable);
        }
    }

    private void stringFree(MemorySegment nativeString) {
        if (!isNull(nativeString)) {
            try {
                stringFree.invokeExact(nativeString);
            } catch (Throwable throwable) {
                throw new CopperlaceException("Failed to free Copperlace string", throwable);
            }
        }
    }

    private void checkStatus(int status, MemorySegment outError) {
        if (status == COPPERLACE_OK) {
            return;
        }

        MemorySegment nativeError = outError.get(ValueLayout.ADDRESS, 0);
        String message = switch (status) {
            case COPPERLACE_PARSE_ERROR -> "Copperlace parse error";
            case COPPERLACE_RENDER_ERROR -> "Copperlace render error";
            default -> "Copperlace native call failed";
        };
        if (!isNull(nativeError)) {
            try {
                message = readNativeString(nativeError);
            } finally {
                stringFree(nativeError);
            }
        }
        throw new CopperlaceException(message);
    }

    private String readNativeString(MemorySegment nativeString) {
        if (isNull(nativeString)) {
            return "";
        }
        return nativeString.reinterpret(Long.MAX_VALUE).getString(0);
    }

    private static MethodHandle downcall(
            Linker linker,
            SymbolLookup lookup,
            String symbol,
            FunctionDescriptor descriptor) {
        Optional<MemorySegment> address = lookup.find(symbol);
        if (address.isEmpty()) {
            throw new CopperlaceException("Could not find native symbol: " + symbol);
        }
        return linker.downcallHandle(address.get(), descriptor);
    }

    private static boolean isNull(MemorySegment segment) {
        return segment == null || segment.equals(MemorySegment.NULL) || segment.address() == 0;
    }

    private static Path findLibrary() {
        String override = System.getenv("COPPERLACE_LIBRARY_PATH");
        if (override != null && !override.isBlank()) {
            Path path = Path.of(override);
            if (Files.exists(path)) {
                return path;
            }
        }

        String libraryName = nativeLibraryName();
        Optional<Path> packagedLibrary = findPackagedLibrary(libraryName);
        if (packagedLibrary.isPresent()) {
            return packagedLibrary.get();
        }

        for (Path candidate : sourceTreeCandidates(libraryName)) {
            if (Files.exists(candidate)) {
                return candidate;
            }
        }

        throw new CopperlaceException(
                "Could not find "
                        + libraryName
                        + " for "
                        + nativeClassifier()
                        + ". Add the matching native classifier artifact, build rust-core, or set COPPERLACE_LIBRARY_PATH.");
    }

    private static Optional<Path> findPackagedLibrary(String libraryName) {
        String resourcePath = packagedResourcePath(nativeClassifier(), libraryName);
        try (InputStream input = NativeLibrary.class.getResourceAsStream("/" + resourcePath)) {
            if (input == null) {
                return Optional.empty();
            }

            Path extracted = Files.createTempFile("copperlace-", "-" + libraryName);
            Files.copy(input, extracted, StandardCopyOption.REPLACE_EXISTING);
            extracted.toFile().deleteOnExit();
            return Optional.of(extracted);
        } catch (IOException exception) {
            throw new CopperlaceException("Failed to extract packaged Copperlace native library", exception);
        }
    }

    static String packagedResourcePath(String classifier, String libraryName) {
        return "dev/mahe/copperlace/native/" + classifier + "/" + libraryName;
    }

    private static Path[] sourceTreeCandidates(String libraryName) {
        Path cwd = Path.of("").toAbsolutePath().normalize();
        return new Path[] {
            cwd.resolve("../rust-core/target/release").resolve(libraryName).normalize(),
            cwd.resolve("rust-core/target/release").resolve(libraryName).normalize()
        };
    }

    static String nativeClassifier() {
        String os = System.getProperty("os.name").toLowerCase(Locale.ROOT);
        String arch = normalizeArch(System.getProperty("os.arch"));
        if (os.contains("win")) {
            return "windows-" + arch;
        }
        if (os.contains("mac") || os.contains("darwin")) {
            return "macos-" + arch;
        }
        if (os.contains("linux")) {
            return "linux-" + arch;
        }
        throw new CopperlaceException("Unsupported native OS: " + System.getProperty("os.name"));
    }

    static String nativeLibraryName() {
        String os = System.getProperty("os.name").toLowerCase(Locale.ROOT);
        if (os.contains("win")) {
            return "copperlace.dll";
        }
        if (os.contains("mac") || os.contains("darwin")) {
            return "libcopperlace.dylib";
        }
        if (os.contains("linux")) {
            return "libcopperlace.so";
        }
        throw new CopperlaceException("Unsupported native OS: " + System.getProperty("os.name"));
    }

    private static String normalizeArch(String rawArch) {
        String arch = rawArch.toLowerCase(Locale.ROOT);
        if (arch.equals("amd64") || arch.equals("x86_64")) {
            return "x86_64";
        }
        if (arch.equals("aarch64") || arch.equals("arm64")) {
            return "aarch64";
        }
        throw new CopperlaceException("Unsupported native architecture: " + rawArch);
    }
}
