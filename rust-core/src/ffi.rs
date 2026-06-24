use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;

use crate::config::{ruleset_from_hocon_file, ruleset_from_hocon_str};
use crate::render::{RenderContext, RuleSet};

/// Status code for a successful C ABI call.
pub const COPPERLACE_OK: c_int = 0;
/// Status code for invalid C ABI arguments such as null required pointers.
pub const COPPERLACE_INVALID_ARGUMENT: c_int = 1;
/// Status code for config loading, parsing, or compilation failures.
pub const COPPERLACE_PARSE_ERROR: c_int = 2;
/// Status code for rule rendering failures.
pub const COPPERLACE_RENDER_ERROR: c_int = 3;

/// Opaque C ABI handle for a compiled Copperlace ruleset.
///
/// Handles are allocated by `copperlace_ruleset_from_file` or
/// `copperlace_ruleset_from_string` and must be released with
/// `copperlace_ruleset_free`.
pub struct CopperlaceRuleSet {
    ruleset: RuleSet,
}

/// Loads a HOCON config file and returns an opaque ruleset handle.
///
/// On success, writes a non-null handle to `out_handle` and returns
/// [`COPPERLACE_OK`]. On failure, writes null to `out_handle`, writes an owned
/// error string to `out_error` when provided, and returns a nonzero status code.
/// Returned error strings must be released with `copperlace_string_free`.
#[unsafe(no_mangle)]
pub extern "C" fn copperlace_ruleset_from_file(
    path: *const c_char,
    out_handle: *mut *mut CopperlaceRuleSet,
    out_error: *mut *mut c_char,
) -> c_int {
    clear_out_error(out_error);

    let Some(path) = read_c_string(path, out_error) else {
        write_null_handle(out_handle);
        return COPPERLACE_INVALID_ARGUMENT;
    };

    match ruleset_from_hocon_file(path) {
        Ok(ruleset) => write_handle(ruleset, out_handle, out_error),
        Err(error) => {
            write_null_handle(out_handle);
            write_out_string(out_error, &error.to_string());
            COPPERLACE_PARSE_ERROR
        }
    }
}

/// Compiles a HOCON config string and returns an opaque ruleset handle.
///
/// On success, writes a non-null handle to `out_handle` and returns
/// [`COPPERLACE_OK`]. On failure, writes null to `out_handle`, writes an owned
/// error string to `out_error` when provided, and returns a nonzero status code.
/// Returned error strings must be released with `copperlace_string_free`.
#[unsafe(no_mangle)]
pub extern "C" fn copperlace_ruleset_from_string(
    config: *const c_char,
    out_handle: *mut *mut CopperlaceRuleSet,
    out_error: *mut *mut c_char,
) -> c_int {
    clear_out_error(out_error);

    let Some(config) = read_c_string(config, out_error) else {
        write_null_handle(out_handle);
        return COPPERLACE_INVALID_ARGUMENT;
    };

    match ruleset_from_hocon_str(&config) {
        Ok(ruleset) => write_handle(ruleset, out_handle, out_error),
        Err(error) => {
            write_null_handle(out_handle);
            write_out_string(out_error, &error.to_string());
            COPPERLACE_PARSE_ERROR
        }
    }
}

/// Renders a named rule from a ruleset handle.
///
/// On success, writes an owned UTF-8 string to `out_string` and returns
/// [`COPPERLACE_OK`]. On failure, writes null to `out_string`, writes an owned
/// error string to `out_error` when provided, and returns a nonzero status code.
/// Returned output and error strings must be released with
/// `copperlace_string_free`.
#[unsafe(no_mangle)]
pub extern "C" fn copperlace_ruleset_render(
    handle: *const CopperlaceRuleSet,
    rule: *const c_char,
    out_string: *mut *mut c_char,
    out_error: *mut *mut c_char,
) -> c_int {
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
#[unsafe(no_mangle)]
pub extern "C" fn copperlace_ruleset_render_with_context(
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

/// Releases a ruleset handle returned by the C ABI.
///
/// Passing null is allowed and has no effect.
#[unsafe(no_mangle)]
pub extern "C" fn copperlace_ruleset_free(handle: *mut CopperlaceRuleSet) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

/// Releases a string returned by the C ABI.
///
/// Passing null is allowed and has no effect.
#[unsafe(no_mangle)]
pub extern "C" fn copperlace_string_free(value: *mut c_char) {
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
