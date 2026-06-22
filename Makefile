.DEFAULT_GOAL := help

CARGO ?= cargo
MVN ?= mvn
PYTHON ?= python

RUST_DIR := rust-core
PYTHON_DIR := python
JAVA_DIR := java

.PHONY: help
help:
	@printf '%s\n' 'Copperlace development targets:'
	@printf '%s\n' '  make rust-fmt       Check Rust formatting'
	@printf '%s\n' '  make rust-build     Build Rust library, native library, and CLI'
	@printf '%s\n' '  make rust-test      Run Rust tests'
	@printf '%s\n' '  make rust-cli       Run the sample CLI render command'
	@printf '%s\n' '  make python-test    Run Python wrapper tests'
	@printf '%s\n' '  make python-wheel   Build the Python wheel'
	@printf '%s\n' '  make java-test      Run Java FFM tests'
	@printf '%s\n' '  make java-package   Build Java API and native classifier JARs'
	@printf '%s\n' '  make test           Run Rust, Python, and Java tests'
	@printf '%s\n' '  make package        Build Python and Java distributable artifacts'
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

.PHONY: python-test
python-test:
	PYTHONPATH=$(PYTHON_DIR) $(PYTHON) -m unittest discover -s $(PYTHON_DIR)/tests

.PHONY: python-wheel
python-wheel:
	cd $(PYTHON_DIR) && $(PYTHON) -m build --wheel

.PHONY: java-test
java-test:
	cd $(JAVA_DIR) && $(MVN) -q test

.PHONY: java-package
java-package:
	cd $(JAVA_DIR) && $(MVN) -q package

.PHONY: test
test: rust-test python-test java-test

.PHONY: package
package: python-wheel java-package

.PHONY: check
check: rust-fmt test

.PHONY: clean
clean:
	cd $(RUST_DIR) && $(CARGO) clean
	rm -rf $(PYTHON_DIR)/build $(PYTHON_DIR)/dist $(PYTHON_DIR)/*.egg-info
	find $(PYTHON_DIR) -type d -name __pycache__ -prune -exec rm -rf {} +
	cd $(JAVA_DIR) && $(MVN) -q clean
