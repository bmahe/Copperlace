package net.copperlace;

import java.nio.file.Path;

public final class Copperlace {
    private Copperlace() {
    }

    public static String renderHoconString(String config, String rule) {
        try (RuleSet ruleset = RuleSet.fromString(config)) {
            return ruleset.render(rule);
        }
    }

    public static String renderHoconFile(Path path, String rule) {
        try (RuleSet ruleset = RuleSet.fromFile(path)) {
            return ruleset.render(rule);
        }
    }
}
