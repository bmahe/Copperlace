use std::env;
use std::process;

use copperlace::Copperlace;

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

    if command != "render" {
        return Err(format!("unknown command: {command}\n\n{}", help()));
    }

    let mut config = None;
    let mut rule = None;
    let mut count = 1usize;
    let mut index = 2;

    while index < args.len() {
        match args[index].as_str() {
            "--config" | "-c" => {
                index += 1;
                config = Some(required_value(args, index, "--config")?);
            }
            "--rule" | "-r" => {
                index += 1;
                rule = Some(required_value(args, index, "--rule")?);
            }
            "--count" | "-n" => {
                index += 1;
                let value = required_value(args, index, "--count")?;
                count = parse_count(&value)?;
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

    let copperlace = Copperlace::from_hocon_file(config).map_err(|error| error.to_string())?;
    let mut output = String::new();
    for render_index in 0..count {
        if render_index > 0 {
            output.push('\n');
        }
        output.push_str(
            &copperlace
                .render(&rule)
                .map_err(|error| error.to_string())?,
        );
    }
    output.push('\n');
    Ok(Some(output))
}

fn required_value(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    let Some(value) = args.get(index) else {
        return Err(format!("missing value for {flag}\n\n{}", render_help()));
    };
    if value.starts_with('-') {
        return Err(format!("missing value for {flag}\n\n{}", render_help()));
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

fn help() -> String {
    format!(
        "Copperlace {}\n\n{}\n",
        env!("CARGO_PKG_VERSION"),
        render_help()
    )
}

fn render_help() -> String {
    "Usage:
  copperlace render --config <path> [--rule <name>] [--count <n>]
  copperlace --help
  copperlace --version

Commands:
  render    Render a named rule from a HOCON config file

Render options:
  -c, --config <path>    HOCON config file to load
  -r, --rule <name>      Rule name to render (default: origin)
  -n, --count <n>        Number of outputs to render from one loaded config
  -h, --help             Show render help
  -V, --version          Show version"
        .to_string()
}
