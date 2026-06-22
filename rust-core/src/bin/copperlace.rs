use std::env;
use std::process;

use copperlace::render_hocon_file;

fn main() {
    let args: Vec<String> = env::args().collect();

    match run(&args) {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("{error}");
            process::exit(1);
        }
    }
}

fn run(args: &[String]) -> Result<String, String> {
    let Some(command) = args.get(1) else {
        return Err(usage());
    };

    if command != "render" {
        return Err(usage());
    }

    let mut config = None;
    let mut rule = None;
    let mut index = 2;

    while index < args.len() {
        match args[index].as_str() {
            "--config" => {
                index += 1;
                config = args.get(index).cloned();
            }
            "--rule" => {
                index += 1;
                rule = args.get(index).cloned();
            }
            "--help" | "-h" => return Err(usage()),
            unknown => return Err(format!("unknown argument: {unknown}\n\n{}", usage())),
        }
        index += 1;
    }

    let config = config.ok_or_else(usage)?;
    let rule = rule.ok_or_else(usage)?;

    render_hocon_file(config, &rule).map_err(|error| error.to_string())
}

fn usage() -> String {
    "usage: copperlace render --config <path> --rule <name>".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn rejects_missing_arguments() {
        let args = vec!["copperlace".to_string(), "render".to_string()];

        assert!(run(&args).unwrap_err().contains("usage:"));
    }

    #[test]
    fn renders_config_file() {
        let config_path = write_temp_config(
            r#"
            name = ["Mia"]
            origin = "{name}"
            "#,
        );
        let args = vec![
            "copperlace".to_string(),
            "render".to_string(),
            "--config".to_string(),
            config_path.to_string_lossy().to_string(),
            "--rule".to_string(),
            "origin".to_string(),
        ];

        assert_eq!(run(&args).unwrap(), "Mia");

        let _ = fs::remove_file(config_path);
    }

    #[test]
    fn returns_error_for_missing_rule() {
        let config_path = write_temp_config(
            r#"
            origin = "{missing}"
            "#,
        );
        let args = vec![
            "copperlace".to_string(),
            "render".to_string(),
            "--config".to_string(),
            config_path.to_string_lossy().to_string(),
            "--rule".to_string(),
            "origin".to_string(),
        ];

        assert!(run(&args).unwrap_err().contains("unknown rule"));

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
}
