use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

use crate::config::{ConfigError, ruleset_from_file, ruleset_from_str};
use crate::render::{Processor, ProcessorRegistry, RenderContext, RuleSet};

/// Status code for a successful C ABI call.
pub const COPPERLACE_OK: c_int = 0;
/// Status code for invalid C ABI arguments such as null required pointers.
pub const COPPERLACE_INVALID_ARGUMENT: c_int = 1;
/// Status code for config loading, parsing, or compilation failures.
pub const COPPERLACE_PARSE_ERROR: c_int = 2;
/// Status code for rule rendering failures.
pub const COPPERLACE_RENDER_ERROR: c_int = 3;

/// Host callback used by custom C ABI processors.
///
/// The callback receives a UTF-8 input string, an opaque result handle, and the
/// user data pointer provided when creating the ruleset. It should set either
/// output or error on `result` and return [`COPPERLACE_OK`] on success.
pub type CopperlaceProcessorCallback =
    unsafe extern "C" fn(*const c_char, *mut CopperlaceProcessorResult, *mut c_void) -> c_int;

/// Opaque C ABI handle for a compiled Copperlace ruleset.
///
/// Handles are allocated by `copperlace_ruleset_from_file` or
/// `copperlace_ruleset_from_string` and must be released with
/// `copperlace_ruleset_free`.
pub struct CopperlaceRuleSet {
    ruleset: RuleSet,
}

/// Opaque C ABI result handle passed to custom processor callbacks.
pub struct CopperlaceProcessorResult {
    output: Option<String>,
    error: Option<String>,
}

struct CallbackProcessor {
    callback: CopperlaceProcessorCallback,
    user_data: *mut c_void,
}

unsafe impl Send for CallbackProcessor {}
unsafe impl Sync for CallbackProcessor {}

impl Processor for CallbackProcessor {
    fn process(&self, value: &str) -> Result<String, String> {
        let input =
            CString::new(value).map_err(|_| "processor input contains an interior NUL byte")?;
        let mut result = CopperlaceProcessorResult {
            output: None,
            error: None,
        };
        let status = unsafe { (self.callback)(input.as_ptr(), &mut result, self.user_data) };

        if let Some(error) = result.error {
            return Err(error);
        }
        if status != COPPERLACE_OK {
            return Err(format!("processor callback failed with status {status}"));
        }
        result
            .output
            .ok_or_else(|| "processor callback did not set output".to_string())
    }
}

/// Loads a configuration file and returns an opaque ruleset handle.
///
/// On success, writes a non-null handle to `out_handle` and returns
/// [`COPPERLACE_OK`]. On failure, writes null to `out_handle`, writes an owned
/// error string to `out_error` when provided, and returns a nonzero status code.
/// Returned error strings must be released with `copperlace_string_free`.
///
/// # Safety
///
/// `path` must point to a valid NUL-terminated UTF-8 string. `out_handle` must
/// be a valid writable pointer when non-null. `out_error` must be valid for
/// writing when non-null, and any returned error string must be released with
/// [`copperlace_string_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn copperlace_ruleset_from_file(
    path: *const c_char,
    out_handle: *mut *mut CopperlaceRuleSet,
    out_error: *mut *mut c_char,
) -> c_int {
    clear_out_error(out_error);

    let Some(path) = read_c_string(path, out_error) else {
        write_null_handle(out_handle);
        return COPPERLACE_INVALID_ARGUMENT;
    };

    match ruleset_from_file(path) {
        Ok(ruleset) => write_handle(ruleset, out_handle, out_error),
        Err(error) => {
            write_null_handle(out_handle);
            write_out_string(out_error, &error.to_string());
            COPPERLACE_PARSE_ERROR
        }
    }
}

/// Loads a configuration file and returns a ruleset handle with custom processors.
///
/// # Safety
///
/// `path` must point to a valid NUL-terminated UTF-8 string. When
/// `processor_len` is nonzero, `processor_names`, `processor_callbacks`, and
/// `processor_user_data` must each point to arrays with at least
/// `processor_len` entries. Processor names must point to valid NUL-terminated
/// UTF-8 strings. `out_handle` must be a valid writable pointer when non-null.
/// `out_error` must be valid for writing when non-null, and any returned error
/// string must be released with [`copperlace_string_free`]. Processor callbacks
/// and user data must remain valid until the returned ruleset handle is freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn copperlace_ruleset_from_file_with_processors(
    path: *const c_char,
    processor_names: *const *const c_char,
    processor_callbacks: *const Option<CopperlaceProcessorCallback>,
    processor_user_data: *const *mut c_void,
    processor_len: usize,
    out_handle: *mut *mut CopperlaceRuleSet,
    out_error: *mut *mut c_char,
) -> c_int {
    clear_out_error(out_error);

    let Some(path) = read_c_string(path, out_error) else {
        write_null_handle(out_handle);
        return COPPERLACE_INVALID_ARGUMENT;
    };
    let Some(processors) = read_processors(
        processor_names,
        processor_callbacks,
        processor_user_data,
        processor_len,
        out_error,
    ) else {
        write_null_handle(out_handle);
        return COPPERLACE_INVALID_ARGUMENT;
    };

    match ruleset_from_file_with_processors(path, processors) {
        Ok(ruleset) => write_handle(ruleset, out_handle, out_error),
        Err(error) => {
            write_null_handle(out_handle);
            write_out_string(out_error, &error.to_string());
            COPPERLACE_PARSE_ERROR
        }
    }
}

/// Compiles a configuration string and returns an opaque ruleset handle.
///
/// On success, writes a non-null handle to `out_handle` and returns
/// [`COPPERLACE_OK`]. On failure, writes null to `out_handle`, writes an owned
/// error string to `out_error` when provided, and returns a nonzero status code.
/// Returned error strings must be released with `copperlace_string_free`.
///
/// # Safety
///
/// `config` must point to a valid NUL-terminated UTF-8 string. `out_handle`
/// must be a valid writable pointer when non-null. `out_error` must be valid
/// for writing when non-null, and any returned error string must be released
/// with [`copperlace_string_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn copperlace_ruleset_from_string(
    config: *const c_char,
    out_handle: *mut *mut CopperlaceRuleSet,
    out_error: *mut *mut c_char,
) -> c_int {
    clear_out_error(out_error);

    let Some(config) = read_c_string(config, out_error) else {
        write_null_handle(out_handle);
        return COPPERLACE_INVALID_ARGUMENT;
    };

    match ruleset_from_str(&config) {
        Ok(ruleset) => write_handle(ruleset, out_handle, out_error),
        Err(error) => {
            write_null_handle(out_handle);
            write_out_string(out_error, &error.to_string());
            COPPERLACE_PARSE_ERROR
        }
    }
}

/// Compiles a configuration string and returns a ruleset handle with custom processors.
///
/// # Safety
///
/// `config` must point to a valid NUL-terminated UTF-8 string. When
/// `processor_len` is nonzero, `processor_names`, `processor_callbacks`, and
/// `processor_user_data` must each point to arrays with at least
/// `processor_len` entries. Processor names must point to valid NUL-terminated
/// UTF-8 strings. `out_handle` must be a valid writable pointer when non-null.
/// `out_error` must be valid for writing when non-null, and any returned error
/// string must be released with [`copperlace_string_free`]. Processor callbacks
/// and user data must remain valid until the returned ruleset handle is freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn copperlace_ruleset_from_string_with_processors(
    config: *const c_char,
    processor_names: *const *const c_char,
    processor_callbacks: *const Option<CopperlaceProcessorCallback>,
    processor_user_data: *const *mut c_void,
    processor_len: usize,
    out_handle: *mut *mut CopperlaceRuleSet,
    out_error: *mut *mut c_char,
) -> c_int {
    clear_out_error(out_error);

    let Some(config) = read_c_string(config, out_error) else {
        write_null_handle(out_handle);
        return COPPERLACE_INVALID_ARGUMENT;
    };
    let Some(processors) = read_processors(
        processor_names,
        processor_callbacks,
        processor_user_data,
        processor_len,
        out_error,
    ) else {
        write_null_handle(out_handle);
        return COPPERLACE_INVALID_ARGUMENT;
    };

    match ruleset_from_str_with_processors(&config, processors) {
        Ok(ruleset) => write_handle(ruleset, out_handle, out_error),
        Err(error) => {
            write_null_handle(out_handle);
            write_out_string(out_error, &error.to_string());
            COPPERLACE_PARSE_ERROR
        }
    }
}

/// Sets the output for a custom processor callback result.
///
/// # Safety
///
/// `result` must be the valid result handle passed to the active processor
/// callback. `value` must point to a valid NUL-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn copperlace_processor_result_set_output(
    result: *mut CopperlaceProcessorResult,
    value: *const c_char,
) -> c_int {
    if result.is_null() {
        return COPPERLACE_INVALID_ARGUMENT;
    }
    let Some(value) = read_c_string(value, ptr::null_mut()) else {
        return COPPERLACE_INVALID_ARGUMENT;
    };
    unsafe {
        (*result).output = Some(value);
    }
    COPPERLACE_OK
}

/// Sets the error for a custom processor callback result.
///
/// # Safety
///
/// `result` must be the valid result handle passed to the active processor
/// callback. `message` must point to a valid NUL-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn copperlace_processor_result_set_error(
    result: *mut CopperlaceProcessorResult,
    message: *const c_char,
) -> c_int {
    if result.is_null() {
        return COPPERLACE_INVALID_ARGUMENT;
    }
    let Some(message) = read_c_string(message, ptr::null_mut()) else {
        return COPPERLACE_INVALID_ARGUMENT;
    };
    unsafe {
        (*result).error = Some(message);
    }
    COPPERLACE_OK
}

/// Renders a named rule from a ruleset handle.
///
/// On success, writes an owned UTF-8 string to `out_string` and returns
/// [`COPPERLACE_OK`]. On failure, writes null to `out_string`, writes an owned
/// error string to `out_error` when provided, and returns a nonzero status code.
/// Returned output and error strings must be released with
/// `copperlace_string_free`.
///
/// # Safety
///
/// `handle` must be a live ruleset handle returned by Copperlace. `rule` must
/// point to a valid NUL-terminated UTF-8 string. `out_string` and `out_error`
/// must be valid for writing when non-null. Any returned output or error string
/// must be released with [`copperlace_string_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn copperlace_ruleset_render(
    handle: *const CopperlaceRuleSet,
    rule: *const c_char,
    out_string: *mut *mut c_char,
    out_error: *mut *mut c_char,
) -> c_int {
    unsafe {
        copperlace_ruleset_render_with_context(
            handle,
            rule,
            ptr::null(),
            ptr::null(),
            0,
            out_string,
            out_error,
        )
    }
}

/// Renders a named rule from a ruleset handle with initial context values.
///
/// `context_keys` and `context_values` are parallel arrays of UTF-8 C strings.
/// They may be null only when `context_len` is zero. Duplicate keys are allowed;
/// later entries replace earlier entries.
///
/// On success, writes an owned UTF-8 string to `out_string` and returns
/// [`COPPERLACE_OK`]. On failure, writes null to `out_string`, writes an owned
/// error string to `out_error` when provided, and returns a nonzero status code.
/// Returned output and error strings must be released with
/// `copperlace_string_free`.
///
/// # Safety
///
/// `handle` must be a live ruleset handle returned by Copperlace. `rule` must
/// point to a valid NUL-terminated UTF-8 string. When `context_len` is nonzero,
/// `context_keys` and `context_values` must each point to arrays with at least
/// `context_len` entries, and every entry must point to a valid
/// NUL-terminated UTF-8 string. `out_string` and `out_error` must be valid for
/// writing when non-null. Any returned output or error string must be released
/// with [`copperlace_string_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn copperlace_ruleset_render_with_context(
    handle: *const CopperlaceRuleSet,
    rule: *const c_char,
    context_keys: *const *const c_char,
    context_values: *const *const c_char,
    context_len: usize,
    out_string: *mut *mut c_char,
    out_error: *mut *mut c_char,
) -> c_int {
    clear_out_error(out_error);
    write_null_string(out_string);

    if handle.is_null() {
        write_out_string(out_error, "ruleset handle is null");
        return COPPERLACE_INVALID_ARGUMENT;
    }

    let Some(rule) = read_c_string(rule, out_error) else {
        return COPPERLACE_INVALID_ARGUMENT;
    };
    let Some(context) = read_context(context_keys, context_values, context_len, out_error) else {
        return COPPERLACE_INVALID_ARGUMENT;
    };

    let ruleset = unsafe { &(*handle).ruleset };
    match ruleset.render_rule_with_context(&rule, context) {
        Ok(output) => {
            if write_out_string(out_string, &output) {
                COPPERLACE_OK
            } else {
                write_out_string(out_error, "output string contains an interior NUL byte");
                COPPERLACE_RENDER_ERROR
            }
        }
        Err(error) => {
            write_out_string(out_error, &error.to_string());
            COPPERLACE_RENDER_ERROR
        }
    }
}

/// Renders a named rule, inferring formatted structured JSON for object-valued rules.
///
/// String-valued and list-valued rules use existing text rendering. Object-valued
/// rules return formatted JSON using tab indentation.
///
/// # Safety
///
/// `handle` must be a live ruleset handle returned by Copperlace. `rule` must
/// point to a valid NUL-terminated UTF-8 string. `out_string` and `out_error`
/// must be valid for writing when non-null. Any returned output or error string
/// must be released with [`copperlace_string_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn copperlace_ruleset_render_inferred(
    handle: *const CopperlaceRuleSet,
    rule: *const c_char,
    out_string: *mut *mut c_char,
    out_error: *mut *mut c_char,
) -> c_int {
    unsafe {
        copperlace_ruleset_render_inferred_with_context(
            handle,
            rule,
            ptr::null(),
            ptr::null(),
            0,
            out_string,
            out_error,
        )
    }
}

/// Renders a named rule with initial context, inferring formatted structured JSON for object-valued rules.
///
/// # Safety
///
/// `handle` must be a live ruleset handle returned by Copperlace. `rule` must
/// point to a valid NUL-terminated UTF-8 string. When `context_len` is nonzero,
/// `context_keys` and `context_values` must each point to arrays with at least
/// `context_len` entries, and every entry must point to a valid
/// NUL-terminated UTF-8 string. `out_string` and `out_error` must be valid for
/// writing when non-null. Any returned output or error string must be released
/// with [`copperlace_string_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn copperlace_ruleset_render_inferred_with_context(
    handle: *const CopperlaceRuleSet,
    rule: *const c_char,
    context_keys: *const *const c_char,
    context_values: *const *const c_char,
    context_len: usize,
    out_string: *mut *mut c_char,
    out_error: *mut *mut c_char,
) -> c_int {
    clear_out_error(out_error);
    write_null_string(out_string);

    if handle.is_null() {
        write_out_string(out_error, "ruleset handle is null");
        return COPPERLACE_INVALID_ARGUMENT;
    }

    let Some(rule) = read_c_string(rule, out_error) else {
        return COPPERLACE_INVALID_ARGUMENT;
    };
    let Some(context) = read_context(context_keys, context_values, context_len, out_error) else {
        return COPPERLACE_INVALID_ARGUMENT;
    };

    let ruleset = unsafe { &(*handle).ruleset };
    match ruleset.render_rule_inferred_with_context(&rule, context) {
        Ok(output) => {
            if write_out_string(out_string, &output) {
                COPPERLACE_OK
            } else {
                write_out_string(out_error, "output string contains an interior NUL byte");
                COPPERLACE_RENDER_ERROR
            }
        }
        Err(error) => {
            write_out_string(out_error, &error.to_string());
            COPPERLACE_RENDER_ERROR
        }
    }
}

/// Renders a named structured rule from a ruleset handle as JSON text.
///
/// On success, writes an owned UTF-8 JSON string to `out_json` and returns
/// [`COPPERLACE_OK`]. When `format_json` is false, the JSON is compact. When
/// true, it is formatted with tab indentation. On failure, writes null to
/// `out_json`, writes an owned error string to `out_error` when provided, and
/// returns a nonzero status code. Returned output and error strings must be
/// released with `copperlace_string_free`.
///
/// # Safety
///
/// `handle` must be a live ruleset handle returned by Copperlace. `rule` must
/// point to a valid NUL-terminated UTF-8 string. `out_json` must be a valid
/// writable pointer. `out_error` must be valid for writing when non-null. Any
/// returned output or error string must be released with
/// [`copperlace_string_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn copperlace_ruleset_render_structured_json(
    handle: *const CopperlaceRuleSet,
    rule: *const c_char,
    format_json: bool,
    out_json: *mut *mut c_char,
    out_error: *mut *mut c_char,
) -> c_int {
    unsafe {
        copperlace_ruleset_render_structured_json_with_context(
            handle,
            rule,
            ptr::null(),
            ptr::null(),
            0,
            format_json,
            out_json,
            out_error,
        )
    }
}

/// Renders a named structured rule from a ruleset handle with initial context.
///
/// `context_keys` and `context_values` are parallel arrays of UTF-8 C strings.
/// They may be null only when `context_len` is zero. Duplicate keys are allowed;
/// later entries replace earlier entries.
///
/// On success, writes an owned UTF-8 JSON string to `out_json` and returns
/// [`COPPERLACE_OK`]. When `format_json` is false, the JSON is compact. When
/// true, it is formatted with tab indentation. On failure, writes null to
/// `out_json`, writes an owned error string to `out_error` when provided, and
/// returns a nonzero status code. Returned output and error strings must be
/// released with `copperlace_string_free`.
///
/// # Safety
///
/// `handle` must be a live ruleset handle returned by Copperlace. `rule` must
/// point to a valid NUL-terminated UTF-8 string. When `context_len` is nonzero,
/// `context_keys` and `context_values` must each point to arrays with at least
/// `context_len` entries, and every entry must point to a valid
/// NUL-terminated UTF-8 string. `out_json` must be a valid writable pointer.
/// `out_error` must be valid for writing when non-null. Any returned output or
/// error string must be released with [`copperlace_string_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn copperlace_ruleset_render_structured_json_with_context(
    handle: *const CopperlaceRuleSet,
    rule: *const c_char,
    context_keys: *const *const c_char,
    context_values: *const *const c_char,
    context_len: usize,
    format_json: bool,
    out_json: *mut *mut c_char,
    out_error: *mut *mut c_char,
) -> c_int {
    clear_out_error(out_error);

    if out_json.is_null() {
        write_out_string(out_error, "out_json is null");
        return COPPERLACE_INVALID_ARGUMENT;
    }
    write_null_string(out_json);

    if handle.is_null() {
        write_out_string(out_error, "ruleset handle is null");
        return COPPERLACE_INVALID_ARGUMENT;
    }

    let Some(rule) = read_c_string(rule, out_error) else {
        return COPPERLACE_INVALID_ARGUMENT;
    };
    let Some(context) = read_context(context_keys, context_values, context_len, out_error) else {
        return COPPERLACE_INVALID_ARGUMENT;
    };

    let ruleset = unsafe { &(*handle).ruleset };
    let json = ruleset
        .render_rule_structured_with_context(&rule, context)
        .and_then(|value| {
            if format_json {
                value.to_formatted_json()
            } else {
                value.to_compact_json()
            }
        });

    match json {
        Ok(output) => {
            if write_out_string(out_json, &output) {
                COPPERLACE_OK
            } else {
                write_out_string(out_error, "structured JSON contains an interior NUL byte");
                COPPERLACE_RENDER_ERROR
            }
        }
        Err(error) => {
            write_out_string(out_error, &error.to_string());
            COPPERLACE_RENDER_ERROR
        }
    }
}

fn read_context(
    keys: *const *const c_char,
    values: *const *const c_char,
    len: usize,
    out_error: *mut *mut c_char,
) -> Option<RenderContext> {
    let mut context = RenderContext::new();
    if len == 0 {
        return Some(context);
    }
    if keys.is_null() {
        write_out_string(out_error, "context keys array is null");
        return None;
    }
    if values.is_null() {
        write_out_string(out_error, "context values array is null");
        return None;
    }

    for index in 0..len {
        let key_ptr = unsafe { *keys.add(index) };
        let value_ptr = unsafe { *values.add(index) };
        let key = read_c_string(key_ptr, out_error)?;
        let value = read_c_string(value_ptr, out_error)?;
        context.insert(key, value);
    }

    Some(context)
}

fn read_processors(
    names: *const *const c_char,
    callbacks: *const Option<CopperlaceProcessorCallback>,
    user_data: *const *mut c_void,
    len: usize,
    out_error: *mut *mut c_char,
) -> Option<ProcessorRegistry> {
    let mut processors = ProcessorRegistry::new();
    if len == 0 {
        return Some(processors);
    }
    if names.is_null() {
        write_out_string(out_error, "processor names array is null");
        return None;
    }
    if callbacks.is_null() {
        write_out_string(out_error, "processor callbacks array is null");
        return None;
    }
    if user_data.is_null() {
        write_out_string(out_error, "processor user data array is null");
        return None;
    }

    for index in 0..len {
        let name_ptr = unsafe { *names.add(index) };
        let callback = unsafe { *callbacks.add(index) };
        let Some(callback) = callback else {
            write_out_string(out_error, "processor callback is null");
            return None;
        };
        let name = read_c_string(name_ptr, out_error)?;
        let user_data = unsafe { *user_data.add(index) };
        processors.insert(
            name,
            std::sync::Arc::new(CallbackProcessor {
                callback,
                user_data,
            }),
        );
    }

    Some(processors)
}

fn ruleset_from_str_with_processors(
    config: &str,
    processors: ProcessorRegistry,
) -> Result<RuleSet, ConfigError> {
    let value = hocon_rs::Config::parse_str::<hocon_rs::Value>(config, None)
        .map_err(|error| ConfigError::Parse(format!("{error:?}")))?;
    RuleSet::from_config_with_processors(value, processors).map_err(ConfigError::Render)
}

fn ruleset_from_file_with_processors(
    path: String,
    processors: ProcessorRegistry,
) -> Result<RuleSet, ConfigError> {
    let value = hocon_rs::Config::load(&path, None)
        .map_err(|error| ConfigError::Parse(format!("{error:?}")))?;
    RuleSet::from_config_with_processors(value, processors).map_err(ConfigError::Render)
}

/// Releases a ruleset handle returned by the C ABI.
///
/// Passing null is allowed and has no effect.
///
/// # Safety
///
/// `handle` must be null or a handle previously returned by Copperlace that has
/// not already been freed. After this call, the handle must not be used again.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn copperlace_ruleset_free(handle: *mut CopperlaceRuleSet) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

/// Releases a string returned by the C ABI.
///
/// Passing null is allowed and has no effect.
///
/// # Safety
///
/// `value` must be null or a string pointer previously returned by Copperlace
/// that has not already been freed. After this call, the pointer must not be
/// used again.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn copperlace_string_free(value: *mut c_char) {
    if !value.is_null() {
        unsafe {
            drop(CString::from_raw(value));
        }
    }
}

fn read_c_string(value: *const c_char, out_error: *mut *mut c_char) -> Option<String> {
    if value.is_null() {
        write_out_string(out_error, "input string is null");
        return None;
    }

    match unsafe { CStr::from_ptr(value) }.to_str() {
        Ok(value) => Some(value.to_string()),
        Err(error) => {
            write_out_string(
                out_error,
                &format!("input string is not valid UTF-8: {error}"),
            );
            None
        }
    }
}

fn write_handle(
    ruleset: RuleSet,
    out_handle: *mut *mut CopperlaceRuleSet,
    out_error: *mut *mut c_char,
) -> c_int {
    if out_handle.is_null() {
        write_out_string(out_error, "out_handle is null");
        return COPPERLACE_INVALID_ARGUMENT;
    }

    let handle = Box::into_raw(Box::new(CopperlaceRuleSet { ruleset }));
    unsafe {
        *out_handle = handle;
    }
    COPPERLACE_OK
}

fn write_null_handle(out_handle: *mut *mut CopperlaceRuleSet) {
    if !out_handle.is_null() {
        unsafe {
            *out_handle = ptr::null_mut();
        }
    }
}

fn write_null_string(out_string: *mut *mut c_char) {
    if !out_string.is_null() {
        unsafe {
            *out_string = ptr::null_mut();
        }
    }
}

fn clear_out_error(out_error: *mut *mut c_char) {
    write_null_string(out_error);
}

fn write_out_string(out_string: *mut *mut c_char, value: &str) -> bool {
    if out_string.is_null() {
        return true;
    }

    let Ok(value) = CString::new(value) else {
        return false;
    };

    unsafe {
        *out_string = value.into_raw();
    }
    true
}
