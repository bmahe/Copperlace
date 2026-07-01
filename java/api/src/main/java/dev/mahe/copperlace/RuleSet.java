package dev.mahe.copperlace;

import java.lang.foreign.MemorySegment;
import java.nio.file.Path;
import java.util.Map;
import java.util.Objects;

import org.apache.commons.lang3.Validate;

/**
 * Compiled Copperlace rules loaded from configuration.
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
    private final NativeLibrary.RulesetHandle ownedHandle;
    private boolean closed;

    private RuleSet(final MemorySegment handle) {
        Validate.isTrue(!NativeLibrary.isNull(handle), "handle must not be null");
        this.handle = handle;
        ownedHandle = null;
    }

    private RuleSet(final NativeLibrary.RulesetHandle ownedHandle) {
        this.ownedHandle = Validate.notNull(ownedHandle, "ownedHandle must not be null");
        handle = ownedHandle.handle();
    }

    /**
     * Compiles a configuration string into a reusable ruleset.
     *
     * @param config configuration text containing Copperlace rules
     * @return a ruleset backed by a native Copperlace handle
     * @throws IllegalArgumentException if {@code config} is blank
     * @throws CopperlaceException if the config cannot be parsed or compiled
     */
    public static RuleSet fromString(final String config) {
        Validate.notBlank(config, "config must not be blank");
        return new RuleSet(NativeLibrary.INSTANCE.rulesetFromString(config));
    }

    /**
     * Compiles a configuration string into a reusable ruleset with custom processors.
     *
     * @param config configuration text containing Copperlace rules
     * @param processors custom processor callbacks keyed by processor name
     * @return a ruleset backed by a native Copperlace handle
     * @throws NullPointerException if {@code processors}, a processor name, or a processor is null
     * @throws IllegalArgumentException if {@code config} is blank
     * @throws CopperlaceException if the config cannot be parsed or compiled
     */
    public static RuleSet fromStringWithProcessors(
            final String config, final Map<String, CopperlaceProcessor> processors) {
        Validate.notBlank(config, "config must not be blank");
        validateProcessors(processors);
        return new RuleSet(NativeLibrary.INSTANCE.rulesetFromStringWithProcessors(config, processors));
    }

    /**
     * Loads and compiles a configuration file into a reusable ruleset.
     *
     * @param path path to the configuration file
     * @return a ruleset backed by a native Copperlace handle
     * @throws NullPointerException if {@code path} is null
     * @throws CopperlaceException if the file cannot be loaded, parsed, or compiled
     */
    public static RuleSet fromFile(final Path path) {
        Objects.requireNonNull(path, "path");
        return new RuleSet(NativeLibrary.INSTANCE.rulesetFromFile(path));
    }

    /**
     * Loads and compiles a configuration file into a reusable ruleset with custom processors.
     *
     * @param path path to the configuration file
     * @param processors custom processor callbacks keyed by processor name
     * @return a ruleset backed by a native Copperlace handle
     * @throws NullPointerException if {@code path}, {@code processors}, a processor name, or a processor is null
     * @throws CopperlaceException if the file cannot be loaded, parsed, or compiled
     */
    public static RuleSet fromFileWithProcessors(
            final Path path, final Map<String, CopperlaceProcessor> processors) {
        Objects.requireNonNull(path, "path");
        validateProcessors(processors);
        return new RuleSet(NativeLibrary.INSTANCE.rulesetFromFileWithProcessors(path, processors));
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
     * Renders a named rule from this ruleset with limited recursion enabled.
     *
     * @param rule name of the rule to render
     * @param maxRecursionDepth recursive re-entries allowed per rule
     * @return rendered text for {@code rule}
     * @throws IllegalArgumentException if {@code rule} is blank or {@code maxRecursionDepth} is negative
     * @throws CopperlaceException if this ruleset is closed or rendering fails
     */
    public String render(final String rule, final int maxRecursionDepth) {
        ensureOpen();
        Validate.notBlank(rule, "rule must not be blank");
        Validate.isTrue(maxRecursionDepth >= 0, "maxRecursionDepth must be non-negative");
        return NativeLibrary.INSTANCE.renderWithContext(handle, rule, Map.of(), maxRecursionDepth);
    }

    /**
     * Renders a named rule from this ruleset with initial context values.
     *
     * <p>The provided context is scoped to this render only. Values resolve
     * before config-defined {@code context} defaults and named rules.
     *
     * @param rule name of the rule to render
     * @param context initial render context values
     * @return rendered text for {@code rule}
     * @throws NullPointerException if {@code context}, a context key, or a context value is null
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if this ruleset is closed or rendering fails
     */
    public String render(final String rule, final Map<String, String> context) {
        ensureOpen();
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(context, "context");
        validateContext(context);
        return NativeLibrary.INSTANCE.renderWithContext(handle, rule, context);
    }

    /**
     * Renders a named rule from this ruleset with initial context values and limited recursion enabled.
     *
     * @param rule name of the rule to render
     * @param context initial render context values
     * @param maxRecursionDepth recursive re-entries allowed per rule
     * @return rendered text for {@code rule}
     * @throws NullPointerException if {@code context}, a context key, or a context value is null
     * @throws IllegalArgumentException if {@code rule} is blank or {@code maxRecursionDepth} is negative
     * @throws CopperlaceException if this ruleset is closed or rendering fails
     */
    public String render(final String rule, final Map<String, String> context, final int maxRecursionDepth) {
        ensureOpen();
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(context, "context");
        Validate.isTrue(maxRecursionDepth >= 0, "maxRecursionDepth must be non-negative");
        validateContext(context);
        return NativeLibrary.INSTANCE.renderWithContext(handle, rule, context, maxRecursionDepth);
    }

    /**
     * Renders a named rule as text, returning formatted JSON for object-valued rules.
     *
     * @param rule name of the rule to render
     * @return rendered text, or formatted JSON for an object-valued rule
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if this ruleset is closed or rendering fails
     */
    public String renderInferred(final String rule) {
        ensureOpen();
        Validate.notBlank(rule, "rule must not be blank");
        return NativeLibrary.INSTANCE.renderInferred(handle, rule);
    }

    /**
     * Renders a named rule as text with limited recursion enabled, returning formatted JSON for object-valued rules.
     *
     * @param rule name of the rule to render
     * @param maxRecursionDepth recursive re-entries allowed per rule
     * @return rendered text, or formatted JSON for an object-valued rule
     * @throws IllegalArgumentException if {@code rule} is blank or {@code maxRecursionDepth} is negative
     * @throws CopperlaceException if this ruleset is closed or rendering fails
     */
    public String renderInferred(final String rule, final int maxRecursionDepth) {
        ensureOpen();
        Validate.notBlank(rule, "rule must not be blank");
        Validate.isTrue(maxRecursionDepth >= 0, "maxRecursionDepth must be non-negative");
        return NativeLibrary.INSTANCE.renderInferredWithContext(handle, rule, Map.of(), maxRecursionDepth);
    }

    /**
     * Renders a named rule as text with initial context, returning formatted JSON for object-valued rules.
     *
     * @param rule name of the rule to render
     * @param context initial render context values
     * @return rendered text, or formatted JSON for an object-valued rule
     * @throws NullPointerException if {@code context}, a context key, or a context value is null
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if this ruleset is closed or rendering fails
     */
    public String renderInferred(final String rule, final Map<String, String> context) {
        ensureOpen();
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(context, "context");
        validateContext(context);
        return NativeLibrary.INSTANCE.renderInferredWithContext(handle, rule, context);
    }

    /**
     * Renders a named rule as text with initial context and limited recursion enabled.
     *
     * @param rule name of the rule to render
     * @param context initial render context values
     * @param maxRecursionDepth recursive re-entries allowed per rule
     * @return rendered text, or formatted JSON for an object-valued rule
     * @throws NullPointerException if {@code context}, a context key, or a context value is null
     * @throws IllegalArgumentException if {@code rule} is blank or {@code maxRecursionDepth} is negative
     * @throws CopperlaceException if this ruleset is closed or rendering fails
     */
    public String renderInferred(final String rule, final Map<String, String> context, final int maxRecursionDepth) {
        ensureOpen();
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(context, "context");
        Validate.isTrue(maxRecursionDepth >= 0, "maxRecursionDepth must be non-negative");
        validateContext(context);
        return NativeLibrary.INSTANCE.renderInferredWithContext(handle, rule, context, maxRecursionDepth);
    }

    /**
     * Renders a named structured rule from this ruleset as formatted JSON text.
     *
     * @param rule name of the structured rule to render
     * @return formatted JSON for {@code rule}
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if this ruleset is closed or rendering fails
     */
    public String renderStructuredJson(final String rule) {
        return renderStructuredJson(rule, true);
    }

    /**
     * Renders a named structured rule from this ruleset as JSON text.
     *
     * @param rule name of the structured rule to render
     * @param formatJson true to format JSON with tabs, false for compact JSON
     * @return JSON for {@code rule}
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if this ruleset is closed or rendering fails
     */
    public String renderStructuredJson(final String rule, final boolean formatJson) {
        ensureOpen();
        Validate.notBlank(rule, "rule must not be blank");
        return NativeLibrary.INSTANCE.renderStructuredJson(handle, rule, formatJson);
    }

    /**
     * Renders a named structured rule from this ruleset as JSON text with limited recursion enabled.
     *
     * @param rule name of the structured rule to render
     * @param formatJson true to format JSON with tabs, false for compact JSON
     * @param maxRecursionDepth recursive re-entries allowed per rule
     * @return JSON for {@code rule}
     * @throws IllegalArgumentException if {@code rule} is blank or {@code maxRecursionDepth} is negative
     * @throws CopperlaceException if this ruleset is closed or rendering fails
     */
    public String renderStructuredJson(final String rule, final boolean formatJson, final int maxRecursionDepth) {
        ensureOpen();
        Validate.notBlank(rule, "rule must not be blank");
        Validate.isTrue(maxRecursionDepth >= 0, "maxRecursionDepth must be non-negative");
        return NativeLibrary.INSTANCE.renderStructuredJsonWithContext(
                handle, rule, Map.of(), formatJson, maxRecursionDepth);
    }

    /**
     * Renders a named structured rule from this ruleset as formatted JSON text with initial context values.
     *
     * @param rule name of the structured rule to render
     * @param context initial render context values
     * @return formatted JSON for {@code rule}
     * @throws NullPointerException if {@code context}, a context key, or a context value is null
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if this ruleset is closed or rendering fails
     */
    public String renderStructuredJson(final String rule, final Map<String, String> context) {
        return renderStructuredJson(rule, context, true);
    }

    /**
     * Renders a named structured rule from this ruleset as JSON text with initial context values.
     *
     * @param rule name of the structured rule to render
     * @param context initial render context values
     * @param formatJson true to format JSON with tabs, false for compact JSON
     * @return JSON for {@code rule}
     * @throws NullPointerException if {@code context}, a context key, or a context value is null
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if this ruleset is closed or rendering fails
     */
    public String renderStructuredJson(
            final String rule, final Map<String, String> context, final boolean formatJson) {
        ensureOpen();
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(context, "context");
        validateContext(context);
        return NativeLibrary.INSTANCE.renderStructuredJsonWithContext(handle, rule, context, formatJson);
    }

    /**
     * Renders a named structured rule from this ruleset as JSON text with initial context and limited recursion enabled.
     *
     * @param rule name of the structured rule to render
     * @param context initial render context values
     * @param formatJson true to format JSON with tabs, false for compact JSON
     * @param maxRecursionDepth recursive re-entries allowed per rule
     * @return JSON for {@code rule}
     * @throws NullPointerException if {@code context}, a context key, or a context value is null
     * @throws IllegalArgumentException if {@code rule} is blank or {@code maxRecursionDepth} is negative
     * @throws CopperlaceException if this ruleset is closed or rendering fails
     */
    public String renderStructuredJson(
            final String rule,
            final Map<String, String> context,
            final boolean formatJson,
            final int maxRecursionDepth) {
        ensureOpen();
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(context, "context");
        Validate.isTrue(maxRecursionDepth >= 0, "maxRecursionDepth must be non-negative");
        validateContext(context);
        return NativeLibrary.INSTANCE.renderStructuredJsonWithContext(
                handle, rule, context, formatJson, maxRecursionDepth);
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
            if (ownedHandle != null) {
                ownedHandle.closeProcessors();
            }
            handle = MemorySegment.NULL;
            closed = true;
        }
    }

    private void ensureOpen() {
        if (closed) {
            throw new CopperlaceException("RuleSet is closed");
        }
    }

    private static void validateContext(final Map<String, String> context) {
        for (final Map.Entry<String, String> entry : context.entrySet()) {
            Objects.requireNonNull(entry.getKey(), "context key");
            Objects.requireNonNull(entry.getValue(), "context value");
        }
    }

    private static void validateProcessors(final Map<String, CopperlaceProcessor> processors) {
        Objects.requireNonNull(processors, "processors");
        for (final Map.Entry<String, CopperlaceProcessor> entry : processors.entrySet()) {
            Objects.requireNonNull(entry.getKey(), "processor name");
            Objects.requireNonNull(entry.getValue(), "processor");
        }
    }
}
