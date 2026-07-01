use std::fs;
use std::io::Write;
use std::process::Command;
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

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
fn renders_with_limited_recursion_depth() {
    let config_path = write_temp_config(
        r#"
        origin = "x{origin}"
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args([
            "render",
            "--config",
            &config_path.to_string_lossy(),
            "--max-recursion-depth",
            "1",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "xx");

    let _ = fs::remove_file(config_path);
}

#[test]
fn rejects_invalid_max_recursion_depth() {
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
            "--max-recursion-depth",
            "-1",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("--max-recursion-depth must be a non-negative integer")
    );

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
fn renders_with_initial_context() {
    let config_path = write_temp_config(
        r#"
        context {
            name = "Mia"
        }
        origin = "Hello {name}"
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args([
            "render",
            "--config",
            &config_path.to_string_lossy(),
            "--rule",
            "origin",
            "--set",
            "name=Darcy",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "Hello Darcy"
    );

    let _ = fs::remove_file(config_path);
}

#[test]
fn renders_object_rule_as_formatted_json_by_default() {
    let config_path = write_temp_config(
        r#"
        name = ["Mia"]
        origin {
            title = "Hello {name}"
            items = ["one", "two"]
            count = 3
            active = true
            missing = null
            nested {
                value = "ok"
            }
        }
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args(["render", "--config", &config_path.to_string_lossy()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\n\t"));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(stdout.trim()).unwrap(),
        json!({
            "active": true,
            "count": 3,
            "items": ["one", "two"],
            "missing": null,
            "nested": {
                "value": "ok"
            },
            "title": "Hello Mia"
        })
    );

    let _ = fs::remove_file(config_path);
}

#[test]
fn renders_object_rule_as_compact_json() {
    let config_path = write_temp_config(
        r#"
        origin {
            title = "Hello"
            items = ["one", "two"]
        }
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args([
            "render",
            "--config",
            &config_path.to_string_lossy(),
            "--compact-json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        r#"{"items":["one","two"],"title":"Hello"}"#
    );

    let _ = fs::remove_file(config_path);
}

#[test]
fn structured_item_example_renders_valid_json() {
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args([
            "render",
            "--config",
            "../examples/structured_item.conf",
            "--set",
            "itemType=compass",
            "--set",
            "itemRarity=rare",
            "--set",
            "itemMaterial=moonstone",
            "--set",
            "itemEffect=spark",
            "--set",
            "itemLevel=2",
            "--set",
            "itemCharges=3",
            "--set",
            "history=save",
            "--set",
            "use=search",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\n\t"));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(stdout.trim()).unwrap(),
        json!({
            "description": "A moonstone compass once saved a lost caravan. It still releases 3 sparks when its bearer is searching in the dark.",
            "id": "rare-moonstone-compass-2nd",
            "level": "2",
            "name": "Rare Moonstone Compass",
            "properties": {
                "charges": "3",
                "effect": "spark",
                "material": "moonstone"
            },
            "rarity": "rare",
            "tags": ["rare", "compass", "spark"],
            "type": "compass"
        })
    );
}

#[test]
fn structured_render_uses_initial_context() {
    let config_path = write_temp_config(
        r#"
        context {
            name = "Mia"
        }
        origin {
            greeting = "Hello {name}"
        }
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args([
            "render",
            "--config",
            &config_path.to_string_lossy(),
            "--set",
            "name=Lina",
            "--compact-json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        r#"{"greeting":"Hello Lina"}"#
    );

    let _ = fs::remove_file(config_path);
}

#[test]
fn structured_render_from_stdin_infers_json_mode() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args([
            "render",
            "--config",
            "-",
            "--rule",
            "origin",
            "--compact-json",
        ])
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
            origin {
                title = "Hello"
            }
            "#,
        )
        .unwrap();

    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        r#"{"title":"Hello"}"#
    );
}

#[test]
fn count_with_formatted_structured_json_separates_values_with_single_newline() {
    let config_path = write_temp_config(
        r#"
        origin {
            title = "Hello"
        }
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args([
            "render",
            "--config",
            &config_path.to_string_lossy(),
            "--count",
            "2",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.as_ref(),
        "{\n\t\"title\": \"Hello\"\n}\n{\n\t\"title\": \"Hello\"\n}\n"
    );

    let _ = fs::remove_file(config_path);
}

#[test]
fn count_with_compact_structured_json_outputs_json_lines() {
    let config_path = write_temp_config(
        r#"
        origin {
            title = "Hello"
        }
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args([
            "render",
            "--config",
            &config_path.to_string_lossy(),
            "--count",
            "2",
            "--compact-json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .collect::<Vec<_>>(),
        vec![r#"{"title":"Hello"}"#, r#"{"title":"Hello"}"#]
    );

    let _ = fs::remove_file(config_path);
}

#[test]
fn compact_json_rejects_text_rules() {
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
            "--compact-json",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("--compact-json can only be used with object-valued structured rules")
    );

    let _ = fs::remove_file(config_path);
}

#[test]
fn compact_json_rejects_list_rules() {
    let config_path = write_temp_config(
        r#"
        origin = ["Mia"]
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args([
            "render",
            "--config",
            &config_path.to_string_lossy(),
            "--compact-json",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("--compact-json can only be used with object-valued structured rules")
    );

    let _ = fs::remove_file(config_path);
}

#[test]
fn default_list_rule_rendering_remains_text_choice() {
    let config_path = write_temp_config(
        r#"
        origin = ["Mia"]
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
fn repeated_context_keys_use_last_value() {
    let config_path = write_temp_config(
        r#"
        origin = "{name}"
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args([
            "render",
            "--config",
            &config_path.to_string_lossy(),
            "--set",
            "name=Mia",
            "--set",
            "name=Lina",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "Lina");

    let _ = fs::remove_file(config_path);
}

#[test]
fn renders_empty_context_value() {
    let config_path = write_temp_config(
        r#"
        origin = "Hello{name}"
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args([
            "render",
            "--config",
            &config_path.to_string_lossy(),
            "--set",
            "name=",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "Hello");

    let _ = fs::remove_file(config_path);
}

#[test]
fn render_count_reuses_initial_context_without_persisting_overwrites() {
    let config_path = write_temp_config(
        r#"
        next = "Darcy"
        origin = "{name}{% name:=next %}"
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args([
            "render",
            "--config",
            &config_path.to_string_lossy(),
            "--set",
            "name=Mia",
            "--count",
            "2",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .collect::<Vec<_>>(),
        vec!["Mia", "Mia"]
    );

    let _ = fs::remove_file(config_path);
}

#[test]
fn rejects_set_without_equals() {
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
            "--set",
            "name",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--set must use key=value"));

    let _ = fs::remove_file(config_path);
}

#[test]
fn rejects_set_with_empty_key() {
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
            "--set",
            "=Mia",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--set key must not be empty"));

    let _ = fs::remove_file(config_path);
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
fn checks_config_from_stdin() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args(["check", "--config", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(br#"origin = "Mia""#)
        .unwrap();

    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "OK\n");
}

#[test]
fn checks_config_from_stdin_with_short_flag() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args(["check", "-c", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(br#"origin = "Mia""#)
        .unwrap();

    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "OK\n");
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
fn check_rejects_invalid_config_string() {
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args(["check", "--string", "[\"Mia\"]"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("failed to parse config"));
}

#[test]
fn check_rejects_invalid_config_from_stdin() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args(["check", "--config", "-"])
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(br#"["Mia"]"#)
        .unwrap();

    let output = child.wait_with_output().unwrap();

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
fn check_rejects_stdin_config_with_string_input() {
    let output = Command::new(env!("CARGO_BIN_EXE_copperlace"))
        .args(["check", "--config", "-", "--string", r#"origin = "Darcy""#])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("only one of --config or --string may be provided")
    );
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
