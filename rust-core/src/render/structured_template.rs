use std::collections::HashMap;

use super::template::parse_for_statement;

pub(crate) struct ParsedTemplateConfig {
    pub(crate) value: hocon_rs::Value,
    pub(crate) loops: HashMap<String, StructuredLoopTemplate>,
}

pub(crate) struct StructuredLoopTemplate {
    pub(crate) variable_name: String,
    pub(crate) source_name: String,
    pub(crate) body: hocon_rs::Value,
}

pub(crate) fn transform_source(
    source: &str,
    marker_prefix: &str,
    next_marker: &mut usize,
) -> Result<(String, Vec<LoopSource>), String> {
    let mut parser = TemplateConfigParser {
        marker_prefix: marker_prefix.to_string(),
        next_marker: *next_marker,
        loops: Vec::new(),
    };
    let transformed = parser.transform(source)?;
    *next_marker = parser.next_marker;
    Ok((transformed, parser.loops))
}

struct TemplateConfigParser {
    marker_prefix: String,
    next_marker: usize,
    loops: Vec<LoopSource>,
}

pub(crate) struct LoopSource {
    pub(crate) marker: String,
    pub(crate) variable_name: String,
    pub(crate) source_name: String,
    pub(crate) body: String,
}

impl TemplateConfigParser {
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
