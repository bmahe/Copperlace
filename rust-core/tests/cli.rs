use std::fs;
use std::process::Command;
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
