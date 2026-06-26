from __future__ import annotations

import os
import platform
import shutil
import subprocess
from pathlib import Path

from setuptools import setup
from setuptools.command.build_py import build_py


class build_py_with_rust(build_py):
    def run(self) -> None:
        self._build_native_library()
        super().run()
        self._copy_native_library()

    def _build_native_library(self) -> None:
        root = Path(__file__).resolve().parent.parent
        manifest = root / "rust-core" / "Cargo.toml"
        cargo = self._find_cargo()
        env = os.environ.copy()
        cargo_bin = Path.home() / ".cargo" / "bin"
        if cargo_bin.exists():
            env["PATH"] = f"{cargo_bin}{os.pathsep}{env.get('PATH', '')}"

        subprocess.run(
            [str(cargo), "build", "--release", "--manifest-path", str(manifest)],
            check=True,
            cwd=root,
            env=env,
        )

    def _copy_native_library(self) -> None:
        root = Path(__file__).resolve().parent.parent
        native_library = self._native_library_path(root)
        package_dir = Path(self.build_lib) / "copperlace" / "native"
        package_dir.mkdir(parents=True, exist_ok=True)
        shutil.copy2(native_library, package_dir / native_library.name)

    def _native_library_path(self, root: Path) -> Path:
        library_name = native_library_name()
        path = root / "rust-core" / "target" / "release" / library_name
        if not path.exists():
            raise RuntimeError(f"Cargo build did not produce {path}")
        return path

    def _find_cargo(self) -> Path:
        cargo = shutil.which("cargo")
        if cargo:
            return Path(cargo)

        rustup_cargo = Path.home() / ".cargo" / "bin" / "cargo"
        if rustup_cargo.exists():
            return rustup_cargo

        raise RuntimeError("cargo was not found on PATH or in ~/.cargo/bin")


def native_library_name() -> str:
    system = platform.system()
    if system == "Windows":
        return "copperlace.dll"
    if system == "Darwin":
        return "libcopperlace.dylib"
    return "libcopperlace.so"


try:
    from wheel.bdist_wheel import bdist_wheel

    class bdist_wheel_platform(bdist_wheel):
        def finalize_options(self) -> None:
            super().finalize_options()
            self.root_is_pure = False

        def get_tag(self) -> tuple[str, str, str]:
            _python, _abi, platform_tag = super().get_tag()
            return "py3", "none", platform_tag

    cmdclass = {
        "build_py": build_py_with_rust,
        "bdist_wheel": bdist_wheel_platform,
    }
except ImportError:
    cmdclass = {"build_py": build_py_with_rust}


setup(cmdclass=cmdclass)
