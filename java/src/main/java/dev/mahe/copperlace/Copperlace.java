package dev.mahe.copperlace;

import java.nio.file.Path;

import org.apache.commons.lang3.Validate;

public final class Copperlace {
    private Copperlace() {
    }

    public static String renderHoconString(final String config, final String rule) {
        Validate.notBlank(config, "config must not be blank");
        Validate.notBlank(rule, "rule must not be blank");

        try (final RuleSet ruleset = RuleSet.fromString(config)) {
            return ruleset.render(rule);
        }
    }

    public static String renderHoconFile(final Path path, final String rule) {
        Validate.notNull(path, "path must not be null");
        Validate.notBlank(rule, "rule must not be blank");

        try (final RuleSet ruleset = RuleSet.fromFile(path)) {
            return ruleset.render(rule);
        }
    }

    public static String renderHoconFile(final String path, final String rule) {
        Validate.notBlank(path, "path must not be blank");
        Validate.notBlank(rule, "rule must not be blank");

        return renderHoconFile(Path.of(path), rule);
    }

}
