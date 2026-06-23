package dev.mahe.copperlace;

import java.lang.foreign.MemorySegment;
import java.nio.file.Path;
import java.util.Objects;

import org.apache.commons.lang3.Validate;

public final class RuleSet implements AutoCloseable {
    private MemorySegment handle;
    private boolean closed;

    private RuleSet(final MemorySegment handle) {
        Validate.isTrue(!NativeLibrary.isNull(handle), "handle must not be null");
        this.handle = handle;
    }

    public static RuleSet fromString(final String config) {
        Validate.notBlank(config, "config must not be blank");
        return new RuleSet(NativeLibrary.INSTANCE.rulesetFromString(config));
    }

    public static RuleSet fromFile(final Path path) {
        Objects.requireNonNull(path, "path");
        return new RuleSet(NativeLibrary.INSTANCE.rulesetFromFile(path));
    }

    public String render(final String rule) {
        ensureOpen();
        Validate.notBlank(rule, "rule must not be blank");
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
