use std::fs;
use std::io::Write;
use std::process::Command;
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn rejects_missing_arguments() {
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .arg("render")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("missing required argument: --config")
    );
}

#[test]
fn renders_config_file() {
    let config_path = write_temp_config(
        r#"
        name = ["Mia"]
        origin = "{name}"
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args([
            "render",
            "--config",
            &config_path.to_string_lossy(),
            "--rule",
            "origin",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "Mia");

    let _ = fs::remove_file(config_path);
}

#[test]
fn defaults_to_origin_rule() {
    let config_path = write_temp_config(
        r#"
        origin = "Mia"
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args(["render", "--config", &config_path.to_string_lossy()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "Mia");

    let _ = fs::remove_file(config_path);
}

#[test]
fn default_command_renders_origin_rule() {
    let config_path = write_temp_config(
        r#"
        origin = "Mia"
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args(["--config", &config_path.to_string_lossy()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "Mia");

    let _ = fs::remove_file(config_path);
}

#[test]
fn explicit_rule_overrides_origin_default() {
    let config_path = write_temp_config(
        r#"
        origin = "Mia"
        other = "Darcy"
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args([
            "render",
            "--config",
            &config_path.to_string_lossy(),
            "--rule",
            "other",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "Darcy");

    let _ = fs::remove_file(config_path);
}

#[test]
fn default_command_renders_explicit_rule_with_short_flags() {
    let config_path = write_temp_config(
        r#"
        origin = "Mia"
        other = "Darcy"
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args(["-c", &config_path.to_string_lossy(), "-r", "other"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "Darcy");

    let _ = fs::remove_file(config_path);
}

#[test]
fn renders_config_file_with_short_flags() {
    let config_path = write_temp_config(
        r#"
        name = ["Mia"]
        origin = "{name}"
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args([
            "render",
            "-c",
            &config_path.to_string_lossy(),
            "-r",
            "origin",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "Mia");

    let _ = fs::remove_file(config_path);
}

#[test]
fn renders_config_from_stdin() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args(["render", "--config", "-", "--rule", "origin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(
            br#"
            name = ["Mia"]
            origin = "{name}"
            "#,
        )
        .unwrap();

    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "Mia");
}

#[test]
fn default_command_renders_config_from_stdin_with_short_flag() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args(["-c", "-", "-r", "origin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(
            br#"
            origin = "Mia"
            "#,
        )
        .unwrap();

    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "Mia");
}

#[test]
fn default_command_renders_multiple_outputs_from_one_config() {
    let config_path = write_temp_config(
        r#"
        name = ["Mia"]
        origin = "{name}"
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args(["--count", "2", "--config", &config_path.to_string_lossy()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).lines().count(), 2);

    let _ = fs::remove_file(config_path);
}

#[test]
fn renders_multiple_outputs_from_one_config() {
    let config_path = write_temp_config(
        r#"
        name = ["Mia"]
        origin = "{name}"
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args([
            "render",
            "--config",
            &config_path.to_string_lossy(),
            "--rule",
            "origin",
            "--count",
            "3",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).lines().count(), 3);

    let _ = fs::remove_file(config_path);
}

#[test]
fn rejects_zero_count() {
    let config_path = write_temp_config(
        r#"
        origin = "Mia"
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args([
            "render",
            "--config",
            &config_path.to_string_lossy(),
            "--rule",
            "origin",
            "--count",
            "0",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--count must be greater than zero"));

    let _ = fs::remove_file(config_path);
}

#[test]
fn rejects_missing_rule_value() {
    let config_path = write_temp_config(
        r#"
        origin = "Mia"
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args([
            "render",
            "--config",
            &config_path.to_string_lossy(),
            "--rule",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing value for --rule"));

    let _ = fs::remove_file(config_path);
}

#[test]
fn checks_config_file() {
    let config_path = write_temp_config(
        r#"
        origin = "Mia"
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args(["check", "--config", &config_path.to_string_lossy()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "OK\n");

    let _ = fs::remove_file(config_path);
}

#[test]
fn checks_config_file_with_short_flag() {
    let config_path = write_temp_config(
        r#"
        origin = "Mia"
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args(["check", "-c", &config_path.to_string_lossy()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "OK\n");

    let _ = fs::remove_file(config_path);
}

#[test]
fn checks_config_string() {
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args(["check", "--string", r#"origin = "Mia""#])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "OK\n");
}

#[test]
fn check_rejects_invalid_hocon_string() {
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args(["check", "--string", "[\"Mia\"]"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("failed to parse config"));
}

#[test]
fn check_rejects_invalid_copperlace_config() {
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args([
            "check",
            "--string",
            r#"origin = "{name | missing_processor}""#,
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown processor"));
}

#[test]
fn check_rejects_missing_input() {
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .arg("check")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("missing required argument: --config or --string")
    );
}

#[test]
fn check_rejects_multiple_inputs() {
    let config_path = write_temp_config(
        r#"
        origin = "Mia"
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args([
            "check",
            "--config",
            &config_path.to_string_lossy(),
            "--string",
            r#"origin = "Darcy""#,
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("only one of --config or --string may be provided")
    );

    let _ = fs::remove_file(config_path);
}

#[test]
fn check_rejects_missing_string_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args(["check", "--string"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing value for --string"));
}

#[test]
fn prints_check_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args(["check", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Check options:"));
}

#[test]
fn prints_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .arg("--help")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage:"));
}

#[test]
fn prints_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .arg("--version")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn rejects_unknown_non_flag_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .arg("validate")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown command: validate"));
}

#[test]
fn returns_error_for_missing_rule() {
    let config_path = write_temp_config(
        r#"
        origin = "{missing}"
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args([
            "render",
            "--config",
            &config_path.to_string_lossy(),
            "--rule",
            "origin",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown rule"));

    let _ = fs::remove_file(config_path);
}

fn write_temp_config(contents: &str) -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("copperlace-{unique}.conf"));
    fs::write(&path, contents).unwrap();
    path
}
