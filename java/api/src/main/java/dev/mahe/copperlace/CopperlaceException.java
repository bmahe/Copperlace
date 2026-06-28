package dev.mahe.copperlace;

/**
 * Runtime exception raised when Copperlace cannot load native code, parse
 * config, compile rules, or render a rule.
 */
public final class CopperlaceException extends RuntimeException {
    private static final long serialVersionUID = 1L;

    /**
     * Creates an exception with a detail message.
     *
     * @param message detail message
     */
    public CopperlaceException(final String message) {
        super(message);
    }

    /**
     * Creates an exception with a detail message and cause.
     *
     * @param message detail message
     * @param cause underlying cause
     */
    public CopperlaceException(final String message, final Throwable cause) {
        super(message, cause);
    }
}
