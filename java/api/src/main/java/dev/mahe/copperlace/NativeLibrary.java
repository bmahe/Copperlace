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
import java.lang.invoke.MethodHandles;
import java.lang.invoke.MethodType;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;

import org.apache.commons.lang3.Validate;

final class NativeLibrary {
    static final NativeLibrary INSTANCE = new NativeLibrary();

    private static final int COPPERLACE_OK = 0;
    private static final int COPPERLACE_PARSE_ERROR = 2;
    private static final int COPPERLACE_RENDER_ERROR = 3;

    private final Arena libraryArena = Arena.ofShared();
    private final MethodHandle rulesetFromFile;
    private final MethodHandle rulesetFromString;
    private final MethodHandle rulesetFromFileWithProcessors;
    private final MethodHandle rulesetFromStringWithProcessors;
    private final MethodHandle rulesetRender;
    private final MethodHandle rulesetRenderWithContext;
    private final MethodHandle rulesetRenderStructuredJson;
    private final MethodHandle rulesetRenderStructuredJsonWithContext;
    private final MethodHandle rulesetFree;
    private final MethodHandle stringFree;
    private final MethodHandle processorResultSetOutput;
    private final MethodHandle processorResultSetError;
    private final MethodHandle processorUpcall;

    private NativeLibrary() {
        final Linker linker = Linker.nativeLinker();
        final SymbolLookup lookup = SymbolLookup.libraryLookup(findLibrary(), libraryArena);
        processorUpcall = processorUpcallHandle();
        rulesetFromFile = downcall(
                linker,
                lookup,
                "copperlace_ruleset_from_file",
                FunctionDescriptor.of(
                        ValueLayout.JAVA_INT,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS));
        rulesetFromFileWithProcessors = downcall(
                linker,
                lookup,
                "copperlace_ruleset_from_file_with_processors",
                FunctionDescriptor.of(
                        ValueLayout.JAVA_INT,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS,
                        ValueLayout.JAVA_LONG,
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
        rulesetFromStringWithProcessors = downcall(
                linker,
                lookup,
                "copperlace_ruleset_from_string_with_processors",
                FunctionDescriptor.of(
                        ValueLayout.JAVA_INT,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS,
                        ValueLayout.JAVA_LONG,
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
        rulesetRenderWithContext = downcall(
                linker,
                lookup,
                "copperlace_ruleset_render_with_context",
                FunctionDescriptor.of(
                        ValueLayout.JAVA_INT,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS,
                        ValueLayout.JAVA_LONG,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS));
        rulesetRenderStructuredJson = downcall(
                linker,
                lookup,
                "copperlace_ruleset_render_structured_json",
                FunctionDescriptor.of(
                        ValueLayout.JAVA_INT,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS,
                        ValueLayout.JAVA_BOOLEAN,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS));
        rulesetRenderStructuredJsonWithContext = downcall(
                linker,
                lookup,
                "copperlace_ruleset_render_structured_json_with_context",
                FunctionDescriptor.of(
                        ValueLayout.JAVA_INT,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS,
                        ValueLayout.JAVA_LONG,
                        ValueLayout.JAVA_BOOLEAN,
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
        processorResultSetOutput = downcall(
                linker,
                lookup,
                "copperlace_processor_result_set_output",
                FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS));
        processorResultSetError = downcall(
                linker,
                lookup,
                "copperlace_processor_result_set_error",
                FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS));
    }

    MemorySegment rulesetFromString(final String config) {
        Validate.notBlank(config, "config must not be blank");

        try (Arena arena = Arena.ofConfined()) {
            final MemorySegment outHandle = arena.allocate(ValueLayout.ADDRESS);
            final MemorySegment outError = arena.allocate(ValueLayout.ADDRESS);
            final MemorySegment configString = arena.allocateFrom(config);

            final int status = (int) rulesetFromString.invokeExact(configString, outHandle, outError);
            checkStatus(status, outError);
            return outHandle.get(ValueLayout.ADDRESS, 0);
        } catch (final CopperlaceException exception) {
            throw exception;
        } catch (final Throwable throwable) {
            throw new CopperlaceException("Failed to create Copperlace ruleset", throwable);
        }
    }

    RulesetHandle rulesetFromStringWithProcessors(
            final String config, final Map<String, CopperlaceProcessor> processors) {
        Validate.notBlank(config, "config must not be blank");
        Objects.requireNonNull(processors, "processors");

        final NativeProcessorRegistry registry = new NativeProcessorRegistry(processors);
        try (Arena arena = Arena.ofConfined()) {
            final MemorySegment outHandle = arena.allocate(ValueLayout.ADDRESS);
            final MemorySegment outError = arena.allocate(ValueLayout.ADDRESS);
            final MemorySegment configString = arena.allocateFrom(config);

            final int status = (int) rulesetFromStringWithProcessors.invokeExact(
                    configString,
                    registry.names,
                    registry.callbacks,
                    registry.userData,
                    registry.length,
                    outHandle,
                    outError);
            checkStatus(status, outError);
            return new RulesetHandle(outHandle.get(ValueLayout.ADDRESS, 0), registry);
        } catch (final CopperlaceException exception) {
            registry.close();
            throw exception;
        } catch (final Throwable throwable) {
            registry.close();
            throw new CopperlaceException("Failed to create Copperlace ruleset", throwable);
        }
    }

    String renderWithContext(final MemorySegment handle, final String rule, final Map<String, String> context) {
        Validate.isTrue(!isNull(handle), "handle must not be null");
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(context, "context");

        try (Arena arena = Arena.ofConfined()) {
            final MemorySegment outString = arena.allocate(ValueLayout.ADDRESS);
            final MemorySegment outError = arena.allocate(ValueLayout.ADDRESS);
            final MemorySegment ruleString = arena.allocateFrom(rule);
            final long contextLength = context.size();
            final long contextBytes = ValueLayout.ADDRESS.byteSize() * contextLength;
            final MemorySegment contextKeys = contextLength == 0
                    ? MemorySegment.NULL
                    : arena.allocate(contextBytes, ValueLayout.ADDRESS.byteAlignment());
            final MemorySegment contextValues = contextLength == 0
                    ? MemorySegment.NULL
                    : arena.allocate(contextBytes, ValueLayout.ADDRESS.byteAlignment());

            long index = 0;
            for (final Map.Entry<String, String> entry : context.entrySet()) {
                final String key = Objects.requireNonNull(entry.getKey(), "context key");
                final String value = Objects.requireNonNull(entry.getValue(), "context value");
                contextKeys.setAtIndex(ValueLayout.ADDRESS, index, arena.allocateFrom(key));
                contextValues.setAtIndex(ValueLayout.ADDRESS, index, arena.allocateFrom(value));
                index++;
            }

            final int status = (int) rulesetRenderWithContext.invokeExact(
                    handle,
                    ruleString,
                    contextKeys,
                    contextValues,
                    contextLength,
                    outString,
                    outError);
            checkStatus(status, outError);

            final MemorySegment nativeString = outString.get(ValueLayout.ADDRESS, 0);
            try {
                return readNativeString(nativeString);
            } finally {
                stringFree(nativeString);
            }
        } catch (final CopperlaceException exception) {
            throw exception;
        } catch (final Throwable throwable) {
            throw new CopperlaceException("Failed to render Copperlace rule", throwable);
        }
    }

    String renderStructuredJson(final MemorySegment handle, final String rule, final boolean formatJson) {
        Validate.isTrue(!isNull(handle), "handle must not be null");
        Validate.notBlank(rule, "rule must not be blank");

        try (Arena arena = Arena.ofConfined()) {
            final MemorySegment outJson = arena.allocate(ValueLayout.ADDRESS);
            final MemorySegment outError = arena.allocate(ValueLayout.ADDRESS);
            final MemorySegment ruleString = arena.allocateFrom(rule);

            final int status = (int) rulesetRenderStructuredJson.invokeExact(
                    handle, ruleString, formatJson, outJson, outError);
            checkStatus(status, outError);

            final MemorySegment nativeString = outJson.get(ValueLayout.ADDRESS, 0);
            try {
                return readNativeString(nativeString);
            } finally {
                stringFree(nativeString);
            }
        } catch (final CopperlaceException exception) {
            throw exception;
        } catch (final Throwable throwable) {
            throw new CopperlaceException("Failed to render Copperlace structured rule", throwable);
        }
    }

    String renderStructuredJsonWithContext(
            final MemorySegment handle,
            final String rule,
            final Map<String, String> context,
            final boolean formatJson) {
        Validate.isTrue(!isNull(handle), "handle must not be null");
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(context, "context");

        try (Arena arena = Arena.ofConfined()) {
            final MemorySegment outJson = arena.allocate(ValueLayout.ADDRESS);
            final MemorySegment outError = arena.allocate(ValueLayout.ADDRESS);
            final MemorySegment ruleString = arena.allocateFrom(rule);
            final long contextLength = context.size();
            final long contextBytes = ValueLayout.ADDRESS.byteSize() * contextLength;
            final MemorySegment contextKeys = contextLength == 0
                    ? MemorySegment.NULL
                    : arena.allocate(contextBytes, ValueLayout.ADDRESS.byteAlignment());
            final MemorySegment contextValues = contextLength == 0
                    ? MemorySegment.NULL
                    : arena.allocate(contextBytes, ValueLayout.ADDRESS.byteAlignment());

            long index = 0;
            for (final Map.Entry<String, String> entry : context.entrySet()) {
                final String key = Objects.requireNonNull(entry.getKey(), "context key");
                final String value = Objects.requireNonNull(entry.getValue(), "context value");
                contextKeys.setAtIndex(ValueLayout.ADDRESS, index, arena.allocateFrom(key));
                contextValues.setAtIndex(ValueLayout.ADDRESS, index, arena.allocateFrom(value));
                index++;
            }

            final int status = (int) rulesetRenderStructuredJsonWithContext.invokeExact(
                    handle,
                    ruleString,
                    contextKeys,
                    contextValues,
                    contextLength,
                    formatJson,
                    outJson,
                    outError);
            checkStatus(status, outError);

            final MemorySegment nativeString = outJson.get(ValueLayout.ADDRESS, 0);
            try {
                return readNativeString(nativeString);
            } finally {
                stringFree(nativeString);
            }
        } catch (final CopperlaceException exception) {
            throw exception;
        } catch (final Throwable throwable) {
            throw new CopperlaceException("Failed to render Copperlace structured rule", throwable);
        }
    }

    MemorySegment rulesetFromFile(final Path path) {
        Objects.requireNonNull(path, "path");

        try (Arena arena = Arena.ofConfined()) {
            final MemorySegment outHandle = arena.allocate(ValueLayout.ADDRESS);
            final MemorySegment outError = arena.allocate(ValueLayout.ADDRESS);
            final MemorySegment pathString = arena.allocateFrom(path.toString());

            final int status = (int) rulesetFromFile.invokeExact(pathString, outHandle, outError);
            checkStatus(status, outError);
            return outHandle.get(ValueLayout.ADDRESS, 0);
        } catch (final CopperlaceException exception) {
            throw exception;
        } catch (final Throwable throwable) {
            throw new CopperlaceException("Failed to create Copperlace ruleset", throwable);
        }
    }

    RulesetHandle rulesetFromFileWithProcessors(
            final Path path, final Map<String, CopperlaceProcessor> processors) {
        Objects.requireNonNull(path, "path");
        Objects.requireNonNull(processors, "processors");

        final NativeProcessorRegistry registry = new NativeProcessorRegistry(processors);
        try (Arena arena = Arena.ofConfined()) {
            final MemorySegment outHandle = arena.allocate(ValueLayout.ADDRESS);
            final MemorySegment outError = arena.allocate(ValueLayout.ADDRESS);
            final MemorySegment pathString = arena.allocateFrom(path.toString());

            final int status = (int) rulesetFromFileWithProcessors.invokeExact(
                    pathString,
                    registry.names,
                    registry.callbacks,
                    registry.userData,
                    registry.length,
                    outHandle,
                    outError);
            checkStatus(status, outError);
            return new RulesetHandle(outHandle.get(ValueLayout.ADDRESS, 0), registry);
        } catch (final CopperlaceException exception) {
            registry.close();
            throw exception;
        } catch (final Throwable throwable) {
            registry.close();
            throw new CopperlaceException("Failed to create Copperlace ruleset", throwable);
        }
    }

    String render(final MemorySegment handle, final String rule) {
        Validate.isTrue(!isNull(handle), "handle must not be null");
        Validate.notBlank(rule, "rule must not be blank");

        try (Arena arena = Arena.ofConfined()) {
            final MemorySegment outString = arena.allocate(ValueLayout.ADDRESS);
            final MemorySegment outError = arena.allocate(ValueLayout.ADDRESS);
            final MemorySegment ruleString = arena.allocateFrom(rule);

            final int status = (int) rulesetRender.invokeExact(handle, ruleString, outString, outError);
            checkStatus(status, outError);

            final MemorySegment nativeString = outString.get(ValueLayout.ADDRESS, 0);
            try {
                return readNativeString(nativeString);
            } finally {
                stringFree(nativeString);
            }
        } catch (final CopperlaceException exception) {
            throw exception;
        } catch (final Throwable throwable) {
            throw new CopperlaceException("Failed to render Copperlace rule", throwable);
        }
    }

    void rulesetFree(final MemorySegment handle) {
        Validate.isTrue(!isNull(handle), "handle must not be null");

        try {
            rulesetFree.invokeExact(handle);
        } catch (final Throwable throwable) {
            throw new CopperlaceException("Failed to free Copperlace ruleset", throwable);
        }
    }

    private void stringFree(final MemorySegment nativeString) {
        if (!isNull(nativeString)) {
            try {
                stringFree.invokeExact(nativeString);
            } catch (final Throwable throwable) {
                throw new CopperlaceException("Failed to free Copperlace string", throwable);
            }
        }
    }

    private int processorResultSetOutput(final MemorySegment result, final String output) throws Throwable {
        try (Arena arena = Arena.ofConfined()) {
            return (int) processorResultSetOutput.invokeExact(result, arena.allocateFrom(output));
        }
    }

    private int processorResultSetError(final MemorySegment result, final String message) throws Throwable {
        try (Arena arena = Arena.ofConfined()) {
            return (int) processorResultSetError.invokeExact(result, arena.allocateFrom(message));
        }
    }

    private void checkStatus(final int status, final MemorySegment outError) {
        Objects.requireNonNull(outError, "outError");

        if (status == COPPERLACE_OK) {
            return;
        }

        final MemorySegment nativeError = outError.get(ValueLayout.ADDRESS, 0);
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

    private String readNativeString(final MemorySegment nativeString) {
        if (isNull(nativeString)) {
            return "";
        }
        return nativeString.reinterpret(Long.MAX_VALUE).getString(0);
    }

    private static MethodHandle downcall(
            final Linker linker,
            final SymbolLookup lookup,
            final String symbol,
            final FunctionDescriptor descriptor) {
        Objects.requireNonNull(linker, "linker");
        Objects.requireNonNull(lookup, "lookup");
        Validate.notBlank(symbol, "symbol must not be blank");
        Objects.requireNonNull(descriptor, "descriptor");

        final Optional<MemorySegment> address = lookup.find(symbol);
        if (address.isEmpty()) {
            throw new CopperlaceException("Could not find native symbol: " + symbol);
        }
        return linker.downcallHandle(address.get(), descriptor);
    }

    private static MethodHandle processorUpcallHandle() {
        try {
            return MethodHandles.lookup()
                    .findStatic(
                            NativeLibrary.class,
                            "invokeProcessor",
                            MethodType.methodType(
                                    int.class,
                                    ProcessorState.class,
                                    MemorySegment.class,
                                    MemorySegment.class,
                                    MemorySegment.class));
        } catch (final NoSuchMethodException | IllegalAccessException exception) {
            throw new CopperlaceException("Failed to initialize processor upcall", exception);
        }
    }

    @SuppressWarnings("unused")
    private static int invokeProcessor(
            final ProcessorState state,
            final MemorySegment input,
            final MemorySegment result,
            final MemorySegment userData) {
        Objects.requireNonNull(state, "state");
        try {
            final String output = state.processor().process(state.library().readNativeString(input));
            if (output == null) {
                return state.library().processorResultSetError(result, "processor returned null");
            }
            return state.library().processorResultSetOutput(result, output);
        } catch (final Throwable throwable) {
            final String message = throwable.getMessage() == null ? throwable.toString() : throwable.getMessage();
            try {
                return state.library().processorResultSetError(result, message);
            } catch (final Throwable nested) {
                return COPPERLACE_RENDER_ERROR;
            }
        }
    }

    static boolean isNull(final MemorySegment segment) {
        return segment == null || segment.equals(MemorySegment.NULL) || segment.address() == 0;
    }

    private static Path findLibrary() {
        final String override = System.getenv("COPPERLACE_LIBRARY_PATH");
        if (override != null && !override.isBlank()) {
            final Path path = Path.of(override);
            if (Files.exists(path)) {
                return path;
            }
        }

        final String libraryName = nativeLibraryName();
        final Optional<Path> packagedLibrary = findPackagedLibrary(libraryName);
        if (packagedLibrary.isPresent()) {
            return packagedLibrary.get();
        }

        for (final Path candidate : sourceTreeCandidates(libraryName)) {
            if (Files.exists(candidate)) {
                return candidate;
            }
        }

        throw new CopperlaceException(
                "Could not find "
                        + libraryName
                        + " for "
                        + nativeClassifier()
                        + ". Add the matching native platform artifact, build rust-core, or set COPPERLACE_LIBRARY_PATH.");
    }

    private static Optional<Path> findPackagedLibrary(final String libraryName) {
        Validate.notBlank(libraryName, "libraryName must not be blank");

        final String resourcePath = packagedResourcePath(nativeClassifier(), libraryName);
        try (InputStream input = NativeLibrary.class.getResourceAsStream("/" + resourcePath)) {
            if (input == null) {
                return Optional.empty();
            }

            final Path extracted = Files.createTempFile("copperlace-", "-" + libraryName);
            Files.copy(input, extracted, StandardCopyOption.REPLACE_EXISTING);
            extracted.toFile().deleteOnExit();
            return Optional.of(extracted);
        } catch (final IOException exception) {
            throw new CopperlaceException("Failed to extract packaged Copperlace native library", exception);
        }
    }

    static String packagedResourcePath(final String classifier, final String libraryName) {
        Validate.notBlank(classifier, "classifier must not be blank");
        Validate.notBlank(libraryName, "libraryName must not be blank");

        return "dev/mahe/copperlace/native/" + classifier + "/" + libraryName;
    }

    private static Path[] sourceTreeCandidates(final String libraryName) {
        Validate.notBlank(libraryName, "libraryName must not be blank");

        final Path cwd = Path.of("").toAbsolutePath().normalize();
        return new Path[] {
            cwd.resolve("../rust-core/target/release").resolve(libraryName).normalize(),
            cwd.resolve("../../rust-core/target/release").resolve(libraryName).normalize(),
            cwd.resolve("rust-core/target/release").resolve(libraryName).normalize()
        };
    }

    static String nativeClassifier() {
        final String os = System.getProperty("os.name").toLowerCase(Locale.ROOT);
        final String arch = normalizeArch(System.getProperty("os.arch"));
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
        final String os = System.getProperty("os.name").toLowerCase(Locale.ROOT);
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

    private static String normalizeArch(final String rawArch) {
        Validate.notBlank(rawArch, "rawArch must not be blank");

        final String arch = rawArch.toLowerCase(Locale.ROOT);
        if (arch.equals("amd64") || arch.equals("x86_64")) {
            return "x86_64";
        }
        if (arch.equals("aarch64") || arch.equals("arm64")) {
            return "aarch64";
        }
        throw new CopperlaceException("Unsupported native architecture: " + rawArch);
    }

    static final class RulesetHandle {
        private final MemorySegment handle;
        private final NativeProcessorRegistry processors;

        private RulesetHandle(final MemorySegment handle, final NativeProcessorRegistry processors) {
            Validate.isTrue(!isNull(handle), "handle must not be null");
            this.handle = handle;
            this.processors = processors;
        }

        MemorySegment handle() {
            return handle;
        }

        void closeProcessors() {
            if (processors != null) {
                processors.close();
            }
        }
    }

    private record ProcessorState(NativeLibrary library, CopperlaceProcessor processor) {}

    private final class NativeProcessorRegistry implements AutoCloseable {
        private final Arena arena = Arena.ofConfined();
        private final List<ProcessorState> states = new ArrayList<>();
        private final long length;
        private final MemorySegment names;
        private final MemorySegment callbacks;
        private final MemorySegment userData;

        private NativeProcessorRegistry(final Map<String, CopperlaceProcessor> processors) {
            length = processors.size();
            final long bytes = ValueLayout.ADDRESS.byteSize() * length;
            names = length == 0 ? MemorySegment.NULL : arena.allocate(bytes, ValueLayout.ADDRESS.byteAlignment());
            callbacks = length == 0 ? MemorySegment.NULL : arena.allocate(bytes, ValueLayout.ADDRESS.byteAlignment());
            userData = length == 0 ? MemorySegment.NULL : arena.allocate(bytes, ValueLayout.ADDRESS.byteAlignment());

            long index = 0;
            final FunctionDescriptor descriptor =
                    FunctionDescriptor.of(
                            ValueLayout.JAVA_INT,
                            ValueLayout.ADDRESS,
                            ValueLayout.ADDRESS,
                            ValueLayout.ADDRESS);
            for (final Map.Entry<String, CopperlaceProcessor> entry : processors.entrySet()) {
                final String name = Objects.requireNonNull(entry.getKey(), "processor name");
                final CopperlaceProcessor processor = Objects.requireNonNull(entry.getValue(), "processor");
                final ProcessorState state = new ProcessorState(NativeLibrary.this, processor);
                states.add(state);
                final MethodHandle boundProcessor = processorUpcall.bindTo(state);
                final MemorySegment callback = Linker.nativeLinker().upcallStub(boundProcessor, descriptor, arena);
                names.setAtIndex(ValueLayout.ADDRESS, index, arena.allocateFrom(name));
                callbacks.setAtIndex(ValueLayout.ADDRESS, index, callback);
                userData.setAtIndex(ValueLayout.ADDRESS, index, MemorySegment.NULL);
                index++;
            }
        }

        @Override
        public void close() {
            arena.close();
        }
    }
}
