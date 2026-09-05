# Makefile for the Copperlace NIF shim.
#
# Invoked by `elixir_make` during `mix compile`. Produces the NIF shared
# library under ../priv/. The Copperlace native library is NOT linked here:
# the NIF resolves it at runtime via dlopen/LoadLibrary (see
# copperlace_nif.c), mirroring the Python ctypes and Java FFM wrappers.
#
# This Makefile follows the elixir_make convention: the default target builds
# the NIF, and `clean` removes it.

# elixir_make supplies MIX_ENV and the project root via the working directory.
ERL ?= erl
CC ?= cc

# Locate the BEAM ERTS include directory (where erl_nif.h lives). We compute
# this directly rather than reusing ERL_EI_INCLUDE_DIR, which elixir_make
# points at the erl_interface include dir (no erl_nif.h there). elixir_make
# builds run from c_src/, so the NIF's working directory is this folder.
ERTS_INCLUDE_DIR ?= $(shell $(ERL) -noshell -s init stop -eval 'io:format("~s/erts-~s/include", [code:root_dir(), erlang:system_info(version)]).' 2>/dev/null | tr -d '\r\n')

# elixir_make sets MIX_APP_PATH; the NIF artifact belongs under that app's
# priv/ so :code.priv_dir(:copperlace) finds it. Fall back to ../priv for
# standalone `make` invocations from c_src/.
PRIV_DIR = $(if $(MIX_APP_PATH),$(MIX_APP_PATH)/priv,../priv)
NIF_NAME = copperlace_nif

UNAME_S := $(shell uname -s 2>/dev/null)

ifeq ($(OS),Windows_NT)
	NIF_EXT = dll
	CFLAGS ?= -O2 -Wall -I"$(ERTS_INCLUDE_DIR)"
	LDFLAGS ?= -shared
else
	NIF_EXT = so
	CFLAGS ?= -O2 -fPIC -Wall -Wextra -I"$(ERTS_INCLUDE_DIR)"
	LDFLAGS ?= -shared -fPIC
	ifeq ($(UNAME_S),Darwin)
		LDFLAGS += -undefined dynamic_lookup
	endif
endif

# dlopen/dlsym are in the libc on Linux/macOS; LoadLibrary in kernel32 on Windows.
ifeq ($(OS),Windows_NT)
	LDLIBS ?=
	RENDERER_NAME = copperlace.dll
else
	LDLIBS ?= -ldl
	ifeq ($(UNAME_S),Darwin)
		RENDERER_NAME = libcopperlace.dylib
	else
		RENDERER_NAME = libcopperlace.so
	endif
endif

NIF = $(PRIV_DIR)/$(NIF_NAME).$(NIF_EXT)
SRC = copperlace_nif.c
SOURCE_PRIV = ../priv
STAGED_NATIVE = $(SOURCE_PRIV)/native/$(RENDERER_NAME)

.PHONY: all clean stage_native

all: $(NIF) stage_native

$(NIF): $(SRC) | $(PRIV_DIR)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $(NIF) $(SRC) $(LDLIBS)

# Copy the staged Copperlace native library from the source priv into the
# build priv so it is included in precompiled archives. This is a no-op
# during normal dev/test builds (the source file does not exist; the NIF
# resolves it at runtime via dlopen).
stage_native: | $(PRIV_DIR)/native
	@if [ -f "$(STAGED_NATIVE)" ] && [ "$(STAGED_NATIVE)" != "$(PRIV_DIR)/native/$(RENDERER_NAME)" ]; then \
		cp "$(STAGED_NATIVE)" "$(PRIV_DIR)/native/$(RENDERER_NAME)"; \
		echo "Staged $(RENDERER_NAME) into $(PRIV_DIR)/native/"; \
	fi

$(PRIV_DIR)/native:
	mkdir -p $@

$(PRIV_DIR):
	mkdir -p $(PRIV_DIR)

clean:
	rm -f $(NIF)
	rm -rf $(PRIV_DIR)/native
