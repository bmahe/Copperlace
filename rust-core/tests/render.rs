use copperlace::render::{ProcessorRegistry, processor};
use copperlace::{RenderError, RuleSet};

fn ruleset(config: &str) -> RuleSet {
    let value = hocon_rs::Config::parse_str::<hocon_rs::Value>(config, None).unwrap();
    RuleSet::from_config(value).unwrap()
}

#[test]
fn renders_from_multiple_named_rules() {
    let rules = ruleset(
        r#"
        name = ["Mia"]
        animal = ["owl"]
        story = ["{hero} and {heroPet}"]
        origin = "{hero:name}{heroPet:animal}{story}"
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
fn binding_reuses_the_same_generated_value() {
    let rules = ruleset(
        r#"
        name = ["Mia"]
        origin = "{hero:name}{hero}/{hero}"
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
        origin = "{hero:first}{hero:second}{hero}"
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
        origin = "{hero}{hero:other}/{hero}"
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
        origin = "{hero:first}{hero:=second}{hero}"
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
        origin = "{hero}{hero:=other}/{hero}"
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
fn overwrite_binding_whitespace_is_trimmed() {
    let rules = ruleset(
        r#"
        first = ["Mia"]
        second = ["Darcy"]
        origin = "{ hero:first }{ hero := second }{hero}"
        "#,
    );

    assert_eq!(rules.render_rule("origin").unwrap(), "Darcy");
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
        origin = "{hero:name | uppercase}{hero:other | lowercase}{hero}"
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
        origin = "{hero:name | uppercase}{hero:=other | titlecase}{hero}"
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
