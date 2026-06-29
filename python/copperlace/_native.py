from __future__ import annotations

import ctypes
import os
import platform
from collections.abc import Callable, Mapping
from pathlib import Path


COPPERLACE_OK = 0
COPPERLACE_INVALID_ARGUMENT = 1
COPPERLACE_PARSE_ERROR = 2
COPPERLACE_RENDER_ERROR = 3

_PROCESSOR_CALLBACK = ctypes.CFUNCTYPE(
    ctypes.c_int, ctypes.c_char_p, ctypes.c_void_p, ctypes.c_void_p
)


class NativeError(RuntimeError):
    def __init__(self, status: int, message: str) -> None:
        self.status = status
        super().__init__(message)


class NativeLibrary:
    def __init__(self) -> None:
        self._library = ctypes.CDLL(str(find_library()))
        self._configure_signatures()

    def ruleset_from_string(
        self, config: str, processors: Mapping[str, Callable[[str], str]] | None = None
    ) -> tuple[ctypes.c_void_p, object | None]:
        if processors:
            registry = NativeProcessorRegistry(self, processors)
            return self._ruleset_from_string_with_processors(config, registry), registry

        handle = ctypes.c_void_p()
        error = ctypes.c_void_p()
        status = self._library.copperlace_ruleset_from_string(
            config.encode("utf-8"),
            ctypes.byref(handle),
            ctypes.byref(error),
        )
        self._raise_for_status(status, error)
        return handle, None

    def ruleset_from_file(
        self,
        path: str | os.PathLike[str],
        processors: Mapping[str, Callable[[str], str]] | None = None,
    ) -> tuple[ctypes.c_void_p, object | None]:
        if processors:
            registry = NativeProcessorRegistry(self, processors)
            return self._ruleset_from_file_with_processors(path, registry), registry

        handle = ctypes.c_void_p()
        error = ctypes.c_void_p()
        status = self._library.copperlace_ruleset_from_file(
            os.fsencode(path),
            ctypes.byref(handle),
            ctypes.byref(error),
        )
        self._raise_for_status(status, error)
        return handle, None

    def _ruleset_from_string_with_processors(
        self, config: str, registry: NativeProcessorRegistry
    ) -> ctypes.c_void_p:
        handle = ctypes.c_void_p()
        error = ctypes.c_void_p()
        status = self._library.copperlace_ruleset_from_string_with_processors(
            config.encode("utf-8"),
            registry.names,
            registry.callbacks,
            registry.user_data,
            ctypes.c_size_t(registry.length),
            ctypes.byref(handle),
            ctypes.byref(error),
        )
        self._raise_for_status(status, error)
        return handle

    def _ruleset_from_file_with_processors(
        self, path: str | os.PathLike[str], registry: NativeProcessorRegistry
    ) -> ctypes.c_void_p:
        handle = ctypes.c_void_p()
        error = ctypes.c_void_p()
        status = self._library.copperlace_ruleset_from_file_with_processors(
            os.fsencode(path),
            registry.names,
            registry.callbacks,
            registry.user_data,
            ctypes.c_size_t(registry.length),
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

    def ruleset_render_with_context(
        self, handle: ctypes.c_void_p, rule: str, context: Mapping[str, str]
    ) -> str:
        encoded_keys = [key.encode("utf-8") for key in context.keys()]
        encoded_values = [value.encode("utf-8") for value in context.values()]
        keys = (ctypes.c_char_p * len(encoded_keys))(*encoded_keys)
        values = (ctypes.c_char_p * len(encoded_values))(*encoded_values)

        output = ctypes.c_void_p()
        error = ctypes.c_void_p()
        status = self._library.copperlace_ruleset_render_with_context(
            handle,
            rule.encode("utf-8"),
            keys,
            values,
            ctypes.c_size_t(len(encoded_keys)),
            ctypes.byref(output),
            ctypes.byref(error),
        )
        self._raise_for_status(status, error)
        try:
            return ctypes.string_at(output).decode("utf-8")
        finally:
            self.string_free(output)

    def ruleset_render_structured_json(
        self, handle: ctypes.c_void_p, rule: str, format_json: bool = False
    ) -> str:
        output = ctypes.c_void_p()
        error = ctypes.c_void_p()
        status = self._library.copperlace_ruleset_render_structured_json(
            handle,
            rule.encode("utf-8"),
            ctypes.c_bool(format_json),
            ctypes.byref(output),
            ctypes.byref(error),
        )
        self._raise_for_status(status, error)
        try:
            return ctypes.string_at(output).decode("utf-8")
        finally:
            self.string_free(output)

    def ruleset_render_structured_json_with_context(
        self,
        handle: ctypes.c_void_p,
        rule: str,
        context: Mapping[str, str],
        format_json: bool = False,
    ) -> str:
        encoded_keys = [key.encode("utf-8") for key in context.keys()]
        encoded_values = [value.encode("utf-8") for value in context.values()]
        keys = (ctypes.c_char_p * len(encoded_keys))(*encoded_keys)
        values = (ctypes.c_char_p * len(encoded_values))(*encoded_values)

        output = ctypes.c_void_p()
        error = ctypes.c_void_p()
        status = self._library.copperlace_ruleset_render_structured_json_with_context(
            handle,
            rule.encode("utf-8"),
            keys,
            values,
            ctypes.c_size_t(len(encoded_keys)),
            ctypes.c_bool(format_json),
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

    def processor_result_set_output(self, result: ctypes.c_void_p, value: str) -> int:
        return int(
            self._library.copperlace_processor_result_set_output(
                result, value.encode("utf-8")
            )
        )

    def processor_result_set_error(self, result: ctypes.c_void_p, message: str) -> int:
        return int(
            self._library.copperlace_processor_result_set_error(
                result, message.encode("utf-8")
            )
        )

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

        self._library.copperlace_ruleset_from_string_with_processors.argtypes = [
            ctypes.c_char_p,
            ctypes.POINTER(ctypes.c_char_p),
            ctypes.POINTER(_PROCESSOR_CALLBACK),
            ctypes.POINTER(ctypes.c_void_p),
            ctypes.c_size_t,
            ctypes.POINTER(ctypes.c_void_p),
            ctypes.POINTER(ctypes.c_void_p),
        ]
        self._library.copperlace_ruleset_from_string_with_processors.restype = ctypes.c_int

        self._library.copperlace_ruleset_from_file_with_processors.argtypes = [
            ctypes.c_char_p,
            ctypes.POINTER(ctypes.c_char_p),
            ctypes.POINTER(_PROCESSOR_CALLBACK),
            ctypes.POINTER(ctypes.c_void_p),
            ctypes.c_size_t,
            ctypes.POINTER(ctypes.c_void_p),
            ctypes.POINTER(ctypes.c_void_p),
        ]
        self._library.copperlace_ruleset_from_file_with_processors.restype = ctypes.c_int

        self._library.copperlace_ruleset_render.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
            ctypes.POINTER(ctypes.c_void_p),
            ctypes.POINTER(ctypes.c_void_p),
        ]
        self._library.copperlace_ruleset_render.restype = ctypes.c_int

        self._library.copperlace_ruleset_render_with_context.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
            ctypes.POINTER(ctypes.c_char_p),
            ctypes.POINTER(ctypes.c_char_p),
            ctypes.c_size_t,
            ctypes.POINTER(ctypes.c_void_p),
            ctypes.POINTER(ctypes.c_void_p),
        ]
        self._library.copperlace_ruleset_render_with_context.restype = ctypes.c_int

        self._library.copperlace_ruleset_render_structured_json.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
            ctypes.c_bool,
            ctypes.POINTER(ctypes.c_void_p),
            ctypes.POINTER(ctypes.c_void_p),
        ]
        self._library.copperlace_ruleset_render_structured_json.restype = ctypes.c_int

        self._library.copperlace_ruleset_render_structured_json_with_context.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
            ctypes.POINTER(ctypes.c_char_p),
            ctypes.POINTER(ctypes.c_char_p),
            ctypes.c_size_t,
            ctypes.c_bool,
            ctypes.POINTER(ctypes.c_void_p),
            ctypes.POINTER(ctypes.c_void_p),
        ]
        self._library.copperlace_ruleset_render_structured_json_with_context.restype = (
            ctypes.c_int
        )

        self._library.copperlace_ruleset_free.argtypes = [ctypes.c_void_p]
        self._library.copperlace_ruleset_free.restype = None

        self._library.copperlace_string_free.argtypes = [ctypes.c_void_p]
        self._library.copperlace_string_free.restype = None

        self._library.copperlace_processor_result_set_output.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
        ]
        self._library.copperlace_processor_result_set_output.restype = ctypes.c_int

        self._library.copperlace_processor_result_set_error.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
        ]
        self._library.copperlace_processor_result_set_error.restype = ctypes.c_int

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


class NativeProcessorRegistry:
    def __init__(
        self, library: NativeLibrary, processors: Mapping[str, Callable[[str], str]]
    ) -> None:
        self._library = library
        self._callbacks: list[_PROCESSOR_CALLBACK] = []
        encoded_names = []
        user_data = []

        for name, processor in processors.items():
            encoded_names.append(name.encode("utf-8"))
            self._callbacks.append(self._callback_for(processor))
            user_data.append(ctypes.c_void_p())

        self.length = len(encoded_names)
        self.names = (ctypes.c_char_p * self.length)(*encoded_names)
        self.callbacks = (_PROCESSOR_CALLBACK * self.length)(*self._callbacks)
        self.user_data = (ctypes.c_void_p * self.length)(*user_data)

    def _callback_for(self, processor: Callable[[str], str]) -> _PROCESSOR_CALLBACK:
        def callback(
            input_value: bytes, result: ctypes.c_void_p, _user_data: ctypes.c_void_p
        ) -> int:
            try:
                output = processor(input_value.decode("utf-8"))
                if not isinstance(output, str):
                    self._library.processor_result_set_error(
                        result, "processor returned a non-string value"
                    )
                    return COPPERLACE_RENDER_ERROR
                return self._library.processor_result_set_output(result, output)
            except Exception as error:
                self._library.processor_result_set_error(result, str(error))
                return COPPERLACE_RENDER_ERROR

        return _PROCESSOR_CALLBACK(callback)


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
