package dev.mahe.copperlace;

import java.lang.foreign.MemorySegment;
import java.nio.file.Path;
import java.util.Objects;

import org.apache.commons.lang3.Validate;

/**
 * Compiled Copperlace rules loaded from HOCON config.
 *
 * <p>{@code RuleSet} is the lower-level load-once API. It owns a native
 * Copperlace ruleset handle and can render named rules repeatedly until
 * {@link #close()} is called.
 *
 * <p>Most callers should use {@link Copperlace}; use {@code RuleSet} directly
 * when the lower-level type better matches the surrounding API.
 */
public final class RuleSet implements AutoCloseable {
    private MemorySegment handle;
    private boolean closed;

    private RuleSet(final MemorySegment handle) {
        Validate.isTrue(!NativeLibrary.isNull(handle), "handle must not be null");
        this.handle = handle;
    }

    /**
     * Compiles a HOCON config string into a reusable ruleset.
     *
     * @param config HOCON config text containing Copperlace rules
     * @return a ruleset backed by a native Copperlace handle
     * @throws IllegalArgumentException if {@code config} is blank
     * @throws CopperlaceException if the config cannot be parsed or compiled
     */
    public static RuleSet fromString(final String config) {
        Validate.notBlank(config, "config must not be blank");
        return new RuleSet(NativeLibrary.INSTANCE.rulesetFromString(config));
    }

    /**
     * Loads and compiles a HOCON config file into a reusable ruleset.
     *
     * @param path path to the HOCON config file
     * @return a ruleset backed by a native Copperlace handle
     * @throws NullPointerException if {@code path} is null
     * @throws CopperlaceException if the file cannot be loaded, parsed, or compiled
     */
    public static RuleSet fromFile(final Path path) {
        Objects.requireNonNull(path, "path");
        return new RuleSet(NativeLibrary.INSTANCE.rulesetFromFile(path));
    }

    /**
     * Renders a named rule from this ruleset.
     *
     * <p>Each call uses a fresh render context, so per-render bindings are
     * consistent within one output but do not carry over to later renders.
     *
     * @param rule name of the rule to render
     * @return rendered text for {@code rule}
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if this ruleset is closed or rendering fails
     */
    public String render(final String rule) {
        ensureOpen();
        Validate.notBlank(rule, "rule must not be blank");
        return NativeLibrary.INSTANCE.render(handle, rule);
    }

    /**
     * Releases this ruleset's native handle.
     *
     * <p>Calling {@code close} more than once is allowed.
     */
    @Override
    public void close() {
        if (!closed) {
            NativeLibrary.INSTANCE.rulesetFree(handle);
            handle = MemorySegment.NULL;
            closed = true;
        }
    }

    private void ensureOpen() {
        if (closed) {
            throw new CopperlaceException("RuleSet is closed");
        }
    }
}
