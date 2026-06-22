use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;

use crate::config::{ruleset_from_hocon_file, ruleset_from_hocon_str};
use crate::render::RuleSet;

pub const COPPERLACE_OK: c_int = 0;
pub const COPPERLACE_INVALID_ARGUMENT: c_int = 1;
pub const COPPERLACE_PARSE_ERROR: c_int = 2;
pub const COPPERLACE_RENDER_ERROR: c_int = 3;

pub struct CopperlaceRuleSet {
    ruleset: RuleSet,
}

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

#[unsafe(no_mangle)]
pub extern "C" fn copperlace_ruleset_free(handle: *mut CopperlaceRuleSet) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

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
