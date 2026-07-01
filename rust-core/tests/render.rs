use copperlace::render::{ProcessorRegistry, processor};
use copperlace::{
    Copperlace, RenderContext, RenderError, RenderOptions, RuleSet,
    render_config_rule_with_context, render_file_with_context, render_str_with_context,
};

fn ruleset(config: &str) -> RuleSet {
    let value = hocon_rs::Config::parse_str::<hocon_rs::Value>(config, None).unwrap();
    RuleSet::from_config(value).unwrap()
}

fn ruleset_result(config: &str) -> Result<RuleSet, RenderError> {
    let value = hocon_rs::Config::parse_str::<hocon_rs::Value>(config, None).unwrap();
    RuleSet::from_config(value)
}

fn assert_ruleset_error(config: &str, expected: RenderError) {
    match ruleset_result(config) {
        Ok(_) => panic!("expected ruleset construction to fail"),
        Err(error) => assert_eq!(error, expected),
    }
}

fn slash_pair(value: &str) -> (&str, &str) {
    value.split_once('/').unwrap()
}

#[test]
fn renders_from_multiple_named_rules() {
    let rules = ruleset(
        r#"
        name = ["Mia"]
        animal = ["owl"]
        story = ["{hero} and {heroPet}"]
        origin = "{% hero:name %}{% heroPet:animal %}{story}"
        context = {
            hero = "{name}"
            heroPet = "{animal}"
        }
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Mia and owl");
    assert_eq!(rules.render_rule("story").unwrap(), "Mia and owl");
    assert_eq!(rules.render_rule("name").unwrap(), "Mia");
    assert_eq!(rules.render_rule("animal").unwrap(), "owl");
}

#[test]
fn copperlace_renders_repeatedly_from_string() {
    let copperlace = Copperlace::from_str(
        r#"
        name = ["Mia"]
        pet = ["owl"]
        origin = "{name}"
        companion = "{name} and {pet}"
        "#,
    )
    .unwrap();

    assert_eq!(copperlace.render("origin").unwrap(), "Mia");
    assert_eq!(copperlace.render("companion").unwrap(), "Mia and owl");
    assert_eq!(copperlace.render("origin").unwrap(), "Mia");
}

#[test]
fn copperlace_renders_repeatedly_from_file() {
    let config_path =
        std::env::temp_dir().join(format!("copperlace-reusable-{}.conf", std::process::id()));
    std::fs::write(
        &config_path,
        r#"
        name = ["Mia"]
        origin = "{name}"
        "#,
    )
    .unwrap();

    let copperlace = Copperlace::from_file(&config_path).unwrap();

    assert_eq!(copperlace.render("origin").unwrap(), "Mia");
    assert_eq!(copperlace.render("origin").unwrap(), "Mia");

    let _ = std::fs::remove_file(config_path);
}

#[test]
fn initial_context_resolves_template_reference() {
    let rules = ruleset(
        r#"
        origin = "Hello {name}"
        "#,
    );
    let mut context = RenderContext::new();
    context.insert("name".to_string(), "Mia".to_string());

    assert_eq!(
        rules.render_rule_with_context("origin", context).unwrap(),
        "Hello Mia"
    );
}

#[test]
fn initial_context_overrides_context_default() {
    let rules = ruleset(
        r#"
        fallback = ["Darcy"]
        origin = "{hero}"
        context = {
            hero = "{fallback}"
        }
        "#,
    );
    let mut context = RenderContext::new();
    context.insert("hero".to_string(), "Mia".to_string());

    assert_eq!(
        rules.render_rule_with_context("origin", context).unwrap(),
        "Mia"
    );
}

#[test]
fn initial_context_overrides_named_rule() {
    let rules = ruleset(
        r#"
        hero = ["Darcy"]
        origin = "{hero}"
        "#,
    );
    let mut context = RenderContext::new();
    context.insert("hero".to_string(), "Mia".to_string());

    assert_eq!(
        rules.render_rule_with_context("origin", context).unwrap(),
        "Mia"
    );
}

#[test]
fn bind_if_missing_preserves_initial_context() {
    let rules = ruleset(
        r#"
        fallback = ["Darcy"]
        origin = "{% hero:fallback %}{hero}"
        "#,
    );
    let mut context = RenderContext::new();
    context.insert("hero".to_string(), "Mia".to_string());

    assert_eq!(
        rules.render_rule_with_context("origin", context).unwrap(),
        "Mia"
    );
}

#[test]
fn overwrite_binding_replaces_initial_context() {
    let rules = ruleset(
        r#"
        fallback = ["Darcy"]
        origin = "{% hero:=fallback %}{hero}"
        "#,
    );
    let mut context = RenderContext::new();
    context.insert("hero".to_string(), "Mia".to_string());

    assert_eq!(
        rules.render_rule_with_context("origin", context).unwrap(),
        "Darcy"
    );
}

#[test]
fn dotted_initial_context_key_resolves_template_reference() {
    let rules = ruleset(
        r#"
        origin = "{hero.name}"
        "#,
    );
    let mut context = RenderContext::new();
    context.insert("hero.name".to_string(), "Mia".to_string());

    assert_eq!(
        rules.render_rule_with_context("origin", context).unwrap(),
        "Mia"
    );
}

#[test]
fn initial_context_does_not_persist_between_renders() {
    let rules = ruleset(
        r#"
        hero = ["Darcy"]
        origin = "{hero}"
        "#,
    );
    let mut context = RenderContext::new();
    context.insert("hero".to_string(), "Mia".to_string());

    assert_eq!(
        rules.render_rule_with_context("origin", context).unwrap(),
        "Mia"
    );
    assert_eq!(rules.render_rule("origin").unwrap(), "Darcy");
}

#[test]
fn copperlace_renders_with_initial_context() {
    let copperlace = Copperlace::from_str(
        r#"
        origin = "Hello {name}"
        "#,
    )
    .unwrap();
    let mut context = RenderContext::new();
    context.insert("name".to_string(), "Mia".to_string());

    assert_eq!(
        copperlace.render_with_context("origin", context).unwrap(),
        "Hello Mia"
    );
}

#[test]
fn render_str_renders_with_initial_context() {
    let mut context = RenderContext::new();
    context.insert("name".to_string(), "Mia".to_string());

    assert_eq!(
        render_str_with_context(r#"origin = "Hello {name}""#, "origin", context).unwrap(),
        "Hello Mia"
    );
}

#[test]
fn render_file_renders_with_initial_context() {
    let config_path =
        std::env::temp_dir().join(format!("copperlace-context-{}.conf", std::process::id()));
    std::fs::write(
        &config_path,
        r#"
        origin = "Hello {name}"
        "#,
    )
    .unwrap();
    let mut context = RenderContext::new();
    context.insert("name".to_string(), "Mia".to_string());

    assert_eq!(
        render_file_with_context(&config_path, "origin", context).unwrap(),
        "Hello Mia"
    );

    let _ = std::fs::remove_file(config_path);
}

#[test]
fn render_config_rule_renders_with_initial_context() {
    let value = hocon_rs::Config::parse_str::<hocon_rs::Value>(
        r#"
        origin = "Hello {name}"
        "#,
        None,
    )
    .unwrap();
    let mut context = RenderContext::new();
    context.insert("name".to_string(), "Mia".to_string());

    assert_eq!(
        render_config_rule_with_context(value, "origin", context).unwrap(),
        "Hello Mia"
    );
}

#[test]
fn binding_reuses_the_same_generated_value() {
    let rules = ruleset(
        r#"
        name = ["Mia"]
        origin = "{% hero:name %}{hero}/{hero}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Mia/Mia");
}

#[test]
fn binding_does_not_overwrite_existing_value() {
    let rules = ruleset(
        r#"
        first = ["Mia"]
        second = ["Darcy"]
        origin = "{% hero:first %}{% hero:second %}{hero}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Mia");
}

#[test]
fn binding_does_not_overwrite_context_default_value() {
    let rules = ruleset(
        r#"
        name = ["Mia"]
        other = ["Darcy"]
        origin = "{hero}{% hero:other %}/{hero}"
        context = {
            hero = "{name}"
        }
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Mia/Mia");
}

#[test]
fn overwrite_binding_replaces_existing_value() {
    let rules = ruleset(
        r#"
        first = ["Mia"]
        second = ["Darcy"]
        origin = "{% hero:first %}{% hero:=second %}{hero}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Darcy");
}

#[test]
fn overwrite_binding_replaces_context_default_value() {
    let rules = ruleset(
        r#"
        name = ["Mia"]
        other = ["Darcy"]
        origin = "{hero}{% hero:=other %}/{hero}"
        context = {
            hero = "{name}"
        }
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Mia/Darcy");
}

#[test]
fn calls_another_rule_without_eager_expansion() {
    let rules = ruleset(
        r#"
        adjective = ["bright"]
        story = ["A {adjective} path"]
        origin = "{story}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "A bright path");
}

#[test]
fn unknown_rule_returns_error() {
    let rules = ruleset(
        r#"
        origin = "{missing}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin"),
        Err(RenderError::UnknownRule("missing".to_string()))
    );
}

#[test]
fn circular_rule_reference_returns_error() {
    let rules = ruleset(
        r#"
        a = "{b}"
        b = "{a}"
        "#,
    );

    assert_eq!(
        rules.render_rule("a"),
        Err(RenderError::CircularRuleReference(vec![
            "a".to_string(),
            "b".to_string(),
            "a".to_string(),
        ]))
    );
}

#[test]
fn limited_self_recursion_returns_empty_at_cutoff() {
    let rules = ruleset(
        r#"
        origin = "x{origin}"
        "#,
    );

    assert_eq!(
        rules
            .render_rule_with_options(
                "origin",
                RenderOptions {
                    max_recursion_depth: 1,
                },
            )
            .unwrap(),
        "xx"
    );
}

#[test]
fn limited_mutual_recursion_returns_empty_at_cutoff() {
    let rules = ruleset(
        r#"
        a = "a{b}"
        b = "b{a}"
        "#,
    );

    assert_eq!(
        rules
            .render_rule_with_options(
                "a",
                RenderOptions {
                    max_recursion_depth: 1,
                },
            )
            .unwrap(),
        "abab"
    );
}

#[test]
fn initial_context_shadows_recursive_rule_without_consuming_depth() {
    let mut context = RenderContext::new();
    context.insert("origin".to_string(), "bound".to_string());
    let rules = ruleset(
        r#"
        origin = "x{origin}"
        "#,
    );

    assert_eq!(
        rules
            .render_rule_with_context_and_options(
                "origin",
                context,
                RenderOptions {
                    max_recursion_depth: 1,
                },
            )
            .unwrap(),
        "xbound"
    );
}

#[test]
fn limited_context_default_recursion_returns_empty_at_cutoff() {
    let rules = ruleset(
        r#"
        origin = "{hero}"
        context {
            hero = "h{hero}"
        }
        "#,
    );

    assert_eq!(
        rules
            .render_rule_with_options(
                "origin",
                RenderOptions {
                    max_recursion_depth: 1,
                },
            )
            .unwrap(),
        "hh"
    );
}

#[test]
fn dotted_path_key_rule_renders_leaf_value() {
    let rules = ruleset(
        r#"
        nordic.vowel = ["ö"]
        origin = "{nordic.vowel}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "ö");
    assert_eq!(rules.render_rule("nordic.vowel").unwrap(), "ö");
}

#[test]
fn nested_object_rule_renders_leaf_value() {
    let rules = ruleset(
        r#"
        nordic {
            vowel = ["ö"]
        }
        origin = "{nordic.vowel}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "ö");
    assert_eq!(rules.render_rule("nordic.vowel").unwrap(), "ö");
}

#[test]
fn rendering_object_parent_returns_error() {
    let rules = ruleset(
        r#"
        nordic.vowel = ["ö"]
        "#,
    );

    assert_eq!(
        rules.render_rule("nordic"),
        Err(RenderError::UnsupportedValue("object".to_string()))
    );
}

#[test]
fn dotted_context_default_renders_and_caches_value() {
    let rules = ruleset(
        r#"
        first = ["Mia"]
        second = ["Darcy"]
        origin = "{hero.name}{% hero.name:=second %}/{hero.name}"
        context.hero.name = "{first}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Mia/Darcy");
}

#[test]
fn rendering_context_object_parent_returns_error() {
    let rules = ruleset(
        r#"
        context.hero.name = "Mia"
        origin = "{hero}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin"),
        Err(RenderError::UnsupportedValue("object".to_string()))
    );
}

#[test]
fn dotted_rules_report_circular_reference_path() {
    let rules = ruleset(
        r#"
        nordic.first = "{nordic.family}"
        nordic.family = "{nordic.first}"
        "#,
    );

    assert_eq!(
        rules.render_rule("nordic.first"),
        Err(RenderError::CircularRuleReference(vec![
            "nordic.first".to_string(),
            "nordic.family".to_string(),
            "nordic.first".to_string(),
        ]))
    );
}

#[test]
fn empty_choice_returns_error() {
    let rules = ruleset(
        r#"
        origin = []
        "#,
    );

    assert_eq!(rules.render_rule("origin"), Err(RenderError::EmptyChoice));
}

#[test]
fn rendering_object_rule_returns_error() {
    let rules = ruleset(
        r#"
        origin = { value = "nested" }
        "#,
    );

    assert_eq!(
        rules.render_rule("origin"),
        Err(RenderError::UnsupportedValue("object".to_string()))
    );
}

#[test]
fn multi_choice_rule_renders_one_allowed_value() {
    let rules = ruleset(
        r#"
        origin = [red, blue]
        "#,
    );

    let output = rules.render_rule("origin").unwrap();

    assert!(["red", "blue"].contains(&output.as_str()));
}

#[test]
fn weighted_choice_rule_renders_only_positive_weight_value() {
    let rules = ruleset(
        r#"
        origin = [
            { value = red, weight = 0 },
            { value = blue, weight = 2.5 }
        ]
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "blue");
}

#[test]
fn weighted_choice_rule_treats_plain_entries_as_weight_one() {
    let rules = ruleset(
        r#"
        origin = [
            { value = red, weight = 0 },
            blue
        ]
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "blue");
}

#[test]
fn weighted_choice_value_uses_normal_rendering_rules() {
    let rules = ruleset(
        r#"
        name = ["Mia"]
        origin = [
            { value = "Hello {name}", weight = 1.5 }
        ]
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Hello Mia");
}

#[test]
fn weighted_choice_value_can_be_nested_array() {
    let rules = ruleset(
        r#"
        origin = [
            { value = [red], weight = 1 }
        ]
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "red");
}

#[test]
fn weighted_choice_rejects_missing_value() {
    assert!(matches!(
        ruleset_result(
            r#"
            origin = [
                { weight = 1 }
            ]
            "#
        ),
        Err(RenderError::InvalidWeightedChoice(_))
    ));
}

#[test]
fn weighted_choice_rejects_missing_weight() {
    assert!(matches!(
        ruleset_result(
            r#"
            origin = [
                { value = red }
            ]
            "#
        ),
        Err(RenderError::InvalidWeightedChoice(_))
    ));
}

#[test]
fn weighted_choice_rejects_string_weight() {
    assert!(matches!(
        ruleset_result(
            r#"
            origin = [
                { value = red, weight = "often" }
            ]
            "#
        ),
        Err(RenderError::InvalidWeightedChoice(_))
    ));
}

#[test]
fn weighted_choice_rejects_negative_weight() {
    assert!(matches!(
        ruleset_result(
            r#"
            origin = [
                { value = red, weight = -1 }
            ]
            "#
        ),
        Err(RenderError::InvalidWeightedChoice(_))
    ));
}

#[test]
fn weighted_choice_rejects_all_zero_weights() {
    assert!(matches!(
        ruleset_result(
            r#"
            origin = [
                { value = red, weight = 0 },
                { value = blue, weight = 0 }
            ]
            "#
        ),
        Err(RenderError::InvalidWeightedChoice(_))
    ));
}

#[test]
fn weighted_choice_rejects_malformed_object_in_weighted_array() {
    assert!(matches!(
        ruleset_result(
            r#"
            origin = [
                { value = red, weight = 1 },
                { nested = blue }
            ]
            "#
        ),
        Err(RenderError::InvalidWeightedChoice(_))
    ));
}

#[test]
fn unique_choice_draws_distinct_entries() {
    let rules = ruleset(
        r#"
        hero = [Mia, Lina]
        origin = "{hero!}/{hero!}"
        "#,
    );

    let output = rules.render_rule("origin").unwrap();
    let (first, second) = slash_pair(&output);

    assert_ne!(first, second);
    assert!(["Mia", "Lina"].contains(&first));
    assert!(["Mia", "Lina"].contains(&second));
}

#[test]
fn normal_repeated_choice_call_is_backward_compatible() {
    let rules = ruleset(
        r#"
        hero = [Mia]
        origin = "{hero}/{hero}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Mia/Mia");
}

#[test]
fn unique_choice_state_resets_between_render_calls() {
    let rules = ruleset(
        r#"
        hero = [Mia]
        origin = "{hero!}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Mia");
    assert_eq!(rules.render_rule("origin").unwrap(), "Mia");
}

#[test]
fn unique_choice_returns_exhausted_when_entries_are_used() {
    let rules = ruleset(
        r#"
        hero = [Mia, Lina]
        origin = "{hero!}, {hero!}, {hero!}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin"),
        Err(RenderError::ExhaustedUniqueChoice("hero".to_string()))
    );
}

#[test]
fn unique_choice_returns_unsupported_for_non_choice_rule() {
    let rules = ruleset(
        r#"
        title = "The {hero}"
        hero = [Mia]
        origin = "{title!}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin"),
        Err(RenderError::UnsupportedUniqueChoice("title".to_string()))
    );
}

#[test]
fn unique_empty_choice_preserves_empty_choice_error() {
    let rules = ruleset(
        r#"
        empty = []
        origin = "{empty!}"
        "#,
    );

    assert_eq!(rules.render_rule("origin"), Err(RenderError::EmptyChoice));
}

#[test]
fn binding_value_wins_before_unique_choice() {
    let rules = ruleset(
        r#"
        name = [Mia]
        origin = "{% hero:name %}{hero!}/{hero!}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Mia/Mia");
}

#[test]
fn initial_context_value_wins_before_unique_choice() {
    let rules = ruleset(
        r#"
        hero = [Mia]
        origin = "{hero!}/{hero!}"
        "#,
    );
    let mut context = RenderContext::new();
    context.insert("hero".to_string(), "Lina".to_string());

    assert_eq!(
        rules.render_rule_with_context("origin", context).unwrap(),
        "Lina/Lina"
    );
}

#[test]
fn context_default_is_rendered_and_cached_before_unique_named_rule() {
    let rules = ruleset(
        r#"
        name = [Mia, Lina]
        origin = "{hero!}/{hero!}"
        context = {
            hero = "{name}"
        }
        "#,
    );

    let output = rules.render_rule("origin").unwrap();
    let (first, second) = slash_pair(&output);
    assert_eq!(first, second);
    assert!(["Mia", "Lina"].contains(&first));
}

#[test]
fn unique_marker_inside_context_default_applies_to_nested_choice() {
    let rules = ruleset(
        r#"
        name = [Mia, Lina]
        origin = "{hero}/{rival}"
        context = {
            hero = "{name!}"
            rival = "{name!}"
        }
        "#,
    );

    let output = rules.render_rule("origin").unwrap();
    let (first, second) = slash_pair(&output);
    assert_ne!(first, second);
}

#[test]
fn weighted_unique_choice_uses_only_positive_unused_entries() {
    let rules = ruleset(
        r#"
        hero = [
            { value = Mia, weight = 1 },
            { value = Lina, weight = 0 }
        ]
        origin = "{hero!}/{hero!}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin"),
        Err(RenderError::ExhaustedUniqueChoice("hero".to_string()))
    );
}

#[test]
fn unique_choice_supports_processors_and_bindings() {
    let rules = ruleset(
        r#"
        hero = [mia]
        origin = "{hero! | uppercase}/{% first:hero! %}{first}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin"),
        Err(RenderError::ExhaustedUniqueChoice("hero".to_string()))
    );
}

#[test]
fn unique_choice_binding_source_renders_and_reuses_bound_value() {
    let rules = ruleset(
        r#"
        hero = [Mia]
        origin = "{% first:hero! %}{first}/{first}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Mia/Mia");
}

#[test]
fn unique_choice_processor_pipeline_applies_after_selection() {
    let rules = ruleset(
        r#"
        hero = [mia]
        origin = "{hero! | uppercase}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "MIA");
}

#[test]
fn malformed_unique_expressions_fail_during_ruleset_construction() {
    assert_ruleset_error(
        r#"origin = "{!}""#,
        RenderError::InvalidExpression("!".to_string()),
    );
    assert_ruleset_error(
        r#"origin = "{% alias:! %}""#,
        RenderError::InvalidExpression("alias:!".to_string()),
    );
}

#[test]
fn unique_expression_preserves_processor_validation() {
    assert_ruleset_error(
        r#"origin = "{hero! | missing_processor}""#,
        RenderError::UnknownProcessor("missing_processor".to_string()),
    );
}

#[test]
fn scalar_rule_renders_as_string() {
    let rules = ruleset(
        r#"
        origin = 3
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "3");
}

#[test]
fn template_expression_whitespace_is_trimmed() {
    let rules = ruleset(
        r#"
        name = ["Mia"]
        origin = "Hello { name }"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Hello Mia");
}

#[test]
fn escaped_template_braces_render_as_literals() {
    let rules = ruleset(
        r#"
        origin = """\{\}"""
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "{}");
}

#[test]
fn escaped_template_braces_work_next_to_expressions() {
    let rules = ruleset(
        r#"
        name = ["Mia"]
        origin = """\{{name}\}"""
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "{Mia}");
}

#[test]
fn normal_config_strings_can_escape_template_braces() {
    let rules = ruleset(
        r#"
        origin = "\\{name\\}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "{name}");
}

#[test]
fn escaped_template_statement_delimiters_render_as_literals() {
    let rules = ruleset(
        r#"
        origin = """\{% name %\}"""
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "{% name %}");
}

#[test]
fn template_backslashes_remain_literal_when_not_escaping_braces() {
    let rules = ruleset(
        r#"
        origin = """path\name"""
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), r"path\name");
}

#[test]
fn unmatched_opening_template_brace_is_invalid() {
    assert!(matches!(
        ruleset_result(
            r#"
            origin = """{"""
            "#
        ),
        Err(RenderError::InvalidExpression(_))
    ));
}

#[test]
fn unmatched_closing_template_brace_is_invalid() {
    assert!(matches!(
        ruleset_result(
            r#"
            origin = """}"""
            "#
        ),
        Err(RenderError::InvalidExpression(_))
    ));
}

#[test]
fn unmatched_opening_template_statement_delimiter_is_invalid() {
    assert!(matches!(
        ruleset_result(
            r#"
            origin = """{% name"""
            "#
        ),
        Err(RenderError::InvalidExpression(_))
    ));
}

#[test]
fn unmatched_closing_template_statement_delimiter_is_invalid() {
    assert!(matches!(
        ruleset_result(
            r#"
            origin = """name %}"""
            "#
        ),
        Err(RenderError::InvalidExpression(_))
    ));
}

#[test]
fn item_json_example_renders_valid_json_with_escaped_braces() {
    let copperlace = Copperlace::from_str(include_str!("../../examples/item_json.conf")).unwrap();
    let rendered = copperlace.render("origin").unwrap();
    let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();

    assert!(
        json.get("properties")
            .is_some_and(|value| value.is_object())
    );
}

#[test]
fn overwrite_binding_whitespace_is_trimmed() {
    let rules = ruleset(
        r#"
        first = ["Mia"]
        second = ["Darcy"]
        origin = "{% hero:first %}{% hero:=second %}{hero}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Darcy");
}

#[test]
fn expression_binding_syntax_is_invalid() {
    assert!(matches!(
        ruleset_result(
            r#"
            name = ["Mia"]
            origin = "{hero:name}"
            "#
        ),
        Err(RenderError::InvalidExpression(expression)) if expression == "hero:name"
    ));
}

#[test]
fn expression_overwrite_binding_syntax_is_invalid() {
    assert!(matches!(
        ruleset_result(
            r#"
            name = ["Mia"]
            origin = "{hero:=name}"
            "#
        ),
        Err(RenderError::InvalidExpression(expression)) if expression == "hero:=name"
    ));
}

#[test]
fn statement_without_side_effect_is_invalid() {
    assert!(matches!(
        ruleset_result(
            r#"
            name = ["Mia"]
            origin = "{% name %}"
            "#
        ),
        Err(RenderError::InvalidExpression(expression)) if expression == "name"
    ));
}

#[test]
fn statement_without_source_is_invalid() {
    assert!(matches!(
        ruleset_result(
            r#"
            origin = "{% hero: %}"
            "#
        ),
        Err(RenderError::InvalidExpression(expression)) if expression == "hero:"
    ));
}

#[test]
fn statement_without_target_is_invalid() {
    assert!(matches!(
        ruleset_result(
            r#"
            origin = "{% :name %}"
            "#
        ),
        Err(RenderError::InvalidExpression(expression)) if expression == ":name"
    ));
}

#[test]
fn statement_with_empty_processor_is_invalid() {
    assert!(matches!(
        ruleset_result(
            r#"
            name = ["Mia"]
            origin = "{% hero:name | %}"
            "#
        ),
        Err(RenderError::InvalidExpression(expression)) if expression == "hero:name |"
    ));
}

#[test]
fn processor_pipeline_transforms_rule_output() {
    let rules = ruleset(
        r#"
        name = ["Mia"]
        origin = "Hello {name | uppercase}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Hello MIA");
}

#[test]
fn processor_pipeline_runs_left_to_right() {
    let rules = ruleset(
        r#"
        name = ["  mIA  "]
        origin = "{name | trim | lowercase | capitalize}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Mia");
}

#[test]
fn article_processor_adds_a_or_an_for_common_words() {
    let rules = ruleset(
        r#"
        apple = ["apple"]
        book = ["book"]
        hour = ["hour"]
        user = ["user"]
        origin = "{apple | article}/{book | article}/{hour | article}/{user | article}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin").unwrap(),
        "an apple/a book/an hour/a user"
    );
}

#[test]
fn article_processor_handles_initialisms_and_numbers() {
    let rules = ruleset(
        r#"
        mri = ["MRI"]
        url = ["URL"]
        eight_ball = ["8-ball"]
        eleven_year_old = ["11-year-old"]
        origin = "{mri | article}/{url | article}/{eight_ball | article}/{eleven_year_old | article}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin").unwrap(),
        "an MRI/a URL/an 8-ball/an 11-year-old"
    );
}

#[test]
fn article_processor_preserves_input_spacing() {
    let rules = ruleset(
        r#"
        padded = ["  apple  "]
        origin = "{padded | article}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "an   apple  ");
}

#[test]
fn article_processor_composes_with_trim() {
    let rules = ruleset(
        r#"
        padded = ["  apple  "]
        origin = "{padded | trim | article}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "an apple");
}

#[test]
fn past_tense_processor_handles_regular_verbs() {
    let rules = ruleset(
        r#"
        walk = ["walk"]
        bake = ["bake"]
        try = ["try"]
        stop = ["stop"]
        origin = "{walk | past_tense}/{bake | past_tense}/{try | past_tense}/{stop | past_tense}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin").unwrap(),
        "walked/baked/tried/stopped"
    );
}

#[test]
fn past_tense_processor_handles_common_irregular_verbs() {
    let rules = ruleset(
        r#"
        go = ["go"]
        run = ["run"]
        be = ["be"]
        are = ["are"]
        read = ["read"]
        origin = "{go | past_tense}/{run | past_tense}/{be | past_tense}/{are | past_tense}/{read | past_tense}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin").unwrap(),
        "went/ran/was/were/read"
    );
}

#[test]
fn past_tense_processor_preserves_capitalization_style() {
    let rules = ruleset(
        r#"
        title = ["Run"]
        upper = ["RUN"]
        origin = "{title | past_tense}/{upper | past_tense}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Ran/RAN");
}

#[test]
fn past_tense_processor_preserves_surrounding_whitespace() {
    let rules = ruleset(
        r#"
        padded = ["  walk  "]
        origin = "{padded | past_tense}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "  walked  ");
}

#[test]
fn past_tense_processor_rejects_blank_input() {
    let rules = ruleset(
        r#"
        blank = ["  "]
        origin = "{blank | past_tense}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin"),
        Err(RenderError::ProcessorError {
            processor: "past_tense".to_string(),
            message: "input must contain one verb".to_string(),
        })
    );
}

#[test]
fn past_tense_processor_rejects_multiple_words() {
    let rules = ruleset(
        r#"
        phrase = ["walk home"]
        origin = "{phrase | past_tense}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin"),
        Err(RenderError::ProcessorError {
            processor: "past_tense".to_string(),
            message: "input must contain exactly one verb token".to_string(),
        })
    );
}

#[test]
fn pluralize_processor_handles_regular_nouns() {
    let rules = ruleset(
        r#"
        cat = ["cat"]
        box = ["box"]
        city = ["city"]
        leaf = ["leaf"]
        knife = ["knife"]
        origin = "{cat | pluralize}/{box | pluralize}/{city | pluralize}/{leaf | pluralize}/{knife | pluralize}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin").unwrap(),
        "cats/boxes/cities/leaves/knives"
    );
}

#[test]
fn pluralize_processor_handles_common_irregular_nouns() {
    let rules = ruleset(
        r#"
        person = ["person"]
        child = ["child"]
        mouse = ["mouse"]
        ox = ["ox"]
        origin = "{person | pluralize}/{child | pluralize}/{mouse | pluralize}/{ox | pluralize}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin").unwrap(),
        "people/children/mice/oxen"
    );
}

#[test]
fn pluralize_processor_preserves_capitalization_and_whitespace() {
    let rules = ruleset(
        r#"
        title = ["Person"]
        upper = ["DOG"]
        padded = ["  cat  "]
        origin = "{title | pluralize}/{upper | pluralize}/{padded | pluralize}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "People/DOGS/  cats  ");
}

#[test]
fn pluralize_processor_rejects_blank_input() {
    let rules = ruleset(
        r#"
        blank = ["  "]
        origin = "{blank | pluralize}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin"),
        Err(RenderError::ProcessorError {
            processor: "pluralize".to_string(),
            message: "input must contain one noun".to_string(),
        })
    );
}

#[test]
fn pluralize_processor_rejects_multiple_words() {
    let rules = ruleset(
        r#"
        phrase = ["red cat"]
        origin = "{phrase | pluralize}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin"),
        Err(RenderError::ProcessorError {
            processor: "pluralize".to_string(),
            message: "input must contain exactly one noun token".to_string(),
        })
    );
}

#[test]
fn singularize_processor_handles_regular_nouns() {
    let rules = ruleset(
        r#"
        cats = ["cats"]
        boxes = ["boxes"]
        cities = ["cities"]
        leaves = ["leaves"]
        wishes = ["wishes"]
        origin = "{cats | singularize}/{boxes | singularize}/{cities | singularize}/{leaves | singularize}/{wishes | singularize}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin").unwrap(),
        "cat/box/city/leaf/wish"
    );
}

#[test]
fn singularize_processor_handles_common_irregular_nouns() {
    let rules = ruleset(
        r#"
        people = ["people"]
        children = ["children"]
        mice = ["mice"]
        oxen = ["oxen"]
        origin = "{people | singularize}/{children | singularize}/{mice | singularize}/{oxen | singularize}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin").unwrap(),
        "person/child/mouse/ox"
    );
}

#[test]
fn singularize_processor_preserves_capitalization_and_whitespace() {
    let rules = ruleset(
        r#"
        title = ["People"]
        upper = ["DOGS"]
        padded = ["  cats  "]
        origin = "{title | singularize}/{upper | singularize}/{padded | singularize}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Person/DOG/  cat  ");
}

#[test]
fn singularize_processor_rejects_blank_input() {
    let rules = ruleset(
        r#"
        blank = ["  "]
        origin = "{blank | singularize}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin"),
        Err(RenderError::ProcessorError {
            processor: "singularize".to_string(),
            message: "input must contain one noun".to_string(),
        })
    );
}

#[test]
fn singularize_processor_rejects_multiple_words() {
    let rules = ruleset(
        r#"
        phrase = ["red cats"]
        origin = "{phrase | singularize}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin"),
        Err(RenderError::ProcessorError {
            processor: "singularize".to_string(),
            message: "input must contain exactly one noun token".to_string(),
        })
    );
}

#[test]
fn possessive_processor_adds_apostrophe_s() {
    let rules = ruleset(
        r#"
        name = ["Mia"]
        origin = "{name | possessive}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Mia's");
}

#[test]
fn possessive_processor_adds_apostrophe_for_s_ending() {
    let rules = ruleset(
        r#"
        name = ["James"]
        origin = "{name | possessive}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "James'");
}

#[test]
fn possessive_processor_preserves_surrounding_whitespace() {
    let rules = ruleset(
        r#"
        name = ["  Mia  "]
        origin = "{name | possessive}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "  Mia's  ");
}

#[test]
fn possessive_processor_rejects_blank_input() {
    let rules = ruleset(
        r#"
        blank = ["  "]
        origin = "{blank | possessive}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin"),
        Err(RenderError::ProcessorError {
            processor: "possessive".to_string(),
            message: "input must contain one name".to_string(),
        })
    );
}

#[test]
fn possessive_processor_rejects_multiple_words() {
    let rules = ruleset(
        r#"
        phrase = ["Mia Rose"]
        origin = "{phrase | possessive}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin"),
        Err(RenderError::ProcessorError {
            processor: "possessive".to_string(),
            message: "input must contain exactly one name token".to_string(),
        })
    );
}

#[test]
fn present_participle_processor_handles_regular_verbs() {
    let rules = ruleset(
        r#"
        walk = ["walk"]
        make = ["make"]
        run = ["run"]
        lie = ["lie"]
        see = ["see"]
        origin = "{walk | present_participle}/{make | present_participle}/{run | present_participle}/{lie | present_participle}/{see | present_participle}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin").unwrap(),
        "walking/making/running/lying/seeing"
    );
}

#[test]
fn present_participle_processor_preserves_capitalization_and_whitespace() {
    let rules = ruleset(
        r#"
        title = ["Run"]
        upper = ["RUN"]
        padded = ["  walk  "]
        origin = "{title | present_participle}/{upper | present_participle}/{padded | present_participle}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin").unwrap(),
        "Running/RUNNING/  walking  "
    );
}

#[test]
fn present_participle_processor_rejects_blank_input() {
    let rules = ruleset(
        r#"
        blank = ["  "]
        origin = "{blank | present_participle}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin"),
        Err(RenderError::ProcessorError {
            processor: "present_participle".to_string(),
            message: "input must contain one verb".to_string(),
        })
    );
}

#[test]
fn present_participle_processor_rejects_multiple_words() {
    let rules = ruleset(
        r#"
        phrase = ["walk home"]
        origin = "{phrase | present_participle}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin"),
        Err(RenderError::ProcessorError {
            processor: "present_participle".to_string(),
            message: "input must contain exactly one verb token".to_string(),
        })
    );
}

#[test]
fn ordinal_processor_adds_number_suffixes() {
    let rules = ruleset(
        r#"
        one = ["1"]
        two = ["2"]
        three = ["3"]
        four = ["4"]
        origin = "{one | ordinal}/{two | ordinal}/{three | ordinal}/{four | ordinal}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "1st/2nd/3rd/4th");
}

#[test]
fn ordinal_processor_handles_teen_exceptions_and_larger_numbers() {
    let rules = ruleset(
        r#"
        eleven = ["11"]
        twelve = ["12"]
        thirteen = ["13"]
        twenty_three = ["23"]
        origin = "{eleven | ordinal}/{twelve | ordinal}/{thirteen | ordinal}/{twenty_three | ordinal}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "11th/12th/13th/23rd");
}

#[test]
fn ordinal_processor_preserves_surrounding_whitespace_and_sign() {
    let rules = ruleset(
        r#"
        padded = ["  21  "]
        negative = ["-1"]
        origin = "{padded | ordinal}/{negative | ordinal}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "  21st  /-1st");
}

#[test]
fn ordinal_processor_rejects_non_integer_input() {
    let rules = ruleset(
        r#"
        word = ["first"]
        origin = "{word | ordinal}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin"),
        Err(RenderError::ProcessorError {
            processor: "ordinal".to_string(),
            message: "input must contain one integer".to_string(),
        })
    );
}

#[test]
fn ordinal_processor_rejects_multiple_words() {
    let rules = ruleset(
        r#"
        phrase = ["1 2"]
        origin = "{phrase | ordinal}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin"),
        Err(RenderError::ProcessorError {
            processor: "ordinal".to_string(),
            message: "input must contain exactly one integer token".to_string(),
        })
    );
}

#[test]
fn sentence_processor_capitalizes_first_alphabetic_character() {
    let rules = ruleset(
        r#"
        text = ["hello world"]
        origin = "{text | sentence}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Hello world");
}

#[test]
fn sentence_processor_skips_leading_punctuation() {
    let rules = ruleset(
        r#"
        text = ["  ...mia waits"]
        origin = "{text | sentence}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "  ...Mia waits");
}

#[test]
fn sentence_processor_leaves_rest_unchanged() {
    let rules = ruleset(
        r#"
        text = ["hello MIA"]
        origin = "{text | sentence}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Hello MIA");
}

#[test]
fn sentence_processor_returns_text_without_letters_unchanged() {
    let rules = ruleset(
        r#"
        empty = [""]
        punctuation = ["  ...  "]
        origin = "{empty | sentence}/{punctuation | sentence}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "/  ...  ");
}

#[test]
fn quote_processor_wraps_text_in_double_quotes() {
    let rules = ruleset(
        r#"
        text = ["Mia waits"]
        origin = "{text | quote}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "\"Mia waits\"");
}

#[test]
fn quote_processor_escapes_quotes_and_backslashes() {
    let rules = ruleset(
        r#"
        text = ["Mia said \"hi\" at C:\\tmp"]
        origin = "{text | quote}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin").unwrap(),
        "\"Mia said \\\"hi\\\" at C:\\\\tmp\""
    );
}

#[test]
fn quote_processor_wraps_empty_text() {
    let rules = ruleset(
        r#"
        text = [""]
        origin = "{text | quote}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "\"\"");
}

#[test]
fn slug_processor_lowercases_and_hyphenates_text() {
    let rules = ruleset(
        r#"
        text = ["Mia's Story"]
        origin = "{text | slug}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "mias-story");
}

#[test]
fn slug_processor_collapses_separator_runs() {
    let rules = ruleset(
        r#"
        text = ["  Mia -- finds_the 8th key!  "]
        origin = "{text | slug}"
        "#,
    );

    assert_eq!(
        rules.render_rule("origin").unwrap(),
        "mia-finds-the-8th-key"
    );
}

#[test]
fn slug_processor_removes_apostrophes_without_separators() {
    let rules = ruleset(
        r#"
        text = ["James' Lantern"]
        origin = "{text | slug}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "james-lantern");
}

#[test]
fn slug_processor_returns_empty_for_text_without_alphanumerics() {
    let rules = ruleset(
        r#"
        text = ["  !!!  "]
        origin = "{text | slug}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "");
}

#[test]
fn processor_pipeline_transforms_context_default() {
    let rules = ruleset(
        r#"
        name = ["mia"]
        origin = "{hero | titlecase}"
        context = {
            hero = "{name}"
        }
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Mia");
}

#[test]
fn bind_if_missing_stores_processed_value() {
    let rules = ruleset(
        r#"
        name = ["mia"]
        other = ["darcy"]
        origin = "{% hero:name | uppercase %}{% hero:other | lowercase %}{hero}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "MIA");
}

#[test]
fn overwrite_binding_stores_processed_value() {
    let rules = ruleset(
        r#"
        name = ["mia"]
        other = ["darcy"]
        origin = "{% hero:name | uppercase %}{% hero:=other | titlecase %}{hero}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Darcy");
}

#[test]
fn unknown_processor_returns_error_while_compiling_ruleset() {
    let value = hocon_rs::Config::parse_str::<hocon_rs::Value>(
        r#"
        name = ["Mia"]
        origin = "{name | missing}"
        "#,
        None,
    )
    .unwrap();

    match RuleSet::from_config(value) {
        Err(RenderError::UnknownProcessor(processor)) => assert_eq!(processor, "missing"),
        Err(error) => panic!("expected unknown processor, got {error:?}"),
        Ok(_) => panic!("expected unknown processor, got ruleset"),
    }
}

#[test]
fn empty_processor_returns_invalid_expression() {
    let value = hocon_rs::Config::parse_str::<hocon_rs::Value>(
        r#"
        name = ["Mia"]
        origin = "{name | }"
        "#,
        None,
    )
    .unwrap();

    match RuleSet::from_config(value) {
        Err(RenderError::InvalidExpression(expression)) => assert_eq!(expression, "name |"),
        Err(error) => panic!("expected invalid expression, got {error:?}"),
        Ok(_) => panic!("expected invalid expression, got ruleset"),
    }
}

#[test]
fn empty_pipeline_source_returns_invalid_expression() {
    let value = hocon_rs::Config::parse_str::<hocon_rs::Value>(
        r#"
        origin = "{ | uppercase}"
        "#,
        None,
    )
    .unwrap();

    match RuleSet::from_config(value) {
        Err(RenderError::InvalidExpression(expression)) => assert_eq!(expression, "| uppercase"),
        Err(error) => panic!("expected invalid expression, got {error:?}"),
        Ok(_) => panic!("expected invalid expression, got ruleset"),
    }
}

#[test]
fn custom_processor_can_be_registered_for_rust_rulesets() {
    let value = hocon_rs::Config::parse_str::<hocon_rs::Value>(
        r#"
        name = ["Mia"]
        origin = "{name | quote}"
        "#,
        None,
    )
    .unwrap();
    let mut processors = ProcessorRegistry::new();
    processors.insert(
        "quote".to_string(),
        processor(|value: &str| Ok(format!("'{value}'"))),
    );
    let rules = RuleSet::from_config_with_processors(value, processors).unwrap();

    assert_eq!(rules.render_rule("origin").unwrap(), "'Mia'");
}

#[test]
fn custom_processor_can_override_builtin_processor() {
    let value = hocon_rs::Config::parse_str::<hocon_rs::Value>(
        r#"
        name = ["Mia"]
        origin = "{name | uppercase}"
        "#,
        None,
    )
    .unwrap();
    let mut processors = ProcessorRegistry::new();
    processors.insert(
        "uppercase".to_string(),
        processor(|value: &str| Ok(format!("{value}!"))),
    );
    let rules = RuleSet::from_config_with_processors(value, processors).unwrap();

    assert_eq!(rules.render_rule("origin").unwrap(), "Mia!");
}

#[test]
fn custom_processor_errors_are_render_errors() {
    let value = hocon_rs::Config::parse_str::<hocon_rs::Value>(
        r#"
        name = ["Mia"]
        origin = "{name | fail}"
        "#,
        None,
    )
    .unwrap();
    let mut processors = ProcessorRegistry::new();
    processors.insert(
        "fail".to_string(),
        processor(|_value: &str| Err("not allowed".to_string())),
    );
    let rules = RuleSet::from_config_with_processors(value, processors).unwrap();

    assert_eq!(
        rules.render_rule("origin"),
        Err(RenderError::ProcessorError {
            processor: "fail".to_string(),
            message: "not allowed".to_string(),
        })
    );
}

#[test]
fn invalid_config_root_returns_error() {
    let config = hocon_rs::Value::String("not an object".to_string());

    match RuleSet::from_config(config) {
        Err(RenderError::InvalidConfigRoot) => {}
        Err(error) => panic!("expected invalid config root, got {error:?}"),
        Ok(_) => panic!("expected invalid config root, got ruleset"),
    }
}

#[test]
fn non_object_context_is_a_normal_rule() {
    let rules = ruleset(
        r#"
        context = "literal"
        "#,
    );

    assert_eq!(rules.render_rule("context").unwrap(), "literal");
}
