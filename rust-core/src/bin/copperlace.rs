use std::env;
use std::io::{self, Read};
use std::process;

use copperlace::{Copperlace, RenderContext, ruleset_from_file, ruleset_from_str};

fn main() {
    let args: Vec<String> = env::args().collect();

    match run(&args) {
        Ok(Some(output)) => print!("{output}"),
        Ok(None) => {}
        Err(error) => {
            eprintln!("{error}");
            process::exit(1);
        }
    }
}

fn run(args: &[String]) -> Result<Option<String>, String> {
    let Some(command) = args.get(1) else {
        return Ok(Some(help()));
    };

    if matches!(command.as_str(), "--help" | "-h" | "help") {
        return Ok(Some(help()));
    }

    if matches!(command.as_str(), "--version" | "-V" | "version") {
        return Ok(Some(format!("copperlace {}\n", env!("CARGO_PKG_VERSION"))));
    }

    if command == "check" {
        return check(args, 2);
    }

    let render_start_index = if command == "render" {
        2
    } else if command.starts_with('-') {
        1
    } else {
        return Err(format!("unknown command: {command}\n\n{}", help()));
    };

    render(args, render_start_index)
}

fn render(args: &[String], start_index: usize) -> Result<Option<String>, String> {
    let mut config = None;
    let mut rule = None;
    let mut context = RenderContext::new();
    let mut count = 1usize;
    let mut index = start_index;

    while index < args.len() {
        match args[index].as_str() {
            "--config" | "-c" => {
                index += 1;
                config = Some(required_config_value(args, index, "--config", render_help)?);
            }
            "--rule" | "-r" => {
                index += 1;
                rule = Some(required_value(args, index, "--rule", render_help)?);
            }
            "--count" | "-n" => {
                index += 1;
                let value = required_value(args, index, "--count", render_help)?;
                count = parse_count(&value)?;
            }
            "--set" => {
                index += 1;
                let value = required_value(args, index, "--set", render_help)?;
                let (key, context_value) = parse_context_binding(&value)?;
                context.insert(key, context_value);
            }
            "--help" | "-h" => return Ok(Some(render_help())),
            "--version" | "-V" => {
                return Ok(Some(format!("copperlace {}\n", env!("CARGO_PKG_VERSION"))));
            }
            unknown => return Err(format!("unknown argument: {unknown}\n\n{}", render_help())),
        }
        index += 1;
    }

    let config = config
        .ok_or_else(|| format!("missing required argument: --config\n\n{}", render_help()))?;
    let rule = rule.unwrap_or_else(|| "origin".to_string());

    let copperlace = copperlace_from_render_config(&config)?;
    let mut output = String::new();
    for render_index in 0..count {
        if render_index > 0 {
            output.push('\n');
        }
        output.push_str(
            &copperlace
                .render_with_context(&rule, context.clone())
                .map_err(|error| error.to_string())?,
        );
    }
    output.push('\n');
    Ok(Some(output))
}

fn copperlace_from_render_config(config: &str) -> Result<Copperlace, String> {
    if config == "-" {
        let input = read_stdin_config()?;
        Copperlace::from_str(&input).map_err(|error| error.to_string())
    } else {
        Copperlace::from_file(config).map_err(|error| error.to_string())
    }
}

fn read_stdin_config() -> Result<String, String> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| format!("failed to read config from stdin: {error}"))?;
    Ok(input)
}

fn check(args: &[String], start_index: usize) -> Result<Option<String>, String> {
    let mut config = None;
    let mut config_string = None;
    let mut index = start_index;

    while index < args.len() {
        match args[index].as_str() {
            "--config" | "-c" => {
                index += 1;
                config = Some(required_config_value(args, index, "--config", check_help)?);
            }
            "--string" | "-s" => {
                index += 1;
                config_string = Some(required_value(args, index, "--string", check_help)?);
            }
            "--help" | "-h" => return Ok(Some(check_help())),
            unknown => return Err(format!("unknown argument: {unknown}\n\n{}", check_help())),
        }
        index += 1;
    }

    match (config, config_string) {
        (Some(path), None) => {
            check_config(&path)?;
        }
        (None, Some(config)) => {
            ruleset_from_str(&config).map_err(|error| error.to_string())?;
        }
        (None, None) => {
            return Err(format!(
                "missing required argument: --config or --string\n\n{}",
                check_help()
            ));
        }
        (Some(_), Some(_)) => {
            return Err(format!(
                "only one of --config or --string may be provided\n\n{}",
                check_help()
            ));
        }
    }

    Ok(Some("OK\n".to_string()))
}

fn check_config(config: &str) -> Result<(), String> {
    if config == "-" {
        let input = read_stdin_config()?;
        ruleset_from_str(&input)
            .map(|_| ())
            .map_err(|error| error.to_string())
    } else {
        ruleset_from_file(config)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

fn required_value(
    args: &[String],
    index: usize,
    flag: &str,
    help: fn() -> String,
) -> Result<String, String> {
    let Some(value) = args.get(index) else {
        return Err(format!("missing value for {flag}\n\n{}", help()));
    };
    if value.starts_with('-') {
        return Err(format!("missing value for {flag}\n\n{}", help()));
    }
    Ok(value.clone())
}

fn required_config_value(
    args: &[String],
    index: usize,
    flag: &str,
    help: fn() -> String,
) -> Result<String, String> {
    let Some(value) = args.get(index) else {
        return Err(format!("missing value for {flag}\n\n{}", help()));
    };
    if value != "-" && value.starts_with('-') {
        return Err(format!("missing value for {flag}\n\n{}", help()));
    }
    Ok(value.clone())
}

fn parse_count(value: &str) -> Result<usize, String> {
    let count = value
        .parse::<usize>()
        .map_err(|_| format!("--count must be a positive integer: {value}"))?;
    if count == 0 {
        return Err("--count must be greater than zero".to_string());
    }
    Ok(count)
}

fn parse_context_binding(value: &str) -> Result<(String, String), String> {
    let Some((key, context_value)) = value.split_once('=') else {
        return Err(format!("--set must use key=value: {value}"));
    };
    if key.is_empty() {
        return Err("--set key must not be empty".to_string());
    }
    Ok((key.to_string(), context_value.to_string()))
}

fn help() -> String {
    format!(
        "Copperlace {}\n\n{}",
        env!("CARGO_PKG_VERSION"),
        top_level_help()
    )
}

fn top_level_help() -> String {
    "Usage:
  copperlace [render] --config <path> [--rule <name>] [--count <n>] [--set <key=value>...]
  copperlace check --config <path>
  copperlace check --config -
  copperlace check --string <config>
  copperlace --help
  copperlace --version

Commands:
  render    Render a named rule from a configuration file or stdin (optional when first argument is a flag)
  check     Parse and compile a configuration without rendering

Run `copperlace render --help` or `copperlace check --help` for command options.
"
    .to_string()
}

fn render_help() -> String {
    "Usage:
  copperlace [render] --config <path> [--rule <name>] [--count <n>] [--set <key=value>...]
  copperlace [render] -c <path> [-r <name>] [-n <n>] [--set <key=value>...]
  copperlace --help
  copperlace --version

Commands:
  render    Render a named rule from a configuration file or stdin (optional when first argument is a flag)
  check     Parse and compile a configuration without rendering

Render options:
  -c, --config <path>    configuration file to load, or - to read stdin
  -r, --rule <name>      Rule name to render (default: origin)
  -n, --count <n>        Number of outputs to render from one loaded config
      --set <key=value>  Initial render context value; may be repeated
  -h, --help             Show render help
  -V, --version          Show version"
        .to_string()
}

fn check_help() -> String {
    "Usage:
  copperlace check --config <path>
  copperlace check -c <path>
  copperlace check --string <config>
  copperlace check -s <config>

Check options:
  -c, --config <path>    configuration file to parse and compile, or - to read stdin
  -s, --string <config>   configuration string to parse and compile
  -h, --help             Show check help"
        .to_string()
}
