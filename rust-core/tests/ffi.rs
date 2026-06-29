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
    copperlace_ruleset_render_structured_json as raw_copperlace_ruleset_render_structured_json,
    copperlace_ruleset_render_structured_json_with_context as raw_copperlace_ruleset_render_structured_json_with_context,
    copperlace_ruleset_render_with_context as raw_copperlace_ruleset_render_with_context,
    copperlace_string_free as raw_copperlace_string_free,
};
use serde_json::json;

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

fn copperlace_ruleset_render_structured_json(
    handle: *const CopperlaceRuleSet,
    rule: *const c_char,
    format_json: bool,
    out_json: *mut *mut c_char,
    out_error: *mut *mut c_char,
) -> c_int {
    unsafe {
        raw_copperlace_ruleset_render_structured_json(
            handle,
            rule,
            format_json,
            out_json,
            out_error,
        )
    }
}

fn copperlace_ruleset_render_structured_json_with_context(
    handle: *const CopperlaceRuleSet,
    rule: *const c_char,
    context_keys: *const *const c_char,
    context_values: *const *const c_char,
    context_len: usize,
    format_json: bool,
    out_json: *mut *mut c_char,
    out_error: *mut *mut c_char,
) -> c_int {
    unsafe {
        raw_copperlace_ruleset_render_structured_json_with_context(
            handle,
            rule,
            context_keys,
            context_values,
            context_len,
            format_json,
            out_json,
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

#[test]
fn renders_structured_json_compact() {
    let config = CString::new(
        r#"
        name = ["Mia"]
        origin {
            title = "Hello {name}"
            tags = ["generated", "{name | slug}"]
            count = 3
            active = true
            missing = null
            nested {
                value = "ok"
            }
        }
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

    assert_eq!(
        copperlace_ruleset_render_structured_json(
            handle,
            rule.as_ptr(),
            false,
            &mut output,
            &mut error,
        ),
        COPPERLACE_OK
    );
    assert!(error.is_null());
    let rendered = unsafe { CStr::from_ptr(output) }.to_str().unwrap();
    assert!(!rendered.contains('\n'));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(rendered).unwrap(),
        json!({
            "active": true,
            "count": 3,
            "missing": null,
            "nested": {
                "value": "ok"
            },
            "tags": ["generated", "mia"],
            "title": "Hello Mia"
        })
    );

    copperlace_string_free(output);
    copperlace_ruleset_free(handle);
}

#[test]
fn renders_structured_json_formatted_with_tabs() {
    let config = CString::new(
        r#"
        origin {
            title = "Hello"
            items = ["one", "two"]
        }
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

    assert_eq!(
        copperlace_ruleset_render_structured_json(
            handle,
            rule.as_ptr(),
            true,
            &mut output,
            &mut error,
        ),
        COPPERLACE_OK
    );
    assert!(error.is_null());
    let rendered = unsafe { CStr::from_ptr(output) }.to_str().unwrap();
    assert!(rendered.contains("\n\t"));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(rendered).unwrap(),
        json!({
            "items": ["one", "two"],
            "title": "Hello"
        })
    );

    copperlace_string_free(output);
    copperlace_ruleset_free(handle);
}

#[test]
fn renders_structured_json_with_initial_context() {
    let config = CString::new(
        r#"
        context {
            name = "Mia"
        }
        origin {
            greeting = "Hello {name}"
        }
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
        copperlace_ruleset_render_structured_json_with_context(
            handle,
            rule.as_ptr(),
            keys.as_ptr(),
            values.as_ptr(),
            keys.len(),
            false,
            &mut output,
            &mut error,
        ),
        COPPERLACE_OK
    );
    assert!(error.is_null());
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(
            unsafe { CStr::from_ptr(output) }.to_str().unwrap()
        )
        .unwrap(),
        json!({
            "greeting": "Hello Lina"
        })
    );

    copperlace_string_free(output);
    copperlace_ruleset_free(handle);
}

#[test]
fn renders_structured_json_named_list_choices() {
    let config = CString::new(
        r#"
        headline = ["Bright meadow", "Quiet harbor"]
        origin {
            headline = "{headline}"
        }
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

    assert_eq!(
        copperlace_ruleset_render_structured_json(
            handle,
            rule.as_ptr(),
            false,
            &mut output,
            &mut error,
        ),
        COPPERLACE_OK
    );
    assert!(error.is_null());
    assert!(
        ["Bright meadow", "Quiet harbor"].contains(
            &serde_json::from_str::<serde_json::Value>(
                unsafe { CStr::from_ptr(output) }.to_str().unwrap()
            )
            .unwrap()["headline"]
                .as_str()
                .unwrap()
        )
    );

    copperlace_string_free(output);
    copperlace_ruleset_free(handle);
}

#[test]
fn renders_structured_json_with_builtin_and_custom_processors() {
    let config = CString::new(
        r#"
        name = "Mia"
        origin {
            builtin = "{name | uppercase}"
            custom = "{name | quote_name}"
        }
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
        copperlace_ruleset_render_structured_json(
            handle,
            rule.as_ptr(),
            false,
            &mut output,
            &mut error,
        ),
        COPPERLACE_OK
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(
            unsafe { CStr::from_ptr(output) }.to_str().unwrap()
        )
        .unwrap(),
        json!({
            "builtin": "MIA",
            "custom": "'Mia!'"
        })
    );

    copperlace_string_free(output);
    copperlace_ruleset_free(handle);
}

#[test]
fn structured_json_render_error_sets_error_string() {
    let config = CString::new(r#"origin { value = "{missing}" }"#).unwrap();
    let rule = CString::new("origin").unwrap();
    let mut handle = ptr::null_mut();
    let mut error = ptr::null_mut();
    let mut output = ptr::null_mut();

    assert_eq!(
        copperlace_ruleset_from_string(config.as_ptr(), &mut handle, &mut error),
        COPPERLACE_OK
    );

    assert_eq!(
        copperlace_ruleset_render_structured_json(
            handle,
            rule.as_ptr(),
            false,
            &mut output,
            &mut error,
        ),
        COPPERLACE_RENDER_ERROR
    );
    assert!(output.is_null());
    assert!(!error.is_null());

    copperlace_string_free(error);
    copperlace_ruleset_free(handle);
}

#[test]
fn structured_json_rejects_invalid_abi_inputs() {
    let config = CString::new(r#"origin { value = "Mia" }"#).unwrap();
    let rule = CString::new("origin").unwrap();
    let key = CString::new("name").unwrap();
    let value = CString::new("Mia").unwrap();
    let invalid_rule = [0xffu8, 0];
    let null_entry_keys = [ptr::null()];
    let null_entry_values = [value.as_ptr()];
    let mut handle = ptr::null_mut();
    let mut error = ptr::null_mut();
    let mut output = ptr::null_mut();

    assert_eq!(
        copperlace_ruleset_from_string(config.as_ptr(), &mut handle, &mut error),
        COPPERLACE_OK
    );

    assert_eq!(
        copperlace_ruleset_render_structured_json(
            ptr::null(),
            rule.as_ptr(),
            false,
            &mut output,
            &mut error,
        ),
        COPPERLACE_INVALID_ARGUMENT
    );
    assert!(output.is_null());
    assert!(!error.is_null());
    copperlace_string_free(error);
    error = ptr::null_mut();

    assert_eq!(
        copperlace_ruleset_render_structured_json(
            handle,
            ptr::null(),
            false,
            &mut output,
            &mut error,
        ),
        COPPERLACE_INVALID_ARGUMENT
    );
    assert!(output.is_null());
    assert!(!error.is_null());
    copperlace_string_free(error);
    error = ptr::null_mut();

    assert_eq!(
        copperlace_ruleset_render_structured_json(
            handle,
            rule.as_ptr(),
            false,
            ptr::null_mut(),
            &mut error,
        ),
        COPPERLACE_INVALID_ARGUMENT
    );
    assert!(!error.is_null());
    copperlace_string_free(error);
    error = ptr::null_mut();

    assert_eq!(
        copperlace_ruleset_render_structured_json_with_context(
            handle,
            rule.as_ptr(),
            ptr::null(),
            ptr::null(),
            1,
            false,
            &mut output,
            &mut error,
        ),
        COPPERLACE_INVALID_ARGUMENT
    );
    assert!(output.is_null());
    assert!(!error.is_null());
    copperlace_string_free(error);
    error = ptr::null_mut();

    assert_eq!(
        copperlace_ruleset_render_structured_json_with_context(
            handle,
            rule.as_ptr(),
            null_entry_keys.as_ptr(),
            null_entry_values.as_ptr(),
            1,
            false,
            &mut output,
            &mut error,
        ),
        COPPERLACE_INVALID_ARGUMENT
    );
    assert!(output.is_null());
    assert!(!error.is_null());
    copperlace_string_free(error);
    error = ptr::null_mut();

    let keys = [key.as_ptr()];
    let invalid_values = [invalid_rule.as_ptr().cast::<c_char>()];
    assert_eq!(
        copperlace_ruleset_render_structured_json_with_context(
            handle,
            rule.as_ptr(),
            keys.as_ptr(),
            invalid_values.as_ptr(),
            1,
            false,
            &mut output,
            &mut error,
        ),
        COPPERLACE_INVALID_ARGUMENT
    );
    assert!(output.is_null());
    assert!(!error.is_null());
    copperlace_string_free(error);
    error = ptr::null_mut();

    assert_eq!(
        copperlace_ruleset_render_structured_json(
            handle,
            invalid_rule.as_ptr().cast::<c_char>(),
            false,
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
