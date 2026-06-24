package dev.mahe.copperlace;

/**
 * Java callback used in Copperlace template processor pipelines.
 */
@FunctionalInterface
public interface CopperlaceProcessor {
    /**
     * Transforms one rendered value.
     *
     * @param value rendered input value
     * @return transformed value
     */
    String process(String value);
}
