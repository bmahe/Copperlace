use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

use copperlace::ffi::{
    COPPERLACE_INVALID_ARGUMENT, COPPERLACE_OK, COPPERLACE_RENDER_ERROR,
    CopperlaceProcessorCallback, CopperlaceProcessorResult, CopperlaceRuleSet,
    copperlace_processor_result_set_error as raw_copperlace_processor_result_set_error,
    copperlace_processor_result_set_output as raw_copperlace_processor_result_set_output,
    copperlace_ruleset_free as raw_copperlace_ruleset_free,
    copperlace_ruleset_from_string as raw_copperlace_ruleset_from_string,
    copperlace_ruleset_from_string_with_processors as raw_copperlace_ruleset_from_string_with_processors,
    copperlace_ruleset_render as raw_copperlace_ruleset_render,
    copperlace_ruleset_render_with_context as raw_copperlace_ruleset_render_with_context,
    copperlace_string_free as raw_copperlace_string_free,
};

fn copperlace_ruleset_from_string(
    config: *const c_char,
    out_handle: *mut *mut CopperlaceRuleSet,
    out_error: *mut *mut c_char,
) -> c_int {
    unsafe { raw_copperlace_ruleset_from_string(config, out_handle, out_error) }
}

fn copperlace_ruleset_from_string_with_processors(
    config: *const c_char,
    processor_names: *const *const c_char,
    processor_callbacks: *const Option<CopperlaceProcessorCallback>,
    processor_user_data: *const *mut c_void,
    processor_len: usize,
    out_handle: *mut *mut CopperlaceRuleSet,
    out_error: *mut *mut c_char,
) -> c_int {
    unsafe {
        raw_copperlace_ruleset_from_string_with_processors(
            config,
            processor_names,
            processor_callbacks,
            processor_user_data,
            processor_len,
            out_handle,
            out_error,
        )
    }
}

fn copperlace_ruleset_render(
    handle: *const CopperlaceRuleSet,
    rule: *const c_char,
    out_string: *mut *mut c_char,
    out_error: *mut *mut c_char,
) -> c_int {
    unsafe { raw_copperlace_ruleset_render(handle, rule, out_string, out_error) }
}

fn copperlace_ruleset_render_with_context(
    handle: *const CopperlaceRuleSet,
    rule: *const c_char,
    context_keys: *const *const c_char,
    context_values: *const *const c_char,
    context_len: usize,
    out_string: *mut *mut c_char,
    out_error: *mut *mut c_char,
) -> c_int {
    unsafe {
        raw_copperlace_ruleset_render_with_context(
            handle,
            rule,
            context_keys,
            context_values,
            context_len,
            out_string,
            out_error,
        )
    }
}

fn copperlace_processor_result_set_output(
    result: *mut CopperlaceProcessorResult,
    value: *const c_char,
) -> c_int {
    unsafe { raw_copperlace_processor_result_set_output(result, value) }
}

fn copperlace_processor_result_set_error(
    result: *mut CopperlaceProcessorResult,
    message: *const c_char,
) -> c_int {
    unsafe { raw_copperlace_processor_result_set_error(result, message) }
}

fn copperlace_ruleset_free(handle: *mut CopperlaceRuleSet) {
    unsafe { raw_copperlace_ruleset_free(handle) }
}

fn copperlace_string_free(value: *mut c_char) {
    unsafe { raw_copperlace_string_free(value) }
}

#[test]
fn creates_handle_and_renders_rule() {
    let config = CString::new(
        r#"
        name = ["Mia"]
        origin = "{name}"
        "#,
    )
    .unwrap();
    let rule = CString::new("origin").unwrap();
    let mut handle = ptr::null_mut();
    let mut error = ptr::null_mut();
    let mut output = ptr::null_mut();

    assert_eq!(
        copperlace_ruleset_from_string(config.as_ptr(), &mut handle, &mut error),
        COPPERLACE_OK
    );
    assert!(error.is_null());
    assert!(!handle.is_null());

    assert_eq!(
        copperlace_ruleset_render(handle, rule.as_ptr(), &mut output, &mut error),
        COPPERLACE_OK
    );
    assert!(error.is_null());
    assert_eq!(unsafe { CStr::from_ptr(output) }.to_str().unwrap(), "Mia");

    copperlace_string_free(output);
    copperlace_ruleset_free(handle);
}

#[test]
fn renders_rule_with_initial_context() {
    let config = CString::new(
        r#"
        context {
            name = "Mia"
        }
        origin = "Hello {name}"
        "#,
    )
    .unwrap();
    let rule = CString::new("origin").unwrap();
    let key = CString::new("name").unwrap();
    let first_value = CString::new("Darcy").unwrap();
    let second_value = CString::new("Lina").unwrap();
    let keys = [key.as_ptr(), key.as_ptr()];
    let values = [first_value.as_ptr(), second_value.as_ptr()];
    let mut handle = ptr::null_mut();
    let mut error = ptr::null_mut();
    let mut output = ptr::null_mut();

    assert_eq!(
        copperlace_ruleset_from_string(config.as_ptr(), &mut handle, &mut error),
        COPPERLACE_OK
    );

    assert_eq!(
        copperlace_ruleset_render_with_context(
            handle,
            rule.as_ptr(),
            keys.as_ptr(),
            values.as_ptr(),
            keys.len(),
            &mut output,
            &mut error,
        ),
        COPPERLACE_OK
    );
    assert!(error.is_null());
    assert_eq!(
        unsafe { CStr::from_ptr(output) }.to_str().unwrap(),
        "Hello Lina"
    );

    copperlace_string_free(output);
    copperlace_ruleset_free(handle);
}

#[test]
fn context_render_rejects_null_context_arrays_with_nonzero_length() {
    let config = CString::new(r#"origin = "Mia""#).unwrap();
    let rule = CString::new("origin").unwrap();
    let mut handle = ptr::null_mut();
    let mut error = ptr::null_mut();
    let mut output = ptr::null_mut();

    assert_eq!(
        copperlace_ruleset_from_string(config.as_ptr(), &mut handle, &mut error),
        COPPERLACE_OK
    );

    assert_eq!(
        copperlace_ruleset_render_with_context(
            handle,
            rule.as_ptr(),
            ptr::null(),
            ptr::null(),
            1,
            &mut output,
            &mut error,
        ),
        COPPERLACE_INVALID_ARGUMENT
    );
    assert!(output.is_null());
    assert!(!error.is_null());

    copperlace_string_free(error);
    copperlace_ruleset_free(handle);
}

#[test]
fn render_error_sets_error_string() {
    let config = CString::new(r#"origin = "{missing}""#).unwrap();
    let rule = CString::new("origin").unwrap();
    let mut handle = ptr::null_mut();
    let mut error = ptr::null_mut();
    let mut output = ptr::null_mut();

    assert_eq!(
        copperlace_ruleset_from_string(config.as_ptr(), &mut handle, &mut error),
        COPPERLACE_OK
    );
    assert_eq!(
        copperlace_ruleset_render(handle, rule.as_ptr(), &mut output, &mut error),
        COPPERLACE_RENDER_ERROR
    );
    assert!(output.is_null());
    assert!(!error.is_null());

    copperlace_string_free(error);
    copperlace_ruleset_free(handle);
}

#[test]
fn renders_with_custom_processor() {
    let config = CString::new(
        r#"
        name = "Mia"
        origin = "{name | quote_name}"
        "#,
    )
    .unwrap();
    let processor_name = CString::new("quote_name").unwrap();
    let names = [processor_name.as_ptr()];
    let callbacks: [Option<CopperlaceProcessorCallback>; 1] = [Some(quote_name)];
    let mut suffix = CString::new("!").unwrap();
    let user_data = [&mut suffix as *mut CString as *mut c_void];
    let rule = CString::new("origin").unwrap();
    let mut handle = ptr::null_mut();
    let mut error = ptr::null_mut();
    let mut output = ptr::null_mut();

    assert_eq!(
        copperlace_ruleset_from_string_with_processors(
            config.as_ptr(),
            names.as_ptr(),
            callbacks.as_ptr(),
            user_data.as_ptr(),
            names.len(),
            &mut handle,
            &mut error,
        ),
        COPPERLACE_OK
    );

    assert_eq!(
        copperlace_ruleset_render(handle, rule.as_ptr(), &mut output, &mut error),
        COPPERLACE_OK
    );
    assert_eq!(
        unsafe { CStr::from_ptr(output) }.to_str().unwrap(),
        "'Mia!'"
    );

    copperlace_string_free(output);
    copperlace_ruleset_free(handle);
}

#[test]
fn custom_processor_overrides_builtin_processor() {
    let config = CString::new(
        r#"
        name = "Mia"
        origin = "{name | uppercase}"
        "#,
    )
    .unwrap();
    let processor_name = CString::new("uppercase").unwrap();
    let names = [processor_name.as_ptr()];
    let callbacks: [Option<CopperlaceProcessorCallback>; 1] = [Some(replace_with_custom)];
    let user_data = [ptr::null_mut()];
    let rule = CString::new("origin").unwrap();
    let mut handle = ptr::null_mut();
    let mut error = ptr::null_mut();
    let mut output = ptr::null_mut();

    assert_eq!(
        copperlace_ruleset_from_string_with_processors(
            config.as_ptr(),
            names.as_ptr(),
            callbacks.as_ptr(),
            user_data.as_ptr(),
            names.len(),
            &mut handle,
            &mut error,
        ),
        COPPERLACE_OK
    );

    assert_eq!(
        copperlace_ruleset_render(handle, rule.as_ptr(), &mut output, &mut error),
        COPPERLACE_OK
    );
    assert_eq!(
        unsafe { CStr::from_ptr(output) }.to_str().unwrap(),
        "custom"
    );

    copperlace_string_free(output);
    copperlace_ruleset_free(handle);
}

#[test]
fn custom_processor_error_sets_render_error() {
    let config = CString::new(
        r#"
        name = "Mia"
        origin = "{name | fail}"
        "#,
    )
    .unwrap();
    let processor_name = CString::new("fail").unwrap();
    let names = [processor_name.as_ptr()];
    let callbacks: [Option<CopperlaceProcessorCallback>; 1] = [Some(fail_processor)];
    let user_data = [ptr::null_mut()];
    let rule = CString::new("origin").unwrap();
    let mut handle = ptr::null_mut();
    let mut error = ptr::null_mut();
    let mut output = ptr::null_mut();

    assert_eq!(
        copperlace_ruleset_from_string_with_processors(
            config.as_ptr(),
            names.as_ptr(),
            callbacks.as_ptr(),
            user_data.as_ptr(),
            names.len(),
            &mut handle,
            &mut error,
        ),
        COPPERLACE_OK
    );

    assert_eq!(
        copperlace_ruleset_render(handle, rule.as_ptr(), &mut output, &mut error),
        COPPERLACE_RENDER_ERROR
    );
    assert!(output.is_null());
    assert!(
        unsafe { CStr::from_ptr(error) }
            .to_str()
            .unwrap()
            .contains("not allowed")
    );

    copperlace_string_free(error);
    copperlace_ruleset_free(handle);
}

#[test]
fn rejects_null_processor_callback() {
    let config = CString::new(r#"origin = "Mia""#).unwrap();
    let processor_name = CString::new("custom").unwrap();
    let names = [processor_name.as_ptr()];
    let callbacks: [Option<CopperlaceProcessorCallback>; 1] = [None];
    let user_data = [ptr::null_mut()];
    let mut handle = ptr::null_mut();
    let mut error = ptr::null_mut();

    assert_eq!(
        copperlace_ruleset_from_string_with_processors(
            config.as_ptr(),
            names.as_ptr(),
            callbacks.as_ptr(),
            user_data.as_ptr(),
            names.len(),
            &mut handle,
            &mut error,
        ),
        COPPERLACE_INVALID_ARGUMENT
    );
    assert!(handle.is_null());
    assert!(!error.is_null());

    copperlace_string_free(error);
}

unsafe extern "C" fn quote_name(
    input: *const c_char,
    result: *mut CopperlaceProcessorResult,
    user_data: *mut c_void,
) -> c_int {
    let value = unsafe { CStr::from_ptr(input) }.to_str().unwrap();
    let suffix = unsafe { &*(user_data as *const CString) };
    let suffix = suffix.to_str().unwrap();
    let output = CString::new(format!("'{value}{suffix}'")).unwrap();
    copperlace_processor_result_set_output(result, output.as_ptr())
}

unsafe extern "C" fn replace_with_custom(
    _input: *const c_char,
    result: *mut CopperlaceProcessorResult,
    _user_data: *mut c_void,
) -> c_int {
    let output = CString::new("custom").unwrap();
    copperlace_processor_result_set_output(result, output.as_ptr())
}

unsafe extern "C" fn fail_processor(
    _input: *const c_char,
    result: *mut CopperlaceProcessorResult,
    _user_data: *mut c_void,
) -> c_int {
    let error = CString::new("not allowed").unwrap();
    copperlace_processor_result_set_error(result, error.as_ptr())
}
