package dev.mahe.copperlace;

import java.nio.file.Path;

import org.apache.commons.lang3.Validate;

public final class Copperlace implements AutoCloseable {
    private final RuleSet ruleset;

    private Copperlace(final RuleSet ruleset) {
        this.ruleset = Validate.notNull(ruleset, "ruleset must not be null");
    }

    public static Copperlace fromString(final String config) {
        Validate.notBlank(config, "config must not be blank");
        return new Copperlace(RuleSet.fromString(config));
    }

    public static Copperlace fromFile(final Path path) {
        Validate.notNull(path, "path must not be null");
        return new Copperlace(RuleSet.fromFile(path));
    }

    public static Copperlace fromFile(final String path) {
        Validate.notBlank(path, "path must not be blank");
        return fromFile(Path.of(path));
    }

    public static String renderHoconString(final String config, final String rule) {
        Validate.notBlank(config, "config must not be blank");
        Validate.notBlank(rule, "rule must not be blank");

        try (final Copperlace copperlace = Copperlace.fromString(config)) {
            return copperlace.render(rule);
        }
    }

    public static String renderHoconFile(final Path path, final String rule) {
        Validate.notNull(path, "path must not be null");
        Validate.notBlank(rule, "rule must not be blank");

        try (final Copperlace copperlace = Copperlace.fromFile(path)) {
            return copperlace.render(rule);
        }
    }

    public static String renderHoconFile(final String path, final String rule) {
        Validate.notBlank(path, "path must not be blank");
        Validate.notBlank(rule, "rule must not be blank");

        return renderHoconFile(Path.of(path), rule);
    }

    public String render(final String rule) {
        Validate.notBlank(rule, "rule must not be blank");
        return ruleset.render(rule);
    }

    @Override
    public void close() {
        ruleset.close();
    }
}
