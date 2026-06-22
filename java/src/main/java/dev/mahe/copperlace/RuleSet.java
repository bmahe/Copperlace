package dev.mahe.copperlace;

import java.lang.foreign.MemorySegment;
import java.nio.file.Path;
import java.util.Objects;

public final class RuleSet implements AutoCloseable {
    private MemorySegment handle;
    private boolean closed;

    private RuleSet(MemorySegment handle) {
        this.handle = handle;
    }

    public static RuleSet fromString(String config) {
        Objects.requireNonNull(config, "config");
        return new RuleSet(NativeLibrary.INSTANCE.rulesetFromString(config));
    }

    public static RuleSet fromFile(Path path) {
        Objects.requireNonNull(path, "path");
        return new RuleSet(NativeLibrary.INSTANCE.rulesetFromFile(path));
    }

    public String render(String rule) {
        ensureOpen();
        Objects.requireNonNull(rule, "rule");
        return NativeLibrary.INSTANCE.render(handle, rule);
    }

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
