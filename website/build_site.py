#!/usr/bin/env python3
from __future__ import annotations

import argparse
import html
import os
import shutil
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SITE = ROOT / "target" / "site"
BUILD = ROOT / "target" / "site-build"

MAIN_PAGES = [
    ("index", ROOT / "website" / "index.adoc", SITE / "index.html"),
    ("wrappers", ROOT / "website" / "wrappers.adoc", SITE / "wrappers.html"),
    ("docs", ROOT / "docs" / "index.adoc", SITE / "docs" / "index.html"),
    ("docs", ROOT / "docs" / "capabilities.adoc", SITE / "docs" / "capabilities.html"),
    ("docs", ROOT / "docs" / "configuration.adoc", SITE / "docs" / "configuration.html"),
    ("docs", ROOT / "docs" / "errors.adoc", SITE / "docs" / "errors.html"),
    ("docs", ROOT / "docs" / "packaging.adoc", SITE / "docs" / "packaging.html"),
    ("examples", ROOT / "examples" / "README.adoc", SITE / "examples" / "index.html"),
    ("wrappers", ROOT / "js" / "README.adoc", SITE / "wrappers" / "js.html"),
]

NAV = [
    ("Home", "/index.html"),
    ("Docs", "/docs/index.html"),
    ("Examples", "/examples/index.html"),
    ("Wrappers", "/wrappers.html"),
    ("Rust API", "/api/rust/index.html"),
    ("Python API", "/api/python/index.html"),
    ("Java API", "/api/java/index.html"),
    ("JS/TS API", "/api/js/index.html"),
]


def main() -> int:
    parser = argparse.ArgumentParser(description="Build the Copperlace website")
    parser.add_argument("--main", action="store_true", help="build the main AsciiDoc site")
    parser.add_argument("--api", action="store_true", help="build generated API docs")
    parser.add_argument("--clean", action="store_true", help="remove generated site output first")
    args = parser.parse_args()

    build_main = args.main or not args.api
    build_api = args.api or not args.main

    if args.clean and SITE.exists():
        shutil.rmtree(SITE)

    SITE.mkdir(parents=True, exist_ok=True)
    copy_assets()

    if build_main:
        build_main_site()
    if build_api:
        build_api_docs()

    return 0


def copy_assets() -> None:
    shutil.copy2(ROOT / "website" / "site.css", SITE / "site.css")


def build_main_site() -> None:
    for section, source, output in MAIN_PAGES:
        body = asciidoctor_body(source)
        if source == ROOT / "examples" / "README.adoc":
            body += examples_section()
        body = rewrite_links(body)
        write_page(output, page_title(source), body, section)

    build_example_sources()


def build_api_docs() -> None:
    build_rust_docs()
    build_python_docs()
    build_java_docs()
    build_js_docs()


def asciidoctor_body(source: Path) -> str:
    result = subprocess.run(
        ["asciidoctor", "-s", "-o", "-", str(source)],
        cwd=ROOT,
        check=True,
        text=True,
        capture_output=True,
    )
    return result.stdout


def page_title(source: Path) -> str:
    for line in source.read_text(encoding="utf-8").splitlines():
        if line.startswith("= "):
            return line[2:].strip()
    return "Copperlace"


def write_page(output: Path, title: str, body: str, section: str) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    relative_root = relative_prefix(output)
    nav = "\n".join(
        f'<a href="{relative_root}{href.lstrip("/")}">{html.escape(label)}</a>'
        for label, href in NAV
    )
    html_text = f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{html.escape(title)} | Copperlace</title>
  <link rel="stylesheet" href="{relative_root}site.css">
</head>
<body>
  <header class="site-header">
    <div class="site-header-inner">
      <a class="brand" href="{relative_root}index.html">Copperlace</a>
      <nav class="top-nav" aria-label="Primary navigation">
        {nav}
      </nav>
    </div>
  </header>
  <div class="site-shell">
    <aside class="side-nav" aria-label="Section navigation">
      <p class="side-nav-title">{html.escape(section)}</p>
      {nav}
    </aside>
    <main class="content">
      <h1>{html.escape(title)}</h1>
      {body}
    </main>
  </div>
  <footer class="site-footer">
    Generated from repository documentation and native API docs.
  </footer>
</body>
</html>
"""
    output.write_text(html_text, encoding="utf-8")


def relative_prefix(output: Path) -> str:
    parent = output.parent
    relative = os.path.relpath(SITE, parent).replace(os.sep, "/")
    if relative == ".":
        return ""
    return f"{relative}/"


def rewrite_links(body: str) -> str:
    replacements = {
        'href="capabilities.adoc"': 'href="capabilities.html"',
        'href="configuration.adoc"': 'href="configuration.html"',
        'href="errors.adoc"': 'href="errors.html"',
        'href="packaging.adoc"': 'href="packaging.html"',
        'href="docs/index.adoc"': 'href="docs/index.html"',
        'href="docs/capabilities.adoc"': 'href="docs/capabilities.html"',
        'href="docs/configuration.adoc"': 'href="docs/configuration.html"',
        'href="docs/errors.adoc"': 'href="docs/errors.html"',
        'href="docs/packaging.adoc"': 'href="docs/packaging.html"',
        'href="examples/README.adoc"': 'href="examples/index.html"',
    }
    for old, new in replacements.items():
        body = body.replace(old, new)
    return body


def examples_section() -> str:
    cards = []
    for config in sorted((ROOT / "examples").glob("*.conf")):
        stem = config.stem.replace("_", " ").title()
        cards.append(
            f"""<div class="example-card">
  <strong>{html.escape(stem)}</strong>
  <a href="conf/{config.name}.html">View source</a><br>
  <a href="conf/{config.name}">Download .conf</a>
</div>"""
        )
    return f"""
<h2 id="runnable-config-sources">Runnable Config Sources</h2>
<div class="example-grid">
{''.join(cards)}
</div>
"""


def build_example_sources() -> None:
    output_dir = SITE / "examples" / "conf"
    output_dir.mkdir(parents=True, exist_ok=True)
    for config in sorted((ROOT / "examples").glob("*.conf")):
        text = config.read_text(encoding="utf-8")
        body = (
            f'<p><a href="{config.name}">Download raw config</a></p>'
            f"<pre><code>{html.escape(text)}</code></pre>"
        )
        write_page(output_dir / f"{config.name}.html", config.name, body, "examples")
        shutil.copy2(config, output_dir / config.name)


def build_rust_docs() -> None:
    target_dir = BUILD / "rustdoc"
    subprocess.run(
        [
            "cargo",
            "doc",
            "--no-deps",
            "--manifest-path",
            str(ROOT / "rust-core" / "Cargo.toml"),
            "--target-dir",
            str(target_dir),
        ],
        cwd=ROOT,
        check=True,
    )
    copy_tree(target_dir / "doc", SITE / "api" / "rust")
    write_api_index(
        SITE / "api" / "rust" / "index.html",
        "Rust API Docs",
        [("copperlace crate", "copperlace/index.html")],
    )


def build_python_docs() -> None:
    output = SITE / "api" / "python"
    output.mkdir(parents=True, exist_ok=True)
    temp = BUILD / "pydoc"
    if temp.exists():
        shutil.rmtree(temp)
    temp.mkdir(parents=True)
    env = os.environ.copy()
    env["PYTHONPATH"] = str(ROOT / "python")
    subprocess.run(
        [
            sys.executable,
            "-m",
            "pydoc",
            "-w",
            "copperlace",
            "copperlace.core",
            "copperlace._native",
        ],
        cwd=temp,
        env=env,
        check=True,
    )
    for generated in temp.glob("*.html"):
        shutil.copy2(generated, output / generated.name)
    write_api_index(
        output / "index.html",
        "Python API Docs",
        [
            ("copperlace package", "copperlace.html"),
            ("copperlace.core module", "copperlace.core.html"),
            ("copperlace._native module", "copperlace._native.html"),
        ],
    )


def build_java_docs() -> None:
    java_site = ROOT / "java" / "target" / "site"
    java_apidocs = ROOT / "java" / "target" / "reports" / "apidocs"
    if java_site.exists():
        shutil.rmtree(java_site)
    if java_apidocs.exists():
        shutil.rmtree(java_apidocs)
    subprocess.run(["mvn", "site", "-DgenerateReports=false"], cwd=ROOT / "java", check=True)
    subprocess.run(["mvn", "javadoc:javadoc"], cwd=ROOT / "java", check=True)
    java_output = SITE / "api" / "java"
    if java_output.exists():
        shutil.rmtree(java_output)
    java_output.mkdir(parents=True)
    copy_tree(java_site, java_output / "site")
    copy_tree(java_apidocs, java_output / "apidocs")
    write_api_index(
        java_output / "index.html",
        "Java API Docs",
        [("Javadocs", "apidocs/index.html")],
    )


def build_js_docs() -> None:
    subprocess.run(
        [
            "wasm-pack",
            "build",
            str(ROOT / "rust-core"),
            "--target",
            "web",
            "--out-dir",
            str(ROOT / "js" / "pkg"),
        ],
        cwd=ROOT,
        check=True,
    )
    output = SITE / "api" / "js"
    output.mkdir(parents=True, exist_ok=True)
    body = asciidoctor_body(ROOT / "js" / "README.adoc")
    declarations = ROOT / "js" / "pkg" / "copperlace.d.ts"
    if declarations.exists():
        body += "<h2>Generated TypeScript Declarations</h2>"
        body += f"<pre><code>{html.escape(declarations.read_text(encoding='utf-8'))}</code></pre>"
    write_page(output / "index.html", "JS/TS API Docs", rewrite_links(body), "api")


def write_api_index(output: Path, title: str, links: list[tuple[str, str]]) -> None:
    items = "\n".join(
        f'<li><a href="{html.escape(href)}">{html.escape(label)}</a></li>'
        for label, href in links
    )
    write_page(output, title, f"<ul>{items}</ul>", "api")


def copy_tree(source: Path, destination: Path) -> None:
    if destination.exists():
        shutil.rmtree(destination)
    shutil.copytree(source, destination)


if __name__ == "__main__":
    raise SystemExit(main())
