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
