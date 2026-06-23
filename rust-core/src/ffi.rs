use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;

use crate::config::{ruleset_from_hocon_file, ruleset_from_hocon_str};
use crate::render::RuleSet;

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
    clear_out_error(out_error);
    write_null_string(out_string);

    if handle.is_null() {
        write_out_string(out_error, "ruleset handle is null");
        return COPPERLACE_INVALID_ARGUMENT;
    }

    let Some(rule) = read_c_string(rule, out_error) else {
        return COPPERLACE_INVALID_ARGUMENT;
    };

    let ruleset = unsafe { &(*handle).ruleset };
    match ruleset.render_rule(&rule) {
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
