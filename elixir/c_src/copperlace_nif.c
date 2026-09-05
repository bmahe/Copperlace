/*
 * Copperlace NIF shim.
 *
 * Binds the shared Copperlace C ABI (libcopperlace / copperlace.dll) to the
 * BEAM through Native Implemented Functions. The native library is resolved
 * at NIF load time with dlopen (or LoadLibrary on Windows) and never linked
 * at build time, mirroring the Python ctypes and Java FFM wrappers:
 *
 *   1. COPPERLACE_LIBRARY_PATH environment variable
 *   2. packaged native library under priv/native/<libname>
 *   3. local Rust build output at ../rust-core/target/release/<libname>
 *
 * Ruleset handles are kept as NIF resource objects so the BEAM can release
 * them; the resource destructor calls copperlace_ruleset_free. Strings
 * returned by the C ABI are copied into BEAM-owned binaries and freed with
 * copperlace_string_free immediately.
 *
 * This first release supports the builtin processor registry only. Custom
 * Elixir processor callbacks are not wired through yet.
 */

#include <erl_nif.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* MSVC exposes POSIX strdup() as _strdup; remap for portability. */
#if defined(_MSC_VER)
#define strdup _strdup
#endif

#if defined(_WIN32)
#include <windows.h>
#else
#include <dlfcn.h>
#endif

/* Status codes mirrored from rust-core/src/ffi.rs. */
#define COPPERLACE_OK 0
#define COPPERLACE_INVALID_ARGUMENT 1
#define COPPERLACE_PARSE_ERROR 2
#define COPPERLACE_RENDER_ERROR 3

#define NIF_MODULE "Elixir.Copperlace.Nif"

/* C ABI function pointer types. */
typedef int (*ruleset_from_string_fn)(const char *config, void **out_handle,
                                       char **out_error);
typedef int (*ruleset_from_file_fn)(const char *path, void **out_handle,
                                     char **out_error);
typedef int (*render_ctx_opts_fn)(const void *handle, const char *rule,
                                   const char **context_keys,
                                   const char **context_values,
                                   size_t context_len,
                                   size_t max_recursion_depth,
                                   char **out_string, char **out_error);
typedef int (*render_inferred_ctx_opts_fn)(const void *handle, const char *rule,
                                            const char **context_keys,
                                            const char **context_values,
                                            size_t context_len,
                                            size_t max_recursion_depth,
                                            char **out_string, char **out_error);
typedef int (*render_structured_ctx_opts_fn)(const void *handle,
                                              const char *rule,
                                              const char **context_keys,
                                              const char **context_values,
                                              size_t context_len,
                                              bool format_json,
                                              size_t max_recursion_depth,
                                              char **out_json, char **out_error);
typedef void (*ruleset_free_fn)(void *handle);
typedef void (*string_free_fn)(char *value);

/* Global table of resolved C ABI symbols. Populated in the load callback. */
static struct {
  void *library;
  ruleset_from_string_fn ruleset_from_string;
  ruleset_from_file_fn ruleset_from_file;
  render_ctx_opts_fn render_with_context_and_options;
  render_inferred_ctx_opts_fn render_inferred_with_context_and_options;
  render_structured_ctx_opts_fn render_structured_json_with_context_and_options;
  ruleset_free_fn ruleset_free;
  string_free_fn string_free;
} copperlace;

static ErlNifResourceType *copperlace_resource_type;

typedef struct {
  void *handle;
} copperlace_resource;

static void free_context_arrays(char **keys, char **values, size_t len);

/* ------------------------------------------------------------------ */
/* Platform dynamic loading                                            */
/* ------------------------------------------------------------------ */

static void *platform_dlopen(const char *path) {
#if defined(_WIN32)
  return (void *)LoadLibraryA(path);
#else
  return dlopen(path, RTLD_NOW | RTLD_LOCAL);
#endif
}

static void *platform_dlsym(void *library, const char *symbol) {
#if defined(_WIN32)
  return (void *)GetProcAddress((HMODULE)library, symbol);
#else
  return dlsym(library, symbol);
#endif
}

static void platform_dlclose(void *library) {
#if defined(_WIN32)
  if (library != NULL) {
    FreeLibrary((HMODULE)library);
  }
#else
  if (library != NULL) {
    dlclose(library);
  }
#endif
}

/* ------------------------------------------------------------------ */
/* Native library path resolution                                      */
/* ------------------------------------------------------------------ */

static const char *native_library_name(void) {
#if defined(_WIN32)
  return "copperlace.dll";
#elif defined(__APPLE__)
  return "libcopperlace.dylib";
#else
  return "libcopperlace.so";
#endif
}

static char *probe_path(const char *dir, const char *name) {
  char candidate[4096];
  int written = snprintf(candidate, sizeof(candidate), "%s/%s", dir, name);
  if (written <= 0 || (size_t)written >= sizeof(candidate)) {
    return NULL;
  }
  FILE *probe = fopen(candidate, "rb");
  if (probe == NULL) {
    return NULL;
  }
  fclose(probe);
  return strdup(candidate);
}

/* Resolve the native library path, returning a freshly malloc'd string or
 * NULL on failure. The caller frees the result. `priv_dir` is the absolute
 * path to the app's priv directory (passed from the Elixir loader via
 * load_info), or NULL when unavailable. */
static char *resolve_library_path(const char *priv_dir) {
  const char *name = native_library_name();

  /* 1. COPPERLACE_LIBRARY_PATH environment variable. */
  const char *override = getenv("COPPERLACE_LIBRARY_PATH");
  if (override != NULL && override[0] != '\0') {
    FILE *probe = fopen(override, "rb");
    if (probe != NULL) {
      fclose(probe);
      return strdup(override);
    }
  }

  /* 2. Packaged native library under <priv_dir>/native/<name>. When the NIF
   *    is loaded as a dependency, priv_dir is the absolute path returned by
   *    :code.priv_dir(:copperlace); this is the only reliable way to find
   *    the precompiled archive's extracted native library. */
  if (priv_dir != NULL) {
    char *path = probe_path(priv_dir, "native");
    if (path != NULL) {
      char *full = probe_path(path, name);
      free(path);
      if (full != NULL) {
        return full;
      }
    }
  }

  /* Fallback: relative packaged paths for standalone builds. */
  const char *packaged_dirs[] = {"priv/native", "native", "../priv/native"};
  for (size_t i = 0; i < sizeof(packaged_dirs) / sizeof(packaged_dirs[0]); i++) {
    char *path = probe_path(packaged_dirs[i], name);
    if (path != NULL) {
      return path;
    }
  }

  /* 3. Local Rust build output for source-tree development. */
  const char *source_dirs[] = {"../rust-core/target/release",
                               "../../rust-core/target/release",
                               "rust-core/target/release"};
  for (size_t i = 0; i < sizeof(source_dirs) / sizeof(source_dirs[0]); i++) {
    char *path = probe_path(source_dirs[i], name);
    if (path != NULL) {
      return path;
    }
  }

  return NULL;
}

static int resolve_symbols(const char *priv_dir, char **error_out) {
  char *path = resolve_library_path(priv_dir);
  if (path == NULL) {
    *error_out = strdup("Could not find the Copperlace native library. Build "
                        "rust-core or set COPPERLACE_LIBRARY_PATH.");
    return -1;
  }

  copperlace.library = platform_dlopen(path);
  if (copperlace.library == NULL) {
    size_t needed = strlen(path) + 64;
    char *msg = malloc(needed);
    if (msg != NULL) {
      snprintf(msg, needed, "Failed to load Copperlace native library: %s", path);
    }
    free(path);
    *error_out = (msg != NULL) ? msg
                                : strdup("Failed to load Copperlace native library");
    return -1;
  }
  free(path);

#define RESOLVE(field, type, symbol)                                          \
  do {                                                                       \
    copperlace.field = (type)platform_dlsym(copperlace.library, symbol);     \
    if (copperlace.field == NULL) {                                          \
      *error_out = strdup("Missing Copperlace symbol: " symbol);            \
      platform_dlclose(copperlace.library);                                  \
      copperlace.library = NULL;                                             \
      return -1;                                                             \
    }                                                                        \
  } while (0)

  RESOLVE(ruleset_from_string, ruleset_from_string_fn,
          "copperlace_ruleset_from_string");
  RESOLVE(ruleset_from_file, ruleset_from_file_fn,
          "copperlace_ruleset_from_file");
  RESOLVE(render_with_context_and_options, render_ctx_opts_fn,
          "copperlace_ruleset_render_with_context_and_options");
  RESOLVE(render_inferred_with_context_and_options, render_inferred_ctx_opts_fn,
          "copperlace_ruleset_render_inferred_with_context_and_options");
  RESOLVE(render_structured_json_with_context_and_options,
          render_structured_ctx_opts_fn,
          "copperlace_ruleset_render_structured_json_with_context_and_options");
  RESOLVE(ruleset_free, ruleset_free_fn, "copperlace_ruleset_free");
  RESOLVE(string_free, string_free_fn, "copperlace_string_free");
#undef RESOLVE

  return 0;
}

/* ------------------------------------------------------------------ */
/* Helpers                                                             */
/* ------------------------------------------------------------------ */

static void copperlace_resource_destructor(ErlNifEnv *env, void *resource) {
  (void)env;
  copperlace_resource *res = (copperlace_resource *)resource;
  /* The BEAM calls the destructor only after the resource is no longer
   * reachable, so there is no concurrent access to worry about. */
  if (res->handle != NULL && copperlace.ruleset_free != NULL) {
    copperlace.ruleset_free(res->handle);
    res->handle = NULL;
  }
}

static ERL_NIF_TERM make_empty_binary(ErlNifEnv *env) {
  ErlNifBinary binary;
  if (!enif_alloc_binary(0, &binary)) {
    return enif_make_list(env, 0);
  }
  return enif_make_binary(env, &binary);
}

/* Read a C string (may be NULL) into an Elixir binary term, then free the C
 * string with copperlace_string_free. Returns the binary term. */
static ERL_NIF_TERM take_native_string(ErlNifEnv *env, char *value) {
  if (value == NULL) {
    return make_empty_binary(env);
  }
  size_t len = strlen(value);
  if (len == 0) {
    if (copperlace.string_free != NULL) {
      copperlace.string_free(value);
    }
    return make_empty_binary(env);
  }
  ErlNifBinary binary;
  if (!enif_alloc_binary(len, &binary)) {
    if (copperlace.string_free != NULL) {
      copperlace.string_free(value);
    }
    return make_empty_binary(env);
  }
  memcpy(binary.data, value, len);
  if (copperlace.string_free != NULL) {
    copperlace.string_free(value);
  }
  return enif_make_binary(env, &binary);
}

/* Build an error tuple {:error, status_atom, message_binary}. Takes ownership
 * of `message` (a C ABI-owned string freed via copperlace_string_free). */
static ERL_NIF_TERM make_error(ErlNifEnv *env, int status, char *message) {
  ERL_NIF_TERM status_atom;
  switch (status) {
    case COPPERLACE_INVALID_ARGUMENT:
      status_atom = enif_make_atom(env, "invalid_argument");
      break;
    case COPPERLACE_PARSE_ERROR:
      status_atom = enif_make_atom(env, "parse_error");
      break;
    case COPPERLACE_RENDER_ERROR:
      status_atom = enif_make_atom(env, "render_error");
      break;
    default:
      status_atom = enif_make_atom(env, "unknown");
      break;
  }
  ERL_NIF_TERM message_term = take_native_string(env, message);
  return enif_make_tuple3(env, enif_make_atom(env, "error"), status_atom,
                          message_term);
}

/* Build an error tuple {:error, status_atom, message_binary} from a C string
 * literal that is NOT owned by the C ABI. The literal is copied into a BEAM
 * binary and never freed via copperlace_string_free. */
static ERL_NIF_TERM make_error_msg(ErlNifEnv *env, int status,
                                    const char *message) {
  ERL_NIF_TERM status_atom;
  switch (status) {
    case COPPERLACE_INVALID_ARGUMENT:
      status_atom = enif_make_atom(env, "invalid_argument");
      break;
    case COPPERLACE_PARSE_ERROR:
      status_atom = enif_make_atom(env, "parse_error");
      break;
    case COPPERLACE_RENDER_ERROR:
      status_atom = enif_make_atom(env, "render_error");
      break;
    default:
      status_atom = enif_make_atom(env, "unknown");
      break;
  }
  size_t len = strlen(message);
  ErlNifBinary binary;
  ERL_NIF_TERM message_term;
  if (enif_alloc_binary(len, &binary)) {
    if (len > 0) {
      memcpy(binary.data, message, len);
    }
    message_term = enif_make_binary(env, &binary);
  } else {
    message_term = make_empty_binary(env);
  }
  return enif_make_tuple3(env, enif_make_atom(env, "error"), status_atom,
                          message_term);
}

static ERL_NIF_TERM make_ok(ErlNifEnv *env, ERL_NIF_TERM value) {
  return enif_make_tuple2(env, enif_make_atom(env, "ok"), value);
}

static ERL_NIF_TERM not_loaded_error(ErlNifEnv *env) {
  return enif_make_tuple3(
      env, enif_make_atom(env, "error"), enif_make_atom(env, "native_not_loaded"),
      enif_make_string(env,
                       "Copperlace native library not loaded; build "
                       "rust-core or set COPPERLACE_LIBRARY_PATH",
                       ERL_NIF_LATIN1));
}

static int get_string(ErlNifEnv *env, ERL_NIF_TERM term, ErlNifBinary *out) {
  return enif_inspect_iolist_as_binary(env, term, out);
}

/* Validate a context term: a map of binary -> binary. Populates parallel
 * arrays of NUL-terminated C strings. Returns true on success. */
static bool build_context(ErlNifEnv *env, ERL_NIF_TERM context_term,
                          char ***keys_out, char ***values_out, size_t *len_out,
                          ERL_NIF_TERM *error_out) {
  *keys_out = NULL;
  *values_out = NULL;
  *len_out = 0;

  if (!enif_is_map(env, context_term)) {
    *error_out =
        make_error_msg(env, COPPERLACE_INVALID_ARGUMENT, "context must be a map");
    return false;
  }

  size_t size = 0;
  enif_get_map_size(env, context_term, &size);
  if (size == 0) {
    return true;
  }

  ErlNifMapIterator iter;
  if (!enif_map_iterator_create(env, context_term, &iter,
                                ERL_NIF_MAP_ITERATOR_FIRST)) {
    *error_out = make_error_msg(env, COPPERLACE_INVALID_ARGUMENT, "out of memory");
    return false;
  }

  char **keys = calloc(size, sizeof(char *));
  char **values = calloc(size, sizeof(char *));
  if (keys == NULL || values == NULL) {
    free(keys);
    free(values);
    enif_map_iterator_destroy(env, &iter);
    *error_out = make_error_msg(env, COPPERLACE_INVALID_ARGUMENT, "out of memory");
    return false;
  }

  size_t index = 0;
  while (!enif_map_iterator_is_tail(env, &iter)) {
    ERL_NIF_TERM key_term;
    ERL_NIF_TERM value_term;
    enif_map_iterator_get_pair(env, &iter, &key_term, &value_term);

    ErlNifBinary key_bin;
    ErlNifBinary value_bin;
    if (!enif_inspect_iolist_as_binary(env, key_term, &key_bin) ||
        !enif_inspect_iolist_as_binary(env, value_term, &value_bin)) {
      free_context_arrays(keys, values, index);
      enif_map_iterator_destroy(env, &iter);
      *error_out = make_error_msg(env, COPPERLACE_INVALID_ARGUMENT,
                                   "context keys and values must be strings");
      return false;
    }

    keys[index] = malloc(key_bin.size + 1);
    values[index] = malloc(value_bin.size + 1);
    if (keys[index] == NULL || values[index] == NULL) {
      free_context_arrays(keys, values, index);
      enif_map_iterator_destroy(env, &iter);
      *error_out = make_error_msg(env, COPPERLACE_INVALID_ARGUMENT, "out of memory");
      return false;
    }
    memcpy(keys[index], key_bin.data, key_bin.size);
    keys[index][key_bin.size] = '\0';
    memcpy(values[index], value_bin.data, value_bin.size);
    values[index][value_bin.size] = '\0';

    index++;
    enif_map_iterator_next(env, &iter);
  }

  enif_map_iterator_destroy(env, &iter);
  *keys_out = keys;
  *values_out = values;
  *len_out = index;
  return true;
}

static void free_context_arrays(char **keys, char **values, size_t len) {
  if (keys != NULL) {
    for (size_t i = 0; i < len; i++) {
      free(keys[i]);
    }
    free(keys);
  }
  if (values != NULL) {
    for (size_t i = 0; i < len; i++) {
      free(values[i]);
    }
    free(values);
  }
}

/* ------------------------------------------------------------------ */
/* NIF entry points                                                    */
/* ------------------------------------------------------------------ */

static ERL_NIF_TERM alloc_ruleset_resource(ErlNifEnv *env, void *handle) {
  copperlace_resource *res = (copperlace_resource *)enif_alloc_resource(
      copperlace_resource_type, sizeof(copperlace_resource));
  if (res == NULL) {
    copperlace.ruleset_free(handle);
    return make_error_msg(env, COPPERLACE_INVALID_ARGUMENT,
                           "failed to allocate ruleset resource");
  }
  res->handle = handle;
  ERL_NIF_TERM resource_term = enif_make_resource(env, res);
  enif_release_resource(res);
  return make_ok(env, resource_term);
}

static ERL_NIF_TERM nif_from_string(ErlNifEnv *env, int argc,
                                    const ERL_NIF_TERM argv[]) {
  if (argc != 1) {
    return enif_make_badarg(env);
  }
  if (copperlace.library == NULL) {
    return not_loaded_error(env);
  }
  ErlNifBinary config_bin;
  if (!get_string(env, argv[0], &config_bin)) {
    return enif_make_badarg(env);
  }
  char *config = malloc(config_bin.size + 1);
  if (config == NULL) {
    return enif_make_badarg(env);
  }
  memcpy(config, config_bin.data, config_bin.size);
  config[config_bin.size] = '\0';

  void *handle = NULL;
  char *error = NULL;
  int status = copperlace.ruleset_from_string(config, &handle, &error);
  free(config);
  if (status != COPPERLACE_OK) {
    return make_error(env, status, error);
  }
  return alloc_ruleset_resource(env, handle);
}

static ERL_NIF_TERM nif_from_file(ErlNifEnv *env, int argc,
                                  const ERL_NIF_TERM argv[]) {
  if (argc != 1) {
    return enif_make_badarg(env);
  }
  if (copperlace.library == NULL) {
    return not_loaded_error(env);
  }
  ErlNifBinary path_bin;
  if (!get_string(env, argv[0], &path_bin)) {
    return enif_make_badarg(env);
  }
  char *path = malloc(path_bin.size + 1);
  if (path == NULL) {
    return enif_make_badarg(env);
  }
  memcpy(path, path_bin.data, path_bin.size);
  path[path_bin.size] = '\0';

  void *handle = NULL;
  char *error = NULL;
  int status = copperlace.ruleset_from_file(path, &handle, &error);
  free(path);
  if (status != COPPERLACE_OK) {
    return make_error(env, status, error);
  }
  return alloc_ruleset_resource(env, handle);
}

typedef enum {
  RENDER_TEXT,
  RENDER_INFERRED,
  RENDER_STRUCTURED
} render_mode;

static ERL_NIF_TERM do_render(ErlNifEnv *env, ERL_NIF_TERM handle_term,
                              ERL_NIF_TERM rule_term, ERL_NIF_TERM context_term,
                              size_t max_recursion_depth, render_mode mode,
                              bool format_json) {
  if (copperlace.library == NULL) {
    return not_loaded_error(env);
  }

  copperlace_resource *res = NULL;
  if (!enif_get_resource(env, handle_term, copperlace_resource_type,
                         (void **)&res) ||
      res == NULL || res->handle == NULL) {
    return make_error_msg(env, COPPERLACE_INVALID_ARGUMENT,
                           "invalid ruleset handle");
  }

  ErlNifBinary rule_bin;
  if (!get_string(env, rule_term, &rule_bin)) {
    return enif_make_badarg(env);
  }
  char *rule = malloc(rule_bin.size + 1);
  if (rule == NULL) {
    return enif_make_badarg(env);
  }
  memcpy(rule, rule_bin.data, rule_bin.size);
  rule[rule_bin.size] = '\0';

  char **keys = NULL;
  char **values = NULL;
  size_t context_len = 0;
  ERL_NIF_TERM context_error;
  if (!build_context(env, context_term, &keys, &values, &context_len,
                     &context_error)) {
    free(rule);
    return context_error;
  }

  char *output = NULL;
  char *error = NULL;
  int status;
  switch (mode) {
    case RENDER_TEXT:
      status = copperlace.render_with_context_and_options(
          res->handle, rule, (const char **)keys, (const char **)values,
          context_len, max_recursion_depth, &output, &error);
      break;
    case RENDER_INFERRED:
      status = copperlace.render_inferred_with_context_and_options(
          res->handle, rule, (const char **)keys, (const char **)values,
          context_len, max_recursion_depth, &output, &error);
      break;
    case RENDER_STRUCTURED:
      status = copperlace.render_structured_json_with_context_and_options(
          res->handle, rule, (const char **)keys, (const char **)values,
          context_len, format_json, max_recursion_depth, &output, &error);
      break;
    default:
      free(rule);
      free_context_arrays(keys, values, context_len);
      return make_error_msg(env, COPPERLACE_INVALID_ARGUMENT, "unknown render mode");
  }

  free(rule);
  free_context_arrays(keys, values, context_len);

  if (status != COPPERLACE_OK) {
    return make_error(env, status, error);
  }
  return make_ok(env, take_native_string(env, output));
}

static ERL_NIF_TERM nif_render(ErlNifEnv *env, int argc,
                               const ERL_NIF_TERM argv[]) {
  if (argc != 4) {
    return enif_make_badarg(env);
  }
  ErlNifUInt64 max_recursion;
  if (!enif_get_uint64(env, argv[3], &max_recursion)) {
    return enif_make_badarg(env);
  }
  return do_render(env, argv[0], argv[1], argv[2], (size_t)max_recursion,
                   RENDER_TEXT, false);
}

static ERL_NIF_TERM nif_render_inferred(ErlNifEnv *env, int argc,
                                        const ERL_NIF_TERM argv[]) {
  if (argc != 4) {
    return enif_make_badarg(env);
  }
  ErlNifUInt64 max_recursion;
  if (!enif_get_uint64(env, argv[3], &max_recursion)) {
    return enif_make_badarg(env);
  }
  return do_render(env, argv[0], argv[1], argv[2], (size_t)max_recursion,
                   RENDER_INFERRED, false);
}

static ERL_NIF_TERM nif_render_structured(ErlNifEnv *env, int argc,
                                           const ERL_NIF_TERM argv[]) {
  if (argc != 5) {
    return enif_make_badarg(env);
  }
  ErlNifUInt64 max_recursion;
  if (!enif_get_uint64(env, argv[3], &max_recursion)) {
    return enif_make_badarg(env);
  }
  ERL_NIF_TERM true_atom = enif_make_atom(env, "true");
  bool format_json = enif_is_identical(argv[4], true_atom) != 0;
  return do_render(env, argv[0], argv[1], argv[2], (size_t)max_recursion,
                   RENDER_STRUCTURED, format_json);
}

static ERL_NIF_TERM nif_loaded(ErlNifEnv *env, int argc,
                               const ERL_NIF_TERM argv[]) {
  (void)argc;
  (void)argv;
  return copperlace.library != NULL ? enif_make_atom(env, "true")
                                   : enif_make_atom(env, "false");
}

/* ------------------------------------------------------------------ */
/* Load / upgrade                                                      */
/* ------------------------------------------------------------------ */

static int load(ErlNifEnv *env, void **priv_data, ERL_NIF_TERM load_info) {
  /* load_info is the priv directory path (a binary) passed by the Elixir
   * loader via :erlang.load_nif/2, so we can resolve the precompiled native
   * library under <priv>/native/ regardless of the consumer's CWD. */
  char priv_dir[4096] = {0};
  ErlNifBinary priv_bin;
  if (enif_inspect_iolist_as_binary(env, load_info, &priv_bin) &&
      priv_bin.size > 0 && priv_bin.size < sizeof(priv_dir)) {
    memcpy(priv_dir, priv_bin.data, priv_bin.size);
    priv_dir[priv_bin.size] = '\0';
  }

  copperlace_resource_type = enif_open_resource_type(
      env, NIF_MODULE, "copperlace_resource", copperlace_resource_destructor,
      ERL_NIF_RT_CREATE, NULL);
  if (copperlace_resource_type == NULL) {
    return -1;
  }

  memset(&copperlace, 0, sizeof(copperlace));
  char *error = NULL;
  const char *dir = priv_dir[0] != '\0' ? priv_dir : NULL;
  if (resolve_symbols(dir, &error) != 0) {
    /* The native library may be absent while compiling the NIF (for example
     * `mix compile` in CI without a Rust build). We do not fail the load:
     * callers get a clear error when they invoke a NIF, and tests are
     * expected to run after `make rust-build`. */
    if (error != NULL) {
      fprintf(stderr, "copperlace_nif: %s\n", error);
      free(error);
    }
  }

  *priv_data = NULL;
  return 0;
}

static int upgrade(ErlNifEnv *env, void **priv_data, void **old_priv_data,
                   ERL_NIF_TERM load_info) {
  (void)old_priv_data;
  return load(env, priv_data, load_info);
}

static void unload(ErlNifEnv *env, void *priv_data) {
  (void)env;
  (void)priv_data;
  platform_dlclose(copperlace.library);
  memset(&copperlace, 0, sizeof(copperlace));
}

static ErlNifFunc nif_funcs[] = {
    {"from_string_raw", 1, nif_from_string, ERL_NIF_DIRTY_JOB_CPU_BOUND},
    {"from_file_raw", 1, nif_from_file, ERL_NIF_DIRTY_JOB_CPU_BOUND},
    {"render_raw", 4, nif_render, ERL_NIF_DIRTY_JOB_CPU_BOUND},
    {"render_inferred_raw", 4, nif_render_inferred, ERL_NIF_DIRTY_JOB_CPU_BOUND},
    {"render_structured_raw", 5, nif_render_structured,
     ERL_NIF_DIRTY_JOB_CPU_BOUND},
    {"loaded", 0, nif_loaded, 0},
};

ERL_NIF_INIT(Elixir.Copperlace.Nif, nif_funcs, load, NULL, upgrade, unload)
