use std::collections::HashMap;

use hocon_rs::ConfigOptions;

use super::template::parse_for_statement;

pub(crate) struct ParsedTemplateConfig {
    pub(crate) value: hocon_rs::Value,
    pub(crate) loops: HashMap<String, StructuredLoopTemplate>,
}

pub(crate) struct StructuredLoopTemplate {
    pub(crate) variable_name: String,
    pub(crate) source_name: String,
    pub(crate) body: Box<ParsedTemplateConfig>,
}

pub(crate) fn parse_config(
    source: &str,
    options: Option<ConfigOptions>,
) -> Result<ParsedTemplateConfig, String> {
    let mut parser = TemplateConfigParser::new(source);
    let transformed = parser.transform(source)?;
    let value = hocon_rs::Config::parse_str::<hocon_rs::Value>(&transformed, options.clone())
        .map_err(|error| format!("{error:?}"))?;

    let mut loops = HashMap::new();
    for loop_source in parser.loops {
        let body_source = format!("structured_template_value = {}", loop_source.body);
        let body = parse_config(&body_source, options.clone())?;
        let hocon_rs::Value::Object(mut values) = body.value else {
            return Err("structured loop body must contain one HOCON value".to_string());
        };
        let Some(body_value) = values.remove("structured_template_value") else {
            return Err("structured loop body must contain one HOCON value".to_string());
        };
        if !values.is_empty() {
            return Err("structured loop body must contain one HOCON value".to_string());
        }

        loops.insert(
            loop_source.marker,
            StructuredLoopTemplate {
                variable_name: loop_source.variable_name,
                source_name: loop_source.source_name,
                body: Box::new(ParsedTemplateConfig {
                    value: body_value,
                    loops: body.loops,
                }),
            },
        );
    }

    Ok(ParsedTemplateConfig { value, loops })
}

struct TemplateConfigParser {
    marker_prefix: String,
    next_marker: usize,
    loops: Vec<LoopSource>,
}

struct LoopSource {
    marker: String,
    variable_name: String,
    source_name: String,
    body: String,
}

impl TemplateConfigParser {
    fn new(source: &str) -> Self {
        let mut suffix = 0;
        let marker_prefix = loop {
            let candidate = format!("__copperlace_structured_loop_{suffix}_");
            if !source.contains(&candidate) {
                break candidate;
            }
            suffix += 1;
        };

        TemplateConfigParser {
            marker_prefix,
            next_marker: 0,
            loops: Vec::new(),
        }
    }

    fn transform(&mut self, source: &str) -> Result<String, String> {
        let mut output = String::with_capacity(source.len());
        let mut copied_until = 0;
        let mut cursor = 0;

        while let Some((start, end, statement)) = next_directive(source, cursor)? {
            let Some(statement) = statement.strip_prefix("for ") else {
                return Err(format!(
                    "unexpected structured template directive: {statement}"
                ));
            };
            let full_statement = format!("for {statement}");
            let (variable_name, source_name) =
                parse_for_statement(&full_statement).map_err(|error| error.to_string())?;
            let (body_end, closing_end) = find_matching_endfor(source, end)?;
            output.push_str(&source[copied_until..start]);

            let marker = format!("{}{}__", self.marker_prefix, self.next_marker);
            self.next_marker += 1;
            output.push('"');
            output.push_str(&marker);
            output.push('"');

            self.loops.push(LoopSource {
                marker,
                variable_name,
                source_name,
                body: source[end..body_end].to_string(),
            });
            copied_until = closing_end;
            cursor = closing_end;
        }

        output.push_str(&source[copied_until..]);
        Ok(output)
    }
}

fn find_matching_endfor(source: &str, after_opening: usize) -> Result<(usize, usize), String> {
    let mut depth = 1usize;
    let mut cursor = after_opening;

    while let Some((start, end, statement)) = next_directive(source, cursor)? {
        if statement.starts_with("for ") {
            depth += 1;
        } else if statement == "endfor" {
            depth -= 1;
            if depth == 0 {
                return Ok((start, end));
            }
        } else {
            return Err(format!(
                "unexpected structured template directive: {statement}"
            ));
        }
        cursor = end;
    }

    Err("missing endfor for structured array loop".to_string())
}

fn next_directive(source: &str, start: usize) -> Result<Option<(usize, usize, String)>, String> {
    let bytes = source.as_bytes();
    let mut cursor = start;
    let mut quote = false;
    let mut triple_quote = false;
    let mut escaped = false;

    while cursor < bytes.len() {
        if quote {
            if triple_quote {
                if bytes[cursor..].starts_with(b"\"\"\"") {
                    cursor += 3;
                    quote = false;
                    triple_quote = false;
                } else {
                    cursor += 1;
                }
                continue;
            }

            let character = bytes[cursor];
            cursor += 1;
            if escaped {
                escaped = false;
            } else if character == b'\\' {
                escaped = true;
            } else if character == b'"' {
                quote = false;
            }
            continue;
        }

        if bytes[cursor..].starts_with(b"\"\"\"") {
            quote = true;
            triple_quote = true;
            cursor += 3;
            continue;
        }
        if bytes[cursor] == b'"' {
            quote = true;
            cursor += 1;
            continue;
        }
        if bytes[cursor] == b'#' || (bytes[cursor] == b'/' && bytes.get(cursor + 1) == Some(&b'/'))
        {
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                cursor += 1;
            }
            continue;
        }
        if bytes[cursor..].starts_with(b"{%") {
            let directive_start = cursor;
            let Some(relative_end) = source[cursor + 2..].find("%}") else {
                return Err("unmatched opening structured template delimiter".to_string());
            };
            let directive_end = cursor + 2 + relative_end + 2;
            let statement = source[cursor + 2..directive_end - 2].trim().to_string();
            return Ok(Some((directive_start, directive_end, statement)));
        }
        cursor += 1;
    }

    Ok(None)
}
