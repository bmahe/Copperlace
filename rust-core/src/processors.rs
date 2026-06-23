use crate::render::{ProcessorRegistry, processor};

pub(crate) fn builtin_processors() -> ProcessorRegistry {
    let mut processors = ProcessorRegistry::new();
    processors.insert(
        "uppercase".to_string(),
        processor(|value: &str| Ok(value.to_uppercase())),
    );
    processors.insert(
        "lowercase".to_string(),
        processor(|value: &str| Ok(value.to_lowercase())),
    );
    processors.insert(
        "trim".to_string(),
        processor(|value: &str| Ok(value.trim().to_string())),
    );
    processors.insert("capitalize".to_string(), processor(capitalize));
    processors.insert("titlecase".to_string(), processor(titlecase));
    processors.insert("article".to_string(), processor(article));
    processors.insert("past_tense".to_string(), processor(past_tense));
    processors.insert("pluralize".to_string(), processor(pluralize));
    processors.insert("singularize".to_string(), processor(singularize));
    processors.insert("possessive".to_string(), processor(possessive));
    processors.insert(
        "present_participle".to_string(),
        processor(present_participle),
    );
    processors.insert("ordinal".to_string(), processor(ordinal));
    processors.insert("sentence".to_string(), processor(sentence));
    processors.insert("quote".to_string(), processor(quote));
    processors.insert("slug".to_string(), processor(slug));
    processors
}

fn capitalize(value: &str) -> Result<String, String> {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Ok(String::new());
    };

    let mut output = String::new();
    output.extend(first.to_uppercase());
    output.push_str(&chars.as_str().to_lowercase());
    Ok(output)
}

fn titlecase(value: &str) -> Result<String, String> {
    let mut output = String::new();
    let mut start_of_word = true;

    for character in value.chars() {
        if character.is_whitespace() {
            start_of_word = true;
            output.push(character);
        } else if start_of_word {
            output.extend(character.to_uppercase());
            start_of_word = false;
        } else {
            output.extend(character.to_lowercase());
        }
    }

    Ok(output)
}

fn sentence(value: &str) -> Result<String, String> {
    for (index, character) in value.char_indices() {
        if character.is_alphabetic() {
            let mut output = String::new();
            output.push_str(&value[..index]);
            output.extend(character.to_uppercase());
            output.push_str(&value[index + character.len_utf8()..]);
            return Ok(output);
        }
    }

    Ok(value.to_string())
}

fn quote(value: &str) -> Result<String, String> {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            _ => output.push(character),
        }
    }
    output.push('"');
    Ok(output)
}

fn slug(value: &str) -> Result<String, String> {
    let mut output = String::new();
    let mut pending_separator = false;

    for character in value.trim().chars() {
        if character.is_alphanumeric() {
            if pending_separator && !output.is_empty() {
                output.push('-');
            }
            output.extend(character.to_lowercase());
            pending_separator = false;
        } else if matches!(character, '\'' | '’') {
            continue;
        } else if !output.is_empty() {
            pending_separator = true;
        }
    }

    Ok(output)
}

fn article(value: &str) -> Result<String, String> {
    let article = if uses_an(value) { "an" } else { "a" };
    Ok(format!("{article} {value}"))
}

fn uses_an(value: &str) -> bool {
    let token = value.split_whitespace().next().unwrap_or("");
    if token.is_empty() {
        return false;
    }

    let token = token.trim_matches(|character: char| {
        !character.is_alphanumeric() && character != '\'' && character != '-'
    });
    if token.is_empty() {
        return false;
    }

    if starts_with_vowel_sound_number(token) {
        return true;
    }

    let lowercase = token.to_lowercase();
    if starts_with_silent_h(&lowercase) {
        return true;
    }
    if starts_with_hard_vowel_sound(&lowercase) {
        return false;
    }
    if is_initialism(token) {
        return starts_with_vowel_sound_initial(token);
    }

    matches!(lowercase.chars().next(), Some('a' | 'e' | 'i' | 'o' | 'u'))
}

fn starts_with_silent_h(value: &str) -> bool {
    ["heir", "honest", "honor", "honour", "hour"]
        .iter()
        .any(|prefix| value.starts_with(prefix))
}

fn starts_with_hard_vowel_sound(value: &str) -> bool {
    [
        "euro",
        "one",
        "ubiquit",
        "uk",
        "unanim",
        "unic",
        "uniform",
        "union",
        "unique",
        "unit",
        "university",
        "use",
        "user",
        "usual",
        "utensil",
        "utility",
        "utopia",
    ]
    .iter()
    .any(|prefix| value.starts_with(prefix))
}

fn starts_with_vowel_sound_number(value: &str) -> bool {
    value.starts_with('8') || value.starts_with("11") || value.starts_with("18")
}

fn is_initialism(value: &str) -> bool {
    let letters: Vec<char> = value
        .chars()
        .filter(|character| character.is_alphabetic())
        .collect();
    !letters.is_empty() && letters.iter().all(|character| character.is_uppercase())
}

fn starts_with_vowel_sound_initial(value: &str) -> bool {
    matches!(
        value.chars().find(|character| character.is_alphabetic()),
        Some('A' | 'E' | 'F' | 'H' | 'I' | 'L' | 'M' | 'N' | 'O' | 'R' | 'S' | 'X')
    )
}

fn past_tense(value: &str) -> Result<String, String> {
    let (leading, token, trailing) = single_token_parts(value, "verb")?;
    let tense = apply_case_style(token, &past_tense_lowercase(&token.to_lowercase()));

    Ok(format!("{leading}{tense}{trailing}"))
}

fn pluralize(value: &str) -> Result<String, String> {
    let (leading, token, trailing) = single_token_parts(value, "noun")?;
    let plural = apply_case_style(token, &pluralize_lowercase(&token.to_lowercase()));

    Ok(format!("{leading}{plural}{trailing}"))
}

fn pluralize_lowercase(value: &str) -> String {
    if let Some(irregular) = irregular_plural(value) {
        return irregular.to_string();
    }

    if let Some(stem) = value.strip_suffix("fe") {
        return format!("{stem}ves");
    }

    if let Some(stem) = value.strip_suffix('f') {
        return format!("{stem}ves");
    }

    if let Some(stem) = value.strip_suffix('y') {
        if stem
            .chars()
            .last()
            .is_some_and(|character| is_consonant(character))
        {
            return format!("{stem}ies");
        }
    }

    if value.ends_with('s')
        || value.ends_with('x')
        || value.ends_with('z')
        || value.ends_with("ch")
        || value.ends_with("sh")
    {
        return format!("{value}es");
    }

    format!("{value}s")
}

fn irregular_plural(value: &str) -> Option<&'static str> {
    match value {
        "person" => Some("people"),
        "child" => Some("children"),
        "mouse" => Some("mice"),
        "goose" => Some("geese"),
        "man" => Some("men"),
        "woman" => Some("women"),
        "tooth" => Some("teeth"),
        "foot" => Some("feet"),
        "ox" => Some("oxen"),
        _ => None,
    }
}

fn singularize(value: &str) -> Result<String, String> {
    let (leading, token, trailing) = single_token_parts(value, "noun")?;
    let singular = apply_case_style(token, &singularize_lowercase(&token.to_lowercase()));

    Ok(format!("{leading}{singular}{trailing}"))
}

fn singularize_lowercase(value: &str) -> String {
    if let Some(irregular) = irregular_singular(value) {
        return irregular.to_string();
    }

    if let Some(stem) = value.strip_suffix("ies") {
        return format!("{stem}y");
    }

    if let Some(stem) = value.strip_suffix("ves") {
        return format!("{stem}f");
    }

    if value.ends_with("ches")
        || value.ends_with("shes")
        || value.ends_with("xes")
        || value.ends_with("ses")
        || value.ends_with("zes")
    {
        return value
            .strip_suffix("es")
            .expect("suffix was checked")
            .to_string();
    }

    if value.len() > 1 {
        if let Some(stem) = value.strip_suffix('s') {
            return stem.to_string();
        }
    }

    value.to_string()
}

fn irregular_singular(value: &str) -> Option<&'static str> {
    match value {
        "people" => Some("person"),
        "children" => Some("child"),
        "mice" => Some("mouse"),
        "geese" => Some("goose"),
        "men" => Some("man"),
        "women" => Some("woman"),
        "teeth" => Some("tooth"),
        "feet" => Some("foot"),
        "oxen" => Some("ox"),
        _ => None,
    }
}

fn possessive(value: &str) -> Result<String, String> {
    let (leading, token, trailing) = single_token_parts(value, "name")?;
    let suffix = if token.ends_with('s') { "'" } else { "'s" };

    Ok(format!("{leading}{token}{suffix}{trailing}"))
}

fn present_participle(value: &str) -> Result<String, String> {
    let (leading, token, trailing) = single_token_parts(value, "verb")?;
    let participle = apply_case_style(token, &present_participle_lowercase(&token.to_lowercase()));

    Ok(format!("{leading}{participle}{trailing}"))
}

fn present_participle_lowercase(value: &str) -> String {
    if let Some(stem) = value.strip_suffix("ie") {
        return format!("{stem}ying");
    }

    if value.ends_with('e')
        && !value.ends_with("ee")
        && !value.ends_with("ye")
        && !value.ends_with("oe")
    {
        return format!(
            "{}ing",
            value.strip_suffix('e').expect("suffix was checked")
        );
    }

    if should_double_final_consonant(value) {
        let final_character = value.chars().last().expect("value is not empty");
        return format!("{value}{final_character}ing");
    }

    format!("{value}ing")
}

fn ordinal(value: &str) -> Result<String, String> {
    let (leading, token, trailing) = single_token_parts(value, "integer")?;
    let digits = token.strip_prefix('-').unwrap_or(token);
    if digits.is_empty() || !digits.chars().all(|character| character.is_ascii_digit()) {
        return Err("input must contain one integer".to_string());
    }

    Ok(format!(
        "{leading}{token}{}{trailing}",
        ordinal_suffix(digits)
    ))
}

fn ordinal_suffix(digits: &str) -> &'static str {
    let last_two = if digits.len() >= 2 {
        &digits[digits.len() - 2..]
    } else {
        digits
    };
    if matches!(last_two, "11" | "12" | "13") {
        return "th";
    }

    match digits.chars().last() {
        Some('1') => "st",
        Some('2') => "nd",
        Some('3') => "rd",
        _ => "th",
    }
}

fn past_tense_lowercase(value: &str) -> String {
    if let Some(irregular) = irregular_past_tense(value) {
        return irregular.to_string();
    }

    if value.ends_with('e') {
        return format!("{value}d");
    }

    if let Some(stem) = value.strip_suffix('y') {
        if stem
            .chars()
            .last()
            .is_some_and(|character| is_consonant(character))
        {
            return format!("{stem}ied");
        }
    }

    if should_double_final_consonant(value) {
        let final_character = value.chars().last().expect("value is not empty");
        return format!("{value}{final_character}ed");
    }

    format!("{value}ed")
}

fn irregular_past_tense(value: &str) -> Option<&'static str> {
    match value {
        "am" | "be" | "is" => Some("was"),
        "are" => Some("were"),
        "go" => Some("went"),
        "do" => Some("did"),
        "have" => Some("had"),
        "make" => Some("made"),
        "take" => Some("took"),
        "come" => Some("came"),
        "run" => Some("ran"),
        "eat" => Some("ate"),
        "see" => Some("saw"),
        "say" => Some("said"),
        "get" => Some("got"),
        "give" => Some("gave"),
        "find" => Some("found"),
        "think" => Some("thought"),
        "buy" => Some("bought"),
        "catch" => Some("caught"),
        "teach" => Some("taught"),
        "bring" => Some("brought"),
        "write" => Some("wrote"),
        "read" => Some("read"),
        _ => None,
    }
}

fn single_token_parts<'a>(
    value: &'a str,
    part_of_speech: &str,
) -> Result<(&'a str, &'a str, &'a str), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("input must contain one {part_of_speech}"));
    }
    if trimmed.split_whitespace().count() != 1 {
        return Err(format!(
            "input must contain exactly one {part_of_speech} token"
        ));
    }

    let leading_len = value.len() - value.trim_start().len();
    let trailing_len = value.len() - value.trim_end().len();
    let leading = &value[..leading_len];
    let trailing = &value[value.len() - trailing_len..];

    Ok((leading, trimmed, trailing))
}

fn should_double_final_consonant(value: &str) -> bool {
    let characters: Vec<char> = value.chars().collect();
    if characters.len() < 3 {
        return false;
    }

    let last = characters[characters.len() - 1];
    let middle = characters[characters.len() - 2];
    let first = characters[characters.len() - 3];

    is_consonant(first)
        && is_vowel(middle)
        && is_consonant(last)
        && !matches!(last, 'w' | 'x' | 'y')
}

fn is_vowel(character: char) -> bool {
    matches!(character, 'a' | 'e' | 'i' | 'o' | 'u')
}

fn is_consonant(character: char) -> bool {
    character.is_ascii_alphabetic() && !is_vowel(character.to_ascii_lowercase())
}

fn apply_case_style(original: &str, value: &str) -> String {
    if original
        .chars()
        .all(|character| !character.is_alphabetic() || character.is_uppercase())
    {
        return value.to_uppercase();
    }

    let mut characters = original
        .chars()
        .filter(|character| character.is_alphabetic());
    if let Some(first) = characters.next() {
        if first.is_uppercase() && characters.all(|character| character.is_lowercase()) {
            return capitalize(value).expect("capitalize is infallible");
        }
    }

    value.to_string()
}
