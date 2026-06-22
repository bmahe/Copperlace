package net.copperlace;

public class CopperlaceException extends RuntimeException {
    public CopperlaceException(String message) {
        super(message);
    }

    public CopperlaceException(String message, Throwable cause) {
        super(message, cause);
    }
}
