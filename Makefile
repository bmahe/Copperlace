.DEFAULT_GOAL := help

CARGO ?= cargo
MVN ?= mvn
PYTHON ?= python
WASM_PACK ?= wasm-pack

RUST_DIR := rust-core
PYTHON_DIR := python
JAVA_DIR := java
JS_DIR := js

.PHONY: help
help:
	@printf '%s\n' 'Copperlace development targets:'
	@printf '%s\n' '  make rust-fmt       Check Rust formatting'
	@printf '%s\n' '  make rust-build     Build Rust library, native library, and CLI'
	@printf '%s\n' '  make rust-test      Run Rust tests'
	@printf '%s\n' '  make rust-cli       Run the sample CLI render command'
	@printf '%s\n' '  make cli-archive    Build a CLI and native-library release archive'
	@printf '%s\n' '  make python-test    Run Python wrapper tests'
	@printf '%s\n' '  make python-wheel   Build the Python wheel'
	@printf '%s\n' '  make js-package     Build JS/TS WebAssembly package for bundlers'
	@printf '%s\n' '  make js-web         Build JS/TS WebAssembly package for direct browser import'
	@printf '%s\n' '  make java-test      Run Java FFM tests'
	@printf '%s\n' '  make java-package   Build Java API and native classifier JARs'
	@printf '%s\n' '  make site           Build website and native API documentation'
	@printf '%s\n' '  make site-main      Build website pages from AsciiDoc sources'
	@printf '%s\n' '  make site-api       Build native API documentation sub-sites'
	@printf '%s\n' '  make site-serve     Serve generated website locally'
	@printf '%s\n' '  make test           Run Rust, Python, and Java tests'
	@printf '%s\n' '  make package        Build Python, JS/TS, and Java distributable artifacts'
	@printf '%s\n' '  make release-check  Check package version metadata consistency'
	@printf '%s\n' '  make check          Run formatting checks and tests'
	@printf '%s\n' '  make clean          Remove build outputs'

.PHONY: rust-fmt
rust-fmt:
	cd $(RUST_DIR) && $(CARGO) fmt --check

.PHONY: rust-build
rust-build:
	cd $(RUST_DIR) && $(CARGO) build --release

.PHONY: rust-test
rust-test:
	cd $(RUST_DIR) && $(CARGO) test

.PHONY: rust-cli
rust-cli:
	cd $(RUST_DIR) && $(CARGO) run --bin copperlace -- render --config example.conf --rule origin

.PHONY: cli-archive
cli-archive: rust-build
	$(PYTHON) scripts/package_cli.py --output-dir target/release-artifacts

.PHONY: python-test
python-test: rust-build
	PYTHONPATH=$(PYTHON_DIR) $(PYTHON) -m unittest discover -s $(PYTHON_DIR)/tests

.PHONY: python-wheel
python-wheel:
	cd $(PYTHON_DIR) && $(PYTHON) -m build --wheel

.PHONY: js-package
js-package:
	$(WASM_PACK) build $(RUST_DIR) --target bundler --out-dir ../$(JS_DIR)/pkg

.PHONY: js-web
js-web:
	$(WASM_PACK) build $(RUST_DIR) --target web --out-dir ../$(JS_DIR)/pkg

.PHONY: java-test
java-test:
	cd $(JAVA_DIR) && $(MVN) -q test

.PHONY: java-package
java-package:
	cd $(JAVA_DIR) && $(MVN) -q package

.PHONY: site
site:
	$(PYTHON) website/build_site.py --clean

.PHONY: site-main
site-main: js-web
	$(PYTHON) website/build_site.py --clean --main

.PHONY: site-api
site-api:
	$(PYTHON) website/build_site.py --api

.PHONY: site-serve
site-serve: site-main
	cd target/site && $(PYTHON) -m http.server 8000

.PHONY: test
test: rust-test python-test java-test

.PHONY: package
package: cli-archive python-wheel js-package java-package

.PHONY: release-check
release-check:
	$(PYTHON) scripts/check_versions.py

.PHONY: check
check: rust-fmt test

.PHONY: clean
clean:
	cd $(RUST_DIR) && $(CARGO) clean
	rm -rf target/release-artifacts
	rm -rf $(PYTHON_DIR)/build $(PYTHON_DIR)/dist $(PYTHON_DIR)/*.egg-info
	rm -rf $(JS_DIR)/pkg
	find $(PYTHON_DIR) -type d -name __pycache__ -prune -exec rm -rf {} +
	cd $(JAVA_DIR) && $(MVN) -q clean
