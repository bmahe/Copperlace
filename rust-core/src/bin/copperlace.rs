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
