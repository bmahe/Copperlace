use std::collections::HashMap;
use std::path::{Path, PathBuf};

use hocon_rs::parser::HoconParser;
use hocon_rs::parser::read::StrRead;
use hocon_rs::raw::field::ObjectField;
use hocon_rs::raw::include::Inclusion;
use hocon_rs::raw::raw_object::RawObject;
use hocon_rs::raw::raw_string::RawString;
use hocon_rs::raw::raw_value::RawValue;
use hocon_rs::{Config, ConfigOptions, Value};

use super::structured_template::{ParsedTemplateConfig, StructuredLoopTemplate, transform_source};

enum LoadError {
    MarkerCollision,
    Parse(String),
}

type LoadResult<T> = Result<T, LoadError>;

#[derive(Clone, Copy)]
enum IncludeLocation {
    File,
    Classpath,
    Either,
}

struct IncludeSpec {
    path: String,
    required: bool,
    location: IncludeLocation,
}

struct ParsedRaw {
    value: RawObject,
    loops: HashMap<String, LoopHeader>,
}

struct LoopHeader {
    variable_name: String,
    source_name: String,
}

struct TemplateConfigLoader {
    options: ConfigOptions,
    marker_prefix: String,
    next_marker: usize,
}

pub(crate) fn load_str(
    source: &str,
    options: Option<ConfigOptions>,
) -> Result<ParsedTemplateConfig, String> {
    retry(|attempt| {
        let mut loader = TemplateConfigLoader::new(options.clone().unwrap_or_default(), attempt);
        loader.parse_document(source, &mut Vec::new())
    })
}

pub(crate) fn load_file(
    path: &Path,
    options: ConfigOptions,
) -> Result<ParsedTemplateConfig, String> {
    retry(|attempt| {
        let mut loader = TemplateConfigLoader::new(options.clone(), attempt);
        loader.parse_file(path)
    })
}

fn retry<T>(mut action: impl FnMut(usize) -> LoadResult<T>) -> Result<T, String> {
    for attempt in 0.. {
        match action(attempt) {
            Ok(value) => return Ok(value),
            Err(LoadError::MarkerCollision) => continue,
            Err(LoadError::Parse(message)) => return Err(message),
        }
    }
    unreachable!()
}

impl TemplateConfigLoader {
    fn new(options: ConfigOptions, attempt: usize) -> Self {
        Self {
            options,
            marker_prefix: format!("__copperlace_internal_{attempt}_"),
            next_marker: 0,
        }
    }

    fn parse_file(&mut self, path: &Path) -> LoadResult<ParsedTemplateConfig> {
        let Some(paths) = config_paths(path) else {
            let value = Config::load::<Value>(path, Some(self.options.clone()))
                .map_err(|error| LoadError::Parse(format!("{error:?}")))?;
            return Ok(ParsedTemplateConfig {
                value,
                loops: HashMap::new(),
            });
        };
        let mut merged = ParsedRaw {
            value: RawObject::default(),
            loops: HashMap::new(),
        };
        let mut active = Vec::new();
        for path in paths {
            let parsed = self.load_path(&path, &mut active)?;
            merged.value.extend(parsed.value.into_inner());
            merged.loops.extend(parsed.loops);
        }
        self.finish(merged)
    }

    fn parse_document(
        &mut self,
        source: &str,
        active: &mut Vec<PathBuf>,
    ) -> LoadResult<ParsedTemplateConfig> {
        let parsed = self.parse_raw(source, active)?;
        self.finish(parsed)
    }

    fn finish(&self, parsed: ParsedRaw) -> LoadResult<ParsedTemplateConfig> {
        let value = Config::from(parsed.value)
            .resolve::<Value>()
            .map_err(|error| LoadError::Parse(format!("resolving configuration: {error:?}")))?;
        let mut bodies = HashMap::new();
        let value = extract_loop_bodies(value, &self.marker_prefix, &mut bodies)?;
        let loops = parsed
            .loops
            .into_iter()
            .filter_map(|(marker, header)| {
                bodies.remove(&marker).map(|body| {
                    (
                        marker,
                        StructuredLoopTemplate {
                            variable_name: header.variable_name,
                            source_name: header.source_name,
                            body,
                        },
                    )
                })
            })
            .collect();
        Ok(ParsedTemplateConfig { value, loops })
    }

    fn parse_raw(&mut self, source: &str, active: &mut Vec<PathBuf>) -> LoadResult<ParsedRaw> {
        if source.contains(&self.marker_prefix) {
            return Err(LoadError::MarkerCollision);
        }
        let (transformed, loop_sources) =
            transform_source(source, &self.marker_prefix, &mut self.next_marker)
                .map_err(LoadError::Parse)?;
        // hocon-rs loads includes while parsing. Placeholders let us preprocess
        // each included .conf first, then give hocon-rs the same raw object tree.
        let (without_includes, mut includes) = self.extract_includes(&transformed);
        let mut value =
            HoconParser::with_options(StrRead::new(&without_includes), self.options.clone())
                .parse()
                .map_err(|error| LoadError::Parse(format!("parsing configuration: {error:?}")))?;
        let mut loops = HashMap::new();
        self.inject_includes(&mut value, &mut includes, &mut loops, active)?;
        for loop_source in loop_sources {
            let body_source = format!("structured_template_value = {}", loop_source.body);
            let body = self.parse_raw(&body_source, active)?;
            let body_value = single_loop_body(body.value)?;
            let envelope = RawValue::object(vec![
                (
                    RawString::quoted(format!("{}loop_marker", self.marker_prefix)),
                    RawValue::quoted_string(&loop_source.marker),
                ),
                (
                    RawString::quoted(format!("{}loop_body", self.marker_prefix)),
                    body_value,
                ),
            ]);
            if !replace_loop_marker(&mut value, &loop_source.marker, &envelope)? {
                return Err(LoadError::Parse(
                    "structured loop marker was not found".to_string(),
                ));
            }
            loops.extend(body.loops);
            loops.insert(
                loop_source.marker,
                LoopHeader {
                    variable_name: loop_source.variable_name,
                    source_name: loop_source.source_name,
                },
            );
        }
        Ok(ParsedRaw { value, loops })
    }

    fn inject_includes(
        &mut self,
        object: &mut RawObject,
        includes: &mut HashMap<String, IncludeSpec>,
        loops: &mut HashMap<String, LoopHeader>,
        active: &mut Vec<PathBuf>,
    ) -> LoadResult<()> {
        for field in object.iter_mut() {
            match field {
                ObjectField::KeyValue { key, value, .. } => {
                    let path = key.as_path();
                    if path.len() == 1
                        && let Some(spec) = includes.remove(path[0])
                    {
                        let parsed = self.load_include(&spec, active)?;
                        loops.extend(parsed.loops);
                        *field = ObjectField::inclusion(Inclusion::new(
                            spec.path.into(),
                            spec.required,
                            None,
                            Some(Box::new(parsed.value)),
                        ));
                    } else {
                        self.inject_value(value, includes, loops, active)?;
                    }
                }
                ObjectField::Inclusion { .. } | ObjectField::NewlineComment(_) => {}
            }
        }
        Ok(())
    }

    fn inject_value(
        &mut self,
        value: &mut RawValue,
        includes: &mut HashMap<String, IncludeSpec>,
        loops: &mut HashMap<String, LoopHeader>,
        active: &mut Vec<PathBuf>,
    ) -> LoadResult<()> {
        match value {
            RawValue::Object(object) => self.inject_includes(object, includes, loops, active)?,
            RawValue::Array(array) => {
                for element in array.iter_mut() {
                    self.inject_value(element, includes, loops, active)?;
                }
            }
            RawValue::Concat(_) => {
                let RawValue::Concat(concat) = std::mem::replace(value, RawValue::Null) else {
                    unreachable!()
                };
                let (mut values, spaces) = concat.into_inner();
                for element in &mut values {
                    self.inject_value(element, includes, loops, active)?;
                }
                *value = RawValue::concat(values, spaces)
                    .map_err(|error| LoadError::Parse(format!("{error:?}")))?;
            }
            RawValue::AddAssign(inner) => self.inject_value(inner, includes, loops, active)?,
            _ => {}
        }
        Ok(())
    }

    fn load_include(
        &mut self,
        spec: &IncludeSpec,
        active: &mut Vec<PathBuf>,
    ) -> LoadResult<ParsedRaw> {
        let Some(paths) = self.resolve_include_paths(spec)? else {
            if spec.required {
                return Err(LoadError::Parse(format!(
                    "required include not found: {}",
                    spec.path
                )));
            }
            return Ok(ParsedRaw {
                value: RawObject::default(),
                loops: HashMap::new(),
            });
        };
        let mut merged = ParsedRaw {
            value: RawObject::default(),
            loops: HashMap::new(),
        };
        for path in paths {
            let parsed = self.load_path(&path, active)?;
            merged.value.extend(parsed.value.into_inner());
            merged.loops.extend(parsed.loops);
        }
        Ok(merged)
    }

    fn load_path(&mut self, path: &Path, active: &mut Vec<PathBuf>) -> LoadResult<ParsedRaw> {
        if path
            .extension()
            .is_some_and(|extension| extension == "conf")
        {
            let canonical = path
                .canonicalize()
                .map_err(|error| LoadError::Parse(format!("{}: {error}", path.display())))?;
            if active.contains(&canonical) {
                return Err(LoadError::Parse(format!(
                    "include cycle at {}",
                    path.display()
                )));
            }
            if active.len() >= self.options.max_include_depth {
                return Err(LoadError::Parse(format!(
                    "include depth exceeded at {}",
                    path.display()
                )));
            }
            let source = std::fs::read_to_string(path)
                .map_err(|error| LoadError::Parse(format!("{}: {error}", path.display())))?;
            active.push(canonical);
            let parsed = self.parse_raw(&source, active);
            active.pop();
            parsed.map_err(|error| match error {
                LoadError::Parse(message) => {
                    LoadError::Parse(format!("{}: {message}", path.display()))
                }
                other => other,
            })
        } else {
            let quoted = serde_json::to_string(&path.to_string_lossy().to_string()).unwrap();
            let include = format!("include required(file({quoted}))");
            let mut parsed =
                HoconParser::with_options(StrRead::new(&include), self.options.clone())
                    .parse()
                    .map_err(|error| LoadError::Parse(format!("{error:?}")))?;
            let Some(ObjectField::Inclusion { inclusion, .. }) = parsed.pop() else {
                return Err(LoadError::Parse(
                    "failed to load included config".to_string(),
                ));
            };
            Ok(ParsedRaw {
                value: *inclusion.val.ok_or_else(|| {
                    LoadError::Parse("failed to load included config".to_string())
                })?,
                loops: HashMap::new(),
            })
        }
    }

    fn resolve_include_paths(&self, spec: &IncludeSpec) -> LoadResult<Option<Vec<PathBuf>>> {
        let path = Path::new(&spec.path);
        if matches!(spec.location, IncludeLocation::Classpath)
            && path.is_absolute()
            && !self.options.classpath.is_empty()
        {
            return Err(LoadError::Parse(
                "absolute path in classpath include".to_string(),
            ));
        }
        if !matches!(spec.location, IncludeLocation::Classpath)
            && let Some(paths) = config_paths(path)
        {
            return Ok(Some(paths));
        }
        if !matches!(spec.location, IncludeLocation::File) {
            for root in self.options.classpath.iter() {
                if let Some(paths) = config_paths(&Path::new(root).join(path)) {
                    return Ok(Some(paths));
                }
            }
        }
        Ok(None)
    }

    fn extract_includes(&mut self, source: &str) -> (String, HashMap<String, IncludeSpec>) {
        let bytes = source.as_bytes();
        let mut output = String::with_capacity(source.len());
        let mut includes = HashMap::new();
        let mut cursor = 0;
        let mut copied_until = 0;
        let mut containers = Vec::new();
        let mut field_start = true;
        while cursor < bytes.len() {
            if bytes[cursor..].starts_with(b"\"\"\"") {
                cursor += 3;
                while cursor < bytes.len() && !bytes[cursor..].starts_with(b"\"\"\"") {
                    cursor += 1;
                }
                cursor = (cursor + 3).min(bytes.len());
                field_start = false;
                continue;
            }
            if bytes[cursor] == b'"' {
                cursor += 1;
                while cursor < bytes.len() {
                    if bytes[cursor] == b'\\' {
                        cursor = (cursor + 2).min(bytes.len());
                    } else if bytes[cursor] == b'"' {
                        cursor += 1;
                        break;
                    } else {
                        cursor += 1;
                    }
                }
                field_start = false;
                continue;
            }
            if bytes[cursor] == b'#' || bytes[cursor..].starts_with(b"//") {
                while cursor < bytes.len() && bytes[cursor] != b'\n' {
                    cursor += 1;
                }
                continue;
            }
            if bytes[cursor..].starts_with(b"${") {
                cursor += 2;
                while cursor < bytes.len() && bytes[cursor] != b'}' {
                    cursor += 1;
                }
                cursor = (cursor + 1).min(bytes.len());
                field_start = false;
                continue;
            }
            if field_start
                && containers.last() != Some(&b'[')
                && let Some((end, spec)) = parse_include_at(source, cursor)
            {
                let marker = format!("{}include_{}__", self.marker_prefix, self.next_marker);
                self.next_marker += 1;
                output.push_str(&source[copied_until..cursor]);
                output.push_str(&format!("\"{marker}\" = null"));
                includes.insert(marker, spec);
                copied_until = end;
                cursor = end;
                field_start = false;
                continue;
            }
            match bytes[cursor] {
                b'{' => {
                    containers.push(b'{');
                    field_start = true;
                }
                b'[' => {
                    containers.push(b'[');
                    field_start = false;
                }
                b'}' | b']' => {
                    containers.pop();
                    field_start = false;
                }
                b',' => field_start = containers.last() != Some(&b'['),
                b'\n' | b'\r' => field_start = containers.last() != Some(&b'['),
                b' ' | b'\t' => {}
                _ => field_start = false,
            }
            cursor += 1;
        }
        output.push_str(&source[copied_until..]);
        (output, includes)
    }
}

fn single_loop_body(object: RawObject) -> LoadResult<RawValue> {
    let mut body = None;
    for field in object.into_inner() {
        match field {
            ObjectField::KeyValue { key, value, .. }
                if key.as_path().len() == 1
                    && key.as_path()[0] == "structured_template_value"
                    && body.is_none() =>
            {
                body = Some(value);
            }
            ObjectField::NewlineComment(_) => {}
            _ => {
                return Err(LoadError::Parse(
                    "structured loop body must contain one HOCON value".to_string(),
                ));
            }
        }
    }
    body.ok_or_else(|| {
        LoadError::Parse("structured loop body must contain one HOCON value".to_string())
    })
}

fn replace_loop_marker(
    object: &mut RawObject,
    marker: &str,
    envelope: &RawValue,
) -> LoadResult<bool> {
    for field in object.iter_mut() {
        if let ObjectField::KeyValue { value, .. } = field
            && replace_marker_value(value, marker, envelope)?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn replace_marker_value(
    value: &mut RawValue,
    marker: &str,
    envelope: &RawValue,
) -> LoadResult<bool> {
    if matches!(value, RawValue::String(RawString::QuotedString(text)) if text == marker) {
        *value = envelope.clone();
        return Ok(true);
    }
    match value {
        RawValue::Object(object) => replace_loop_marker(object, marker, envelope),
        RawValue::Array(array) => {
            for entry in array.iter_mut() {
                if replace_marker_value(entry, marker, envelope)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        RawValue::Concat(_) => {
            let RawValue::Concat(concat) = std::mem::replace(value, RawValue::Null) else {
                unreachable!()
            };
            let (mut values, spaces) = concat.into_inner();
            let mut found = false;
            for entry in &mut values {
                if replace_marker_value(entry, marker, envelope)? {
                    found = true;
                    break;
                }
            }
            *value = RawValue::concat(values, spaces)
                .map_err(|error| LoadError::Parse(format!("{error:?}")))?;
            Ok(found)
        }
        RawValue::AddAssign(inner) => replace_marker_value(inner, marker, envelope),
        _ => Ok(false),
    }
}

fn extract_loop_bodies(
    value: Value,
    marker_prefix: &str,
    bodies: &mut HashMap<String, Value>,
) -> LoadResult<Value> {
    match value {
        Value::Object(mut fields) => {
            let marker_key = format!("{marker_prefix}loop_marker");
            let body_key = format!("{marker_prefix}loop_body");
            if fields.contains_key(&marker_key) {
                if fields.len() != 2 || !fields.contains_key(&body_key) {
                    return Err(LoadError::Parse(
                        "structured loop body must contain one HOCON value".to_string(),
                    ));
                }
                let Some(Value::String(marker)) = fields.remove(&marker_key) else {
                    return Err(LoadError::Parse(
                        "invalid structured loop marker".to_string(),
                    ));
                };
                let body = fields.remove(&body_key).unwrap();
                let body = extract_loop_bodies(body, marker_prefix, bodies)?;
                bodies.insert(marker.clone(), body);
                return Ok(Value::String(marker));
            }
            let fields = fields
                .into_iter()
                .map(|(key, value)| {
                    extract_loop_bodies(value, marker_prefix, bodies).map(|value| (key, value))
                })
                .collect::<LoadResult<HashMap<_, _>>>()?;
            Ok(Value::Object(fields))
        }
        Value::Array(entries) => Ok(Value::Array(
            entries
                .into_iter()
                .map(|entry| extract_loop_bodies(entry, marker_prefix, bodies))
                .collect::<LoadResult<Vec<_>>>()?,
        )),
        other => Ok(other),
    }
}

fn config_paths(path: &Path) -> Option<Vec<PathBuf>> {
    if let Some(extension) = path.extension() {
        return (["conf", "json", "properties"].contains(&extension.to_str()?) && path.is_file())
            .then(|| vec![path.to_path_buf()]);
    }
    // Match the default hocon-rs syntax precedence for extensionless paths.
    let paths: Vec<_> = ["conf", "json", "properties"]
        .into_iter()
        .map(|extension| path.with_extension(extension))
        .filter(|candidate| candidate.is_file())
        .collect();
    (!paths.is_empty()).then_some(paths)
}

fn parse_include_at(source: &str, start: usize) -> Option<(usize, IncludeSpec)> {
    let bytes = source.as_bytes();
    if !bytes[start..].starts_with(b"include") {
        return None;
    }
    let mut cursor = start + 7;
    if bytes
        .get(cursor)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
    {
        return None;
    }
    skip_space(bytes, &mut cursor);
    let required = bytes[cursor..].starts_with(b"required(");
    if required {
        cursor += 9;
        skip_space(bytes, &mut cursor);
    }
    let location = if bytes[cursor..].starts_with(b"classpath(") {
        cursor += 10;
        IncludeLocation::Classpath
    } else if bytes[cursor..].starts_with(b"file(") {
        cursor += 5;
        IncludeLocation::File
    } else {
        IncludeLocation::Either
    };
    skip_space(bytes, &mut cursor);
    if bytes.get(cursor) != Some(&b'"') {
        return None;
    }
    let quoted_start = cursor;
    cursor += 1;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\\' {
            cursor += 2;
        } else if bytes[cursor] == b'"' {
            cursor += 1;
            break;
        } else {
            cursor += 1;
        }
    }
    let path = serde_json::from_str::<String>(source.get(quoted_start..cursor)?).ok()?;
    skip_space(bytes, &mut cursor);
    if !matches!(location, IncludeLocation::Either) {
        if bytes.get(cursor) != Some(&b')') {
            return None;
        }
        cursor += 1;
        skip_space(bytes, &mut cursor);
    }
    if required {
        if bytes.get(cursor) != Some(&b')') {
            return None;
        }
        cursor += 1;
    }
    Some((
        cursor,
        IncludeSpec {
            path,
            required,
            location,
        },
    ))
}

fn skip_space(bytes: &[u8], cursor: &mut usize) {
    while bytes
        .get(*cursor)
        .is_some_and(|byte| *byte == b' ' || *byte == b'\t')
    {
        *cursor += 1;
    }
}
