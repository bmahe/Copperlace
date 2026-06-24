use std::ffi::{CStr, CString};
use std::ptr;

use copperlace::ffi::{
    COPPERLACE_INVALID_ARGUMENT, COPPERLACE_OK, COPPERLACE_RENDER_ERROR, copperlace_ruleset_free,
    copperlace_ruleset_from_string, copperlace_ruleset_render,
    copperlace_ruleset_render_with_context, copperlace_string_free,
};

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
