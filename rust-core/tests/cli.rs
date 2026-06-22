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
        String::from_utf8_lossy(&output.stderr)
            .contains("usage: copperlace render --config <path> --rule <name>")
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
