from __future__ import annotations

import ctypes
from collections.abc import Callable, Mapping
from pathlib import Path
from types import TracebackType
from typing import Self

from ._native import NativeError, native


class CopperlaceError(RuntimeError):
    """Raised when Copperlace cannot parse config, render a rule, or call native code."""

    pass


class RuleSet:
    """Compiled Copperlace rules loaded from configuration.

    Use :meth:`from_string` or :meth:`from_file` to compile config once, then
    call :meth:`render` repeatedly. ``RuleSet`` owns a native handle, so callers
    should use it as a context manager or call :meth:`close` when finished.
    """

    def __init__(self, handle: ctypes.c_void_p, processor_registry: object | None = None) -> None:
        self._handle = handle
        self._processor_registry = processor_registry
        self._closed = False

    @classmethod
    def from_string(
        cls,
        config: str,
        processors: Mapping[str, Callable[[str], str]] | None = None,
    ) -> Self:
        """Compile a ruleset from a configuration string.

        Args:
            config: configuration text containing Copperlace rules.
            processors: Optional custom processor callbacks.

        Returns:
            A ``RuleSet`` backed by a native Copperlace handle.

        Raises:
            CopperlaceError: If the config cannot be parsed or compiled.
        """

        try:
            handle, processor_registry = native().ruleset_from_string(
                config, _validate_processors(processors)
            )
            return cls(handle, processor_registry)
        except NativeError as error:
            raise CopperlaceError(str(error)) from error

    @classmethod
    def from_file(
        cls,
        path: str | Path,
        processors: Mapping[str, Callable[[str], str]] | None = None,
    ) -> Self:
        """Compile a ruleset from a configuration file.

        Args:
            path: Path to the configuration file.
            processors: Optional custom processor callbacks.

        Returns:
            A ``RuleSet`` backed by a native Copperlace handle.

        Raises:
            CopperlaceError: If the file cannot be loaded, parsed, or compiled.
        """

        try:
            handle, processor_registry = native().ruleset_from_file(
                path, _validate_processors(processors)
            )
            return cls(handle, processor_registry)
        except NativeError as error:
            raise CopperlaceError(str(error)) from error

    def render(self, rule: str, context: Mapping[str, str] | None = None) -> str:
        """Render a named rule from this ruleset.

        Each call uses a fresh render context, so per-render bindings are
        consistent within one output but do not carry over to later renders.
        ``context`` provides initial string bindings for this render only.

        Args:
            rule: Name of the rule to render.
            context: Optional initial render context values.

        Returns:
            Rendered text for the requested rule.

        Raises:
            CopperlaceError: If this ruleset is closed or rendering fails.
        """

        self._ensure_open()
        if context is not None:
            validated_context = _validate_context(context)
            try:
                return native().ruleset_render_with_context(
                    self._handle, rule, validated_context
                )
            except NativeError as error:
                raise CopperlaceError(str(error)) from error

        try:
            return native().ruleset_render(self._handle, rule)
        except NativeError as error:
            raise CopperlaceError(str(error)) from error

    def close(self) -> None:
        """Release this ruleset's native handle.

        Calling ``close`` more than once is allowed. Rendering after close raises
        ``CopperlaceError``.
        """

        if not self._closed:
            native().ruleset_free(self._handle)
            self._closed = True
            self._processor_registry = None
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


class Copperlace:
    """Load-once Copperlace renderer for repeated renders.

    ``Copperlace`` is the recommended high-level API when rendering multiple
    rules or rendering the same rule multiple times from one config. It wraps a
    ``RuleSet`` and can be used as a context manager.
    """

    def __init__(self, ruleset: RuleSet) -> None:
        self._ruleset = ruleset

    @classmethod
    def from_string(
        cls,
        config: str,
        processors: Mapping[str, Callable[[str], str]] | None = None,
    ) -> Self:
        """Create a renderer from a configuration string.

        Args:
            config: configuration text containing Copperlace rules.
            processors: Optional custom processor callbacks.

        Returns:
            A ``Copperlace`` renderer that can render rules repeatedly.

        Raises:
            CopperlaceError: If the config cannot be parsed or compiled.
        """

        return cls(RuleSet.from_string(config, processors))

    @classmethod
    def from_file(
        cls,
        path: str | Path,
        processors: Mapping[str, Callable[[str], str]] | None = None,
    ) -> Self:
        """Create a renderer from a configuration file.

        Args:
            path: Path to the configuration file.
            processors: Optional custom processor callbacks.

        Returns:
            A ``Copperlace`` renderer that can render rules repeatedly.

        Raises:
            CopperlaceError: If the file cannot be loaded, parsed, or compiled.
        """

        return cls(RuleSet.from_file(path, processors))

    def render(self, rule: str, context: Mapping[str, str] | None = None) -> str:
        """Render a named rule from the loaded config.

        Args:
            rule: Name of the rule to render.
            context: Optional initial render context values.

        Returns:
            Rendered text for the requested rule.

        Raises:
            CopperlaceError: If the renderer is closed or rendering fails.
        """

        return self._ruleset.render(rule, context)

    def close(self) -> None:
        """Release the underlying native ruleset handle."""

        self._ruleset.close()

    def __enter__(self) -> Self:
        self._ruleset._ensure_open()
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


def render_str(
    config: str,
    rule: str,
    context: Mapping[str, str] | None = None,
    *,
    processors: Mapping[str, Callable[[str], str]] | None = None,
) -> str:
    """Render one rule from a configuration string.

    This convenience helper compiles the config, renders one rule, and releases
    the native handle. Use ``Copperlace.from_string`` for repeated renders.

    Args:
        config: configuration text containing Copperlace rules.
        rule: Name of the rule to render.
        context: Optional initial render context values.
        processors: Optional custom processor callbacks.

    Returns:
        Rendered text for the requested rule.

    Raises:
        CopperlaceError: If parsing, compilation, or rendering fails.
    """

    with RuleSet.from_string(config, processors) as ruleset:
        return ruleset.render(rule, context)


def render_file(
    path: str | Path,
    rule: str,
    context: Mapping[str, str] | None = None,
    *,
    processors: Mapping[str, Callable[[str], str]] | None = None,
) -> str:
    """Render one rule from a configuration file.

    This convenience helper loads the file, renders one rule, and releases the
    native handle. Use ``Copperlace.from_file`` for repeated renders.

    Args:
        path: Path to the configuration file.
        rule: Name of the rule to render.
        context: Optional initial render context values.
        processors: Optional custom processor callbacks.

    Returns:
        Rendered text for the requested rule.

    Raises:
        CopperlaceError: If loading, parsing, compilation, or rendering fails.
    """

    with RuleSet.from_file(path, processors) as ruleset:
        return ruleset.render(rule, context)


def _validate_context(context: Mapping[str, str]) -> dict[str, str]:
    validated = dict[str, str]()
    for key, value in context.items():
        if not isinstance(key, str):
            raise TypeError("context keys must be strings")
        if not isinstance(value, str):
            raise TypeError("context values must be strings")
        validated[key] = value
    return validated


def _validate_processors(
    processors: Mapping[str, Callable[[str], str]] | None,
) -> dict[str, Callable[[str], str]] | None:
    if processors is None:
        return None
    validated = dict[str, Callable[[str], str]]()
    for name, processor in processors.items():
        if not isinstance(name, str):
            raise TypeError("processor names must be strings")
        if not callable(processor):
            raise TypeError("processors must be callable")
        validated[name] = processor
    return validated
