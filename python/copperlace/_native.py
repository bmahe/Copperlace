from __future__ import annotations

import ctypes
import os
import platform
from pathlib import Path


COPPERLACE_OK = 0
COPPERLACE_INVALID_ARGUMENT = 1
COPPERLACE_PARSE_ERROR = 2
COPPERLACE_RENDER_ERROR = 3


class NativeError(RuntimeError):
    def __init__(self, status: int, message: str) -> None:
        self.status = status
        super().__init__(message)


class NativeLibrary:
    def __init__(self) -> None:
        self._library = ctypes.CDLL(str(find_library()))
        self._configure_signatures()

    def ruleset_from_string(self, config: str) -> ctypes.c_void_p:
        handle = ctypes.c_void_p()
        error = ctypes.c_void_p()
        status = self._library.copperlace_ruleset_from_string(
            config.encode("utf-8"),
            ctypes.byref(handle),
            ctypes.byref(error),
        )
        self._raise_for_status(status, error)
        return handle

    def ruleset_from_file(self, path: str | os.PathLike[str]) -> ctypes.c_void_p:
        handle = ctypes.c_void_p()
        error = ctypes.c_void_p()
        status = self._library.copperlace_ruleset_from_file(
            os.fsencode(path),
            ctypes.byref(handle),
            ctypes.byref(error),
        )
        self._raise_for_status(status, error)
        return handle

    def ruleset_render(self, handle: ctypes.c_void_p, rule: str) -> str:
        output = ctypes.c_void_p()
        error = ctypes.c_void_p()
        status = self._library.copperlace_ruleset_render(
            handle,
            rule.encode("utf-8"),
            ctypes.byref(output),
            ctypes.byref(error),
        )
        self._raise_for_status(status, error)
        try:
            return ctypes.string_at(output).decode("utf-8")
        finally:
            self.string_free(output)

    def ruleset_free(self, handle: ctypes.c_void_p) -> None:
        self._library.copperlace_ruleset_free(handle)

    def string_free(self, value: ctypes.c_void_p) -> None:
        self._library.copperlace_string_free(value)

    def _configure_signatures(self) -> None:
        self._library.copperlace_ruleset_from_string.argtypes = [
            ctypes.c_char_p,
            ctypes.POINTER(ctypes.c_void_p),
            ctypes.POINTER(ctypes.c_void_p),
        ]
        self._library.copperlace_ruleset_from_string.restype = ctypes.c_int

        self._library.copperlace_ruleset_from_file.argtypes = [
            ctypes.c_char_p,
            ctypes.POINTER(ctypes.c_void_p),
            ctypes.POINTER(ctypes.c_void_p),
        ]
        self._library.copperlace_ruleset_from_file.restype = ctypes.c_int

        self._library.copperlace_ruleset_render.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
            ctypes.POINTER(ctypes.c_void_p),
            ctypes.POINTER(ctypes.c_void_p),
        ]
        self._library.copperlace_ruleset_render.restype = ctypes.c_int

        self._library.copperlace_ruleset_free.argtypes = [ctypes.c_void_p]
        self._library.copperlace_ruleset_free.restype = None

        self._library.copperlace_string_free.argtypes = [ctypes.c_void_p]
        self._library.copperlace_string_free.restype = None

    def _raise_for_status(self, status: int, error: ctypes.c_void_p) -> None:
        if status == COPPERLACE_OK:
            return

        message = "Copperlace native call failed"
        if error:
            try:
                message = ctypes.string_at(error).decode("utf-8")
            finally:
                self.string_free(error)
        raise NativeError(status, message)


def find_library() -> Path:
    override = os.environ.get("COPPERLACE_LIBRARY_PATH")
    if override:
        path = Path(override)
        if path.exists():
            return path

    library_name = native_library_name()
    package_library = Path(__file__).resolve().parent / "native" / library_name
    if package_library.exists():
        return package_library

    repo_library = (
        Path(__file__).resolve().parents[2]
        / "rust-core"
        / "target"
        / "release"
        / library_name
    )
    if repo_library.exists():
        return repo_library

    raise FileNotFoundError(
        f"Could not find {library_name}. Build rust-core or set COPPERLACE_LIBRARY_PATH."
    )


def native_library_name() -> str:
    system = platform.system()
    if system == "Windows":
        return "copperlace.dll"
    if system == "Darwin":
        return "libcopperlace.dylib"
    return "libcopperlace.so"


_NATIVE: NativeLibrary | None = None


def native() -> NativeLibrary:
    global _NATIVE
    if _NATIVE is None:
        _NATIVE = NativeLibrary()
    return _NATIVE
