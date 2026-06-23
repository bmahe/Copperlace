package dev.mahe.copperlace;

public final class CopperlaceException extends RuntimeException {
    private static final long serialVersionUID = 1L;

    public CopperlaceException(final String message) {
        super(message);
    }

    public CopperlaceException(final String message, final Throwable cause) {
        super(message, cause);
    }
}
