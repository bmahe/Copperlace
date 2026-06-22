from __future__ import annotations

import ctypes
from pathlib import Path
from types import TracebackType
from typing import Self

from ._native import NativeError, native


class CopperlaceError(RuntimeError):
    pass


class RuleSet:
    def __init__(self, handle: ctypes.c_void_p) -> None:
        self._handle = handle
        self._closed = False

    @classmethod
    def from_string(cls, config: str) -> Self:
        try:
            return cls(native().ruleset_from_string(config))
        except NativeError as error:
            raise CopperlaceError(str(error)) from error

    @classmethod
    def from_file(cls, path: str | Path) -> Self:
        try:
            return cls(native().ruleset_from_file(path))
        except NativeError as error:
            raise CopperlaceError(str(error)) from error

    def render(self, rule: str) -> str:
        self._ensure_open()
        try:
            return native().ruleset_render(self._handle, rule)
        except NativeError as error:
            raise CopperlaceError(str(error)) from error

    def close(self) -> None:
        if not self._closed:
            native().ruleset_free(self._handle)
            self._closed = True
            self._handle = ctypes.c_void_p()

    def __enter__(self) -> Self:
        self._ensure_open()
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        traceback: TracebackType | None,
    ) -> None:
        self.close()

    def __del__(self) -> None:
        try:
            self.close()
        except Exception:
            pass

    def _ensure_open(self) -> None:
        if self._closed:
            raise CopperlaceError("RuleSet is closed")


def render_hocon_str(config: str, rule: str) -> str:
    with RuleSet.from_string(config) as ruleset:
        return ruleset.render(rule)


def render_hocon_file(path: str | Path, rule: str) -> str:
    with RuleSet.from_file(path) as ruleset:
        return ruleset.render(rule)
