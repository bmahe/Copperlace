package dev.mahe.copperlace;

import java.nio.file.Path;
import java.util.Map;
import java.util.Objects;

import org.apache.commons.lang3.Validate;

/**
 * High-level Copperlace renderer for repeated renders from one configuration.
 *
 * <p>{@code Copperlace} compiles a config once and owns the resulting native
 * ruleset until {@link #close()} is called. Use this class when rendering more
 * than one rule, or when rendering the same rule multiple times, so the config
 * does not need to be parsed and compiled for each render.
 *
 * <p>Instances are {@link AutoCloseable}; callers should prefer
 * try-with-resources.
 */
public final class Copperlace implements AutoCloseable {
    private final RuleSet ruleset;

    private Copperlace(final RuleSet ruleset) {
        this.ruleset = Validate.notNull(ruleset, "ruleset must not be null");
    }

    /**
     * Compiles a configuration string into a reusable renderer.
     *
     * @param config configuration text containing Copperlace rules
     * @return a renderer backed by a native Copperlace ruleset
     * @throws IllegalArgumentException if {@code config} is blank
     * @throws CopperlaceException if the config cannot be parsed or compiled
     */
    public static Copperlace fromString(final String config) {
        Validate.notBlank(config, "config must not be blank");
        return new Copperlace(RuleSet.fromString(config));
    }

    /**
     * Compiles a configuration string into a reusable renderer with custom processors.
     *
     * @param config configuration text containing Copperlace rules
     * @param processors custom processor callbacks keyed by processor name
     * @return a renderer backed by a native Copperlace ruleset
     * @throws NullPointerException if {@code processors}, a processor name, or a processor is null
     * @throws IllegalArgumentException if {@code config} is blank
     * @throws CopperlaceException if the config cannot be parsed or compiled
     */
    public static Copperlace fromStringWithProcessors(
            final String config, final Map<String, CopperlaceProcessor> processors) {
        Validate.notBlank(config, "config must not be blank");
        Objects.requireNonNull(processors, "processors");
        return new Copperlace(RuleSet.fromStringWithProcessors(config, processors));
    }

    /**
     * Loads and compiles a configuration file into a reusable renderer.
     *
     * @param path path to the configuration file
     * @return a renderer backed by a native Copperlace ruleset
     * @throws NullPointerException if {@code path} is null
     * @throws CopperlaceException if the file cannot be loaded, parsed, or compiled
     */
    public static Copperlace fromFile(final Path path) {
        Validate.notNull(path, "path must not be null");
        return new Copperlace(RuleSet.fromFile(path));
    }

    /**
     * Loads and compiles a configuration file into a reusable renderer with custom processors.
     *
     * @param path path to the configuration file
     * @param processors custom processor callbacks keyed by processor name
     * @return a renderer backed by a native Copperlace ruleset
     * @throws NullPointerException if {@code path}, {@code processors}, a processor name, or a processor is null
     * @throws CopperlaceException if the file cannot be loaded, parsed, or compiled
     */
    public static Copperlace fromFileWithProcessors(
            final Path path, final Map<String, CopperlaceProcessor> processors) {
        Validate.notNull(path, "path must not be null");
        Objects.requireNonNull(processors, "processors");
        return new Copperlace(RuleSet.fromFileWithProcessors(path, processors));
    }

    /**
     * Loads and compiles a configuration file into a reusable renderer.
     *
     * @param path path to the configuration file
     * @return a renderer backed by a native Copperlace ruleset
     * @throws IllegalArgumentException if {@code path} is blank
     * @throws CopperlaceException if the file cannot be loaded, parsed, or compiled
     */
    public static Copperlace fromFile(final String path) {
        Validate.notBlank(path, "path must not be blank");
        return fromFile(Path.of(path));
    }

    /**
     * Loads and compiles a configuration file into a reusable renderer with custom processors.
     *
     * @param path path to the configuration file
     * @param processors custom processor callbacks keyed by processor name
     * @return a renderer backed by a native Copperlace ruleset
     * @throws NullPointerException if {@code processors}, a processor name, or a processor is null
     * @throws IllegalArgumentException if {@code path} is blank
     * @throws CopperlaceException if the file cannot be loaded, parsed, or compiled
     */
    public static Copperlace fromFileWithProcessors(
            final String path, final Map<String, CopperlaceProcessor> processors) {
        Validate.notBlank(path, "path must not be blank");
        Objects.requireNonNull(processors, "processors");
        return fromFileWithProcessors(Path.of(path), processors);
    }

    /**
     * Renders one rule from a configuration string.
     *
     * <p>This convenience method compiles the config, renders one rule, and
     * releases the native ruleset. Use {@link #fromString(String)} for repeated
     * renders from the same config.
     *
     * @param config configuration text containing Copperlace rules
     * @param rule name of the rule to render
     * @return rendered text for {@code rule}
     * @throws IllegalArgumentException if {@code config} or {@code rule} is blank
     * @throws CopperlaceException if parsing, compilation, or rendering fails
     */
    public static String renderString(final String config, final String rule) {
        Validate.notBlank(config, "config must not be blank");
        Validate.notBlank(rule, "rule must not be blank");

        try (final Copperlace copperlace = Copperlace.fromString(config)) {
            return copperlace.render(rule);
        }
    }

    /**
     * Renders one rule from a configuration string with custom processors.
     *
     * @param config configuration text containing Copperlace rules
     * @param rule name of the rule to render
     * @param processors custom processor callbacks keyed by processor name
     * @return rendered text for {@code rule}
     * @throws NullPointerException if {@code processors}, a processor name, or a processor is null
     * @throws IllegalArgumentException if {@code config} or {@code rule} is blank
     * @throws CopperlaceException if parsing, compilation, or rendering fails
     */
    public static String renderStringWithProcessors(
            final String config, final String rule, final Map<String, CopperlaceProcessor> processors) {
        Validate.notBlank(config, "config must not be blank");
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(processors, "processors");

        try (final Copperlace copperlace = Copperlace.fromStringWithProcessors(config, processors)) {
            return copperlace.render(rule);
        }
    }

    /**
     * Renders one rule from a configuration string with custom processors and initial context values.
     *
     * @param config configuration text containing Copperlace rules
     * @param rule name of the rule to render
     * @param context initial render context values
     * @param processors custom processor callbacks keyed by processor name
     * @return rendered text for {@code rule}
     * @throws NullPointerException if {@code context}, {@code processors}, a key, value, processor name, or processor is null
     * @throws IllegalArgumentException if {@code config} or {@code rule} is blank
     * @throws CopperlaceException if parsing, compilation, or rendering fails
     */
    public static String renderStringWithProcessors(
            final String config,
            final String rule,
            final Map<String, String> context,
            final Map<String, CopperlaceProcessor> processors) {
        Validate.notBlank(config, "config must not be blank");
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(context, "context");
        Objects.requireNonNull(processors, "processors");

        try (final Copperlace copperlace = Copperlace.fromStringWithProcessors(config, processors)) {
            return copperlace.render(rule, context);
        }
    }

    /**
     * Renders one rule from a configuration string with initial context values.
     *
     * <p>This convenience method compiles the config, renders one rule, and
     * releases the native ruleset. Use {@link #fromString(String)} for repeated
     * renders from the same config.
     *
     * @param config configuration text containing Copperlace rules
     * @param rule name of the rule to render
     * @param context initial render context values
     * @return rendered text for {@code rule}
     * @throws NullPointerException if {@code context}, a context key, or a context value is null
     * @throws IllegalArgumentException if {@code config} or {@code rule} is blank
     * @throws CopperlaceException if parsing, compilation, or rendering fails
     */
    public static String renderString(
            final String config, final String rule, final Map<String, String> context) {
        Validate.notBlank(config, "config must not be blank");
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(context, "context");

        try (final Copperlace copperlace = Copperlace.fromString(config)) {
            return copperlace.render(rule, context);
        }
    }

    /**
     * Renders one rule from a configuration string as text, returning formatted JSON for object-valued rules.
     *
     * @param config configuration text containing Copperlace rules
     * @param rule name of the rule to render
     * @return rendered text, or formatted JSON for an object-valued rule
     * @throws IllegalArgumentException if {@code config} or {@code rule} is blank
     * @throws CopperlaceException if parsing, compilation, or rendering fails
     */
    public static String renderStringInferred(final String config, final String rule) {
        Validate.notBlank(config, "config must not be blank");
        Validate.notBlank(rule, "rule must not be blank");

        try (final Copperlace copperlace = Copperlace.fromString(config)) {
            return copperlace.renderInferred(rule);
        }
    }

    /**
     * Renders one rule from a configuration string with initial context, returning formatted JSON for object-valued rules.
     *
     * @param config configuration text containing Copperlace rules
     * @param rule name of the rule to render
     * @param context initial render context values
     * @return rendered text, or formatted JSON for an object-valued rule
     * @throws NullPointerException if {@code context}, a context key, or a context value is null
     * @throws IllegalArgumentException if {@code config} or {@code rule} is blank
     * @throws CopperlaceException if parsing, compilation, or rendering fails
     */
    public static String renderStringInferred(
            final String config, final String rule, final Map<String, String> context) {
        Validate.notBlank(config, "config must not be blank");
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(context, "context");

        try (final Copperlace copperlace = Copperlace.fromString(config)) {
            return copperlace.renderInferred(rule, context);
        }
    }

    /**
     * Renders one structured rule from a configuration string as formatted JSON text.
     *
     * @param config configuration text containing Copperlace rules
     * @param rule name of the structured rule to render
     * @return formatted JSON for {@code rule}
     * @throws IllegalArgumentException if {@code config} or {@code rule} is blank
     * @throws CopperlaceException if parsing, compilation, or rendering fails
     */
    public static String renderStringStructuredJson(final String config, final String rule) {
        return renderStringStructuredJson(config, rule, true);
    }

    /**
     * Renders one structured rule from a configuration string as JSON text.
     *
     * @param config configuration text containing Copperlace rules
     * @param rule name of the structured rule to render
     * @param formatJson true to format JSON with tabs, false for compact JSON
     * @return JSON for {@code rule}
     * @throws IllegalArgumentException if {@code config} or {@code rule} is blank
     * @throws CopperlaceException if parsing, compilation, or rendering fails
     */
    public static String renderStringStructuredJson(
            final String config, final String rule, final boolean formatJson) {
        Validate.notBlank(config, "config must not be blank");
        Validate.notBlank(rule, "rule must not be blank");

        try (final Copperlace copperlace = Copperlace.fromString(config)) {
            return copperlace.renderStructuredJson(rule, formatJson);
        }
    }

    /**
     * Renders one structured rule from a configuration string as formatted JSON text with initial context values.
     *
     * @param config configuration text containing Copperlace rules
     * @param rule name of the structured rule to render
     * @param context initial render context values
     * @return formatted JSON for {@code rule}
     * @throws NullPointerException if {@code context}, a context key, or a context value is null
     * @throws IllegalArgumentException if {@code config} or {@code rule} is blank
     * @throws CopperlaceException if parsing, compilation, or rendering fails
     */
    public static String renderStringStructuredJson(
            final String config, final String rule, final Map<String, String> context) {
        return renderStringStructuredJson(config, rule, context, true);
    }

    /**
     * Renders one structured rule from a configuration string as JSON text with initial context values.
     *
     * @param config configuration text containing Copperlace rules
     * @param rule name of the structured rule to render
     * @param context initial render context values
     * @param formatJson true to format JSON with tabs, false for compact JSON
     * @return JSON for {@code rule}
     * @throws NullPointerException if {@code context}, a context key, or a context value is null
     * @throws IllegalArgumentException if {@code config} or {@code rule} is blank
     * @throws CopperlaceException if parsing, compilation, or rendering fails
     */
    public static String renderStringStructuredJson(
            final String config,
            final String rule,
            final Map<String, String> context,
            final boolean formatJson) {
        Validate.notBlank(config, "config must not be blank");
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(context, "context");

        try (final Copperlace copperlace = Copperlace.fromString(config)) {
            return copperlace.renderStructuredJson(rule, context, formatJson);
        }
    }

    /**
     * Renders one rule from a configuration file.
     *
     * <p>This convenience method loads and compiles the file, renders one rule,
     * and releases the native ruleset. Use {@link #fromFile(Path)} for repeated
     * renders from the same config.
     *
     * @param path path to the configuration file
     * @param rule name of the rule to render
     * @return rendered text for {@code rule}
     * @throws NullPointerException if {@code path} is null
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if loading, parsing, compilation, or rendering fails
     */
    public static String renderFile(final Path path, final String rule) {
        Validate.notNull(path, "path must not be null");
        Validate.notBlank(rule, "rule must not be blank");

        try (final Copperlace copperlace = Copperlace.fromFile(path)) {
            return copperlace.render(rule);
        }
    }

    /**
     * Renders one rule from a configuration file with custom processors.
     *
     * @param path path to the configuration file
     * @param rule name of the rule to render
     * @param processors custom processor callbacks keyed by processor name
     * @return rendered text for {@code rule}
     * @throws NullPointerException if {@code path}, {@code processors}, a processor name, or a processor is null
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if loading, parsing, compilation, or rendering fails
     */
    public static String renderFileWithProcessors(
            final Path path, final String rule, final Map<String, CopperlaceProcessor> processors) {
        Validate.notNull(path, "path must not be null");
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(processors, "processors");

        try (final Copperlace copperlace = Copperlace.fromFileWithProcessors(path, processors)) {
            return copperlace.render(rule);
        }
    }

    /**
     * Renders one rule from a configuration file with custom processors and initial context values.
     *
     * @param path path to the configuration file
     * @param rule name of the rule to render
     * @param context initial render context values
     * @param processors custom processor callbacks keyed by processor name
     * @return rendered text for {@code rule}
     * @throws NullPointerException if {@code path}, {@code context}, {@code processors}, a key, value, processor name, or processor is null
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if loading, parsing, compilation, or rendering fails
     */
    public static String renderFileWithProcessors(
            final Path path,
            final String rule,
            final Map<String, String> context,
            final Map<String, CopperlaceProcessor> processors) {
        Validate.notNull(path, "path must not be null");
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(context, "context");
        Objects.requireNonNull(processors, "processors");

        try (final Copperlace copperlace = Copperlace.fromFileWithProcessors(path, processors)) {
            return copperlace.render(rule, context);
        }
    }

    /**
     * Renders one rule from a configuration file with initial context values.
     *
     * <p>This convenience method loads and compiles the file, renders one rule,
     * and releases the native ruleset. Use {@link #fromFile(Path)} for repeated
     * renders from the same config.
     *
     * @param path path to the configuration file
     * @param rule name of the rule to render
     * @param context initial render context values
     * @return rendered text for {@code rule}
     * @throws NullPointerException if {@code path}, {@code context}, a context key, or a context value is null
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if loading, parsing, compilation, or rendering fails
     */
    public static String renderFile(final Path path, final String rule, final Map<String, String> context) {
        Validate.notNull(path, "path must not be null");
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(context, "context");

        try (final Copperlace copperlace = Copperlace.fromFile(path)) {
            return copperlace.render(rule, context);
        }
    }

    /**
     * Renders one rule from a configuration file as text, returning formatted JSON for object-valued rules.
     *
     * @param path path to the configuration file
     * @param rule name of the rule to render
     * @return rendered text, or formatted JSON for an object-valued rule
     * @throws NullPointerException if {@code path} is null
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if loading, parsing, compilation, or rendering fails
     */
    public static String renderFileInferred(final Path path, final String rule) {
        Validate.notNull(path, "path must not be null");
        Validate.notBlank(rule, "rule must not be blank");

        try (final Copperlace copperlace = Copperlace.fromFile(path)) {
            return copperlace.renderInferred(rule);
        }
    }

    /**
     * Renders one rule from a configuration file with initial context, returning formatted JSON for object-valued rules.
     *
     * @param path path to the configuration file
     * @param rule name of the rule to render
     * @param context initial render context values
     * @return rendered text, or formatted JSON for an object-valued rule
     * @throws NullPointerException if {@code path}, {@code context}, a context key, or a context value is null
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if loading, parsing, compilation, or rendering fails
     */
    public static String renderFileInferred(
            final Path path, final String rule, final Map<String, String> context) {
        Validate.notNull(path, "path must not be null");
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(context, "context");

        try (final Copperlace copperlace = Copperlace.fromFile(path)) {
            return copperlace.renderInferred(rule, context);
        }
    }

    /**
     * Renders one structured rule from a configuration file as formatted JSON text.
     *
     * @param path path to the configuration file
     * @param rule name of the structured rule to render
     * @return formatted JSON for {@code rule}
     * @throws NullPointerException if {@code path} is null
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if loading, parsing, compilation, or rendering fails
     */
    public static String renderFileStructuredJson(final Path path, final String rule) {
        return renderFileStructuredJson(path, rule, true);
    }

    /**
     * Renders one structured rule from a configuration file as JSON text.
     *
     * @param path path to the configuration file
     * @param rule name of the structured rule to render
     * @param formatJson true to format JSON with tabs, false for compact JSON
     * @return JSON for {@code rule}
     * @throws NullPointerException if {@code path} is null
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if loading, parsing, compilation, or rendering fails
     */
    public static String renderFileStructuredJson(final Path path, final String rule, final boolean formatJson) {
        Validate.notNull(path, "path must not be null");
        Validate.notBlank(rule, "rule must not be blank");

        try (final Copperlace copperlace = Copperlace.fromFile(path)) {
            return copperlace.renderStructuredJson(rule, formatJson);
        }
    }

    /**
     * Renders one structured rule from a configuration file as formatted JSON text with initial context values.
     *
     * @param path path to the configuration file
     * @param rule name of the structured rule to render
     * @param context initial render context values
     * @return formatted JSON for {@code rule}
     * @throws NullPointerException if {@code path}, {@code context}, a context key, or a context value is null
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if loading, parsing, compilation, or rendering fails
     */
    public static String renderFileStructuredJson(
            final Path path, final String rule, final Map<String, String> context) {
        return renderFileStructuredJson(path, rule, context, true);
    }

    /**
     * Renders one structured rule from a configuration file as JSON text with initial context values.
     *
     * @param path path to the configuration file
     * @param rule name of the structured rule to render
     * @param context initial render context values
     * @param formatJson true to format JSON with tabs, false for compact JSON
     * @return JSON for {@code rule}
     * @throws NullPointerException if {@code path}, {@code context}, a context key, or a context value is null
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if loading, parsing, compilation, or rendering fails
     */
    public static String renderFileStructuredJson(
            final Path path,
            final String rule,
            final Map<String, String> context,
            final boolean formatJson) {
        Validate.notNull(path, "path must not be null");
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(context, "context");

        try (final Copperlace copperlace = Copperlace.fromFile(path)) {
            return copperlace.renderStructuredJson(rule, context, formatJson);
        }
    }

    /**
     * Renders one rule from a configuration file.
     *
     * <p>This convenience method loads and compiles the file, renders one rule,
     * and releases the native ruleset. Use {@link #fromFile(String)} for
     * repeated renders from the same config.
     *
     * @param path path to the configuration file
     * @param rule name of the rule to render
     * @return rendered text for {@code rule}
     * @throws IllegalArgumentException if {@code path} or {@code rule} is blank
     * @throws CopperlaceException if loading, parsing, compilation, or rendering fails
     */
    public static String renderFile(final String path, final String rule) {
        Validate.notBlank(path, "path must not be blank");
        Validate.notBlank(rule, "rule must not be blank");

        return renderFile(Path.of(path), rule);
    }

    /**
     * Renders one rule from a configuration file with custom processors.
     *
     * @param path path to the configuration file
     * @param rule name of the rule to render
     * @param processors custom processor callbacks keyed by processor name
     * @return rendered text for {@code rule}
     * @throws NullPointerException if {@code processors}, a processor name, or a processor is null
     * @throws IllegalArgumentException if {@code path} or {@code rule} is blank
     * @throws CopperlaceException if loading, parsing, compilation, or rendering fails
     */
    public static String renderFileWithProcessors(
            final String path, final String rule, final Map<String, CopperlaceProcessor> processors) {
        Validate.notBlank(path, "path must not be blank");
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(processors, "processors");

        return renderFileWithProcessors(Path.of(path), rule, processors);
    }

    /**
     * Renders one rule from a configuration file with custom processors and initial context values.
     *
     * @param path path to the configuration file
     * @param rule name of the rule to render
     * @param context initial render context values
     * @param processors custom processor callbacks keyed by processor name
     * @return rendered text for {@code rule}
     * @throws NullPointerException if {@code context}, {@code processors}, a key, value, processor name, or processor is null
     * @throws IllegalArgumentException if {@code path} or {@code rule} is blank
     * @throws CopperlaceException if loading, parsing, compilation, or rendering fails
     */
    public static String renderFileWithProcessors(
            final String path,
            final String rule,
            final Map<String, String> context,
            final Map<String, CopperlaceProcessor> processors) {
        Validate.notBlank(path, "path must not be blank");
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(context, "context");
        Objects.requireNonNull(processors, "processors");

        return renderFileWithProcessors(Path.of(path), rule, context, processors);
    }

    /**
     * Renders one rule from a configuration file with initial context values.
     *
     * <p>This convenience method loads and compiles the file, renders one rule,
     * and releases the native ruleset. Use {@link #fromFile(String)} for
     * repeated renders from the same config.
     *
     * @param path path to the configuration file
     * @param rule name of the rule to render
     * @param context initial render context values
     * @return rendered text for {@code rule}
     * @throws NullPointerException if {@code context}, a context key, or a context value is null
     * @throws IllegalArgumentException if {@code path} or {@code rule} is blank
     * @throws CopperlaceException if loading, parsing, compilation, or rendering fails
     */
    public static String renderFile(final String path, final String rule, final Map<String, String> context) {
        Validate.notBlank(path, "path must not be blank");
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(context, "context");

        return renderFile(Path.of(path), rule, context);
    }

    /**
     * Renders one rule from a configuration file as text, returning formatted JSON for object-valued rules.
     *
     * @param path path to the configuration file
     * @param rule name of the rule to render
     * @return rendered text, or formatted JSON for an object-valued rule
     * @throws IllegalArgumentException if {@code path} or {@code rule} is blank
     * @throws CopperlaceException if loading, parsing, compilation, or rendering fails
     */
    public static String renderFileInferred(final String path, final String rule) {
        Validate.notBlank(path, "path must not be blank");
        Validate.notBlank(rule, "rule must not be blank");

        return renderFileInferred(Path.of(path), rule);
    }

    /**
     * Renders one rule from a configuration file with initial context, returning formatted JSON for object-valued rules.
     *
     * @param path path to the configuration file
     * @param rule name of the rule to render
     * @param context initial render context values
     * @return rendered text, or formatted JSON for an object-valued rule
     * @throws NullPointerException if {@code context}, a context key, or a context value is null
     * @throws IllegalArgumentException if {@code path} or {@code rule} is blank
     * @throws CopperlaceException if loading, parsing, compilation, or rendering fails
     */
    public static String renderFileInferred(
            final String path, final String rule, final Map<String, String> context) {
        Validate.notBlank(path, "path must not be blank");
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(context, "context");

        return renderFileInferred(Path.of(path), rule, context);
    }

    /**
     * Renders one structured rule from a configuration file as formatted JSON text.
     *
     * @param path path to the configuration file
     * @param rule name of the structured rule to render
     * @return formatted JSON for {@code rule}
     * @throws IllegalArgumentException if {@code path} or {@code rule} is blank
     * @throws CopperlaceException if loading, parsing, compilation, or rendering fails
     */
    public static String renderFileStructuredJson(final String path, final String rule) {
        Validate.notBlank(path, "path must not be blank");
        Validate.notBlank(rule, "rule must not be blank");

        return renderFileStructuredJson(Path.of(path), rule);
    }

    /**
     * Renders one structured rule from a configuration file as JSON text.
     *
     * @param path path to the configuration file
     * @param rule name of the structured rule to render
     * @param formatJson true to format JSON with tabs, false for compact JSON
     * @return JSON for {@code rule}
     * @throws IllegalArgumentException if {@code path} or {@code rule} is blank
     * @throws CopperlaceException if loading, parsing, compilation, or rendering fails
     */
    public static String renderFileStructuredJson(final String path, final String rule, final boolean formatJson) {
        Validate.notBlank(path, "path must not be blank");
        Validate.notBlank(rule, "rule must not be blank");

        return renderFileStructuredJson(Path.of(path), rule, formatJson);
    }

    /**
     * Renders one structured rule from a configuration file as formatted JSON text with initial context values.
     *
     * @param path path to the configuration file
     * @param rule name of the structured rule to render
     * @param context initial render context values
     * @return formatted JSON for {@code rule}
     * @throws NullPointerException if {@code context}, a context key, or a context value is null
     * @throws IllegalArgumentException if {@code path} or {@code rule} is blank
     * @throws CopperlaceException if loading, parsing, compilation, or rendering fails
     */
    public static String renderFileStructuredJson(
            final String path, final String rule, final Map<String, String> context) {
        return renderFileStructuredJson(path, rule, context, true);
    }

    /**
     * Renders one structured rule from a configuration file as JSON text with initial context values.
     *
     * @param path path to the configuration file
     * @param rule name of the structured rule to render
     * @param context initial render context values
     * @param formatJson true to format JSON with tabs, false for compact JSON
     * @return JSON for {@code rule}
     * @throws NullPointerException if {@code context}, a context key, or a context value is null
     * @throws IllegalArgumentException if {@code path} or {@code rule} is blank
     * @throws CopperlaceException if loading, parsing, compilation, or rendering fails
     */
    public static String renderFileStructuredJson(
            final String path,
            final String rule,
            final Map<String, String> context,
            final boolean formatJson) {
        Validate.notBlank(path, "path must not be blank");
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(context, "context");

        return renderFileStructuredJson(Path.of(path), rule, context, formatJson);
    }

    /**
     * Renders a named rule from the loaded config.
     *
     * <p>Each call uses a fresh render context, so per-render bindings are
     * consistent within one output but do not carry over to later renders.
     *
     * @param rule name of the rule to render
     * @return rendered text for {@code rule}
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if this renderer is closed or rendering fails
     */
    public String render(final String rule) {
        Validate.notBlank(rule, "rule must not be blank");
        return ruleset.render(rule);
    }

    /**
     * Renders a named rule from the loaded config with initial context values.
     *
     * <p>The provided context is scoped to this render only. Values resolve
     * before config-defined {@code context} defaults and named rules.
     *
     * @param rule name of the rule to render
     * @param context initial render context values
     * @return rendered text for {@code rule}
     * @throws NullPointerException if {@code context}, a context key, or a context value is null
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if this renderer is closed or rendering fails
     */
    public String render(final String rule, final Map<String, String> context) {
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(context, "context");
        return ruleset.render(rule, context);
    }

    /**
     * Renders a named rule as text, returning formatted JSON for object-valued rules.
     *
     * @param rule name of the rule to render
     * @return rendered text, or formatted JSON for an object-valued rule
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if this renderer is closed or rendering fails
     */
    public String renderInferred(final String rule) {
        Validate.notBlank(rule, "rule must not be blank");
        return ruleset.renderInferred(rule);
    }

    /**
     * Renders a named rule with initial context, returning formatted JSON for object-valued rules.
     *
     * @param rule name of the rule to render
     * @param context initial render context values
     * @return rendered text, or formatted JSON for an object-valued rule
     * @throws NullPointerException if {@code context}, a context key, or a context value is null
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if this renderer is closed or rendering fails
     */
    public String renderInferred(final String rule, final Map<String, String> context) {
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(context, "context");
        return ruleset.renderInferred(rule, context);
    }

    /**
     * Renders a named structured rule from the loaded config as formatted JSON text.
     *
     * @param rule name of the structured rule to render
     * @return formatted JSON for {@code rule}
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if this renderer is closed or rendering fails
     */
    public String renderStructuredJson(final String rule) {
        return renderStructuredJson(rule, true);
    }

    /**
     * Renders a named structured rule from the loaded config as JSON text.
     *
     * @param rule name of the structured rule to render
     * @param formatJson true to format JSON with tabs, false for compact JSON
     * @return JSON for {@code rule}
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if this renderer is closed or rendering fails
     */
    public String renderStructuredJson(final String rule, final boolean formatJson) {
        Validate.notBlank(rule, "rule must not be blank");
        return ruleset.renderStructuredJson(rule, formatJson);
    }

    /**
     * Renders a named structured rule from the loaded config as formatted JSON text with initial context values.
     *
     * @param rule name of the structured rule to render
     * @param context initial render context values
     * @return formatted JSON for {@code rule}
     * @throws NullPointerException if {@code context}, a context key, or a context value is null
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if this renderer is closed or rendering fails
     */
    public String renderStructuredJson(final String rule, final Map<String, String> context) {
        return renderStructuredJson(rule, context, true);
    }

    /**
     * Renders a named structured rule from the loaded config as JSON text with initial context values.
     *
     * @param rule name of the structured rule to render
     * @param context initial render context values
     * @param formatJson true to format JSON with tabs, false for compact JSON
     * @return JSON for {@code rule}
     * @throws NullPointerException if {@code context}, a context key, or a context value is null
     * @throws IllegalArgumentException if {@code rule} is blank
     * @throws CopperlaceException if this renderer is closed or rendering fails
     */
    public String renderStructuredJson(
            final String rule, final Map<String, String> context, final boolean formatJson) {
        Validate.notBlank(rule, "rule must not be blank");
        Objects.requireNonNull(context, "context");
        return ruleset.renderStructuredJson(rule, context, formatJson);
    }

    /**
     * Releases the underlying native ruleset handle.
     *
     * <p>Calling {@code close} more than once is allowed.
     */
    @Override
    public void close() {
        ruleset.close();
    }
}
