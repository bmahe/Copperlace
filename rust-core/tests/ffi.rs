use std::ffi::{CStr, CString};
use std::ptr;

use copperlace::ffi::{
    COPPERLACE_OK, COPPERLACE_RENDER_ERROR, copperlace_ruleset_free,
    copperlace_ruleset_from_string, copperlace_ruleset_render, copperlace_string_free,
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
