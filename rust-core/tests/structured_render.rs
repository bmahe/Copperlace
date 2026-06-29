use std::collections::BTreeMap;

use copperlace::{
    ConfigError, Copperlace, CopperlaceNumber, CopperlaceValue, RenderContext, RenderError,
    RuleSet, processor, render_config_rule_structured_with_context, render_file_inferred,
    render_file_inferred_with_context, render_file_structured, render_file_structured_with_context,
    render_str_inferred, render_str_inferred_with_context, render_str_structured,
    render_str_structured_with_context,
};

fn ruleset(config: &str) -> RuleSet {
    let value = hocon_rs::Config::parse_str::<hocon_rs::Value>(config, None).unwrap();
    RuleSet::from_config(value).unwrap()
}

fn ruleset_result(config: &str) -> Result<RuleSet, RenderError> {
    let value = hocon_rs::Config::parse_str::<hocon_rs::Value>(config, None).unwrap();
    RuleSet::from_config(value)
}

fn object_field<'a>(value: &'a CopperlaceValue, field: &str) -> &'a CopperlaceValue {
    let CopperlaceValue::Object(values) = value else {
        panic!("expected object");
    };
    values.get(field).unwrap()
}

#[test]
fn object_valued_rule_renders_to_structured_object() {
    let rules = ruleset(
        r#"
        origin {
            kind = "scene"
            count = 3
            active = true
            missing = null
        }
        "#,
    );

    let mut expected = BTreeMap::new();
    expected.insert(
        "kind".to_string(),
        CopperlaceValue::String("scene".to_string()),
    );
    expected.insert(
        "count".to_string(),
        CopperlaceValue::Number(CopperlaceNumber::Integer(3)),
    );
    expected.insert("active".to_string(), CopperlaceValue::Boolean(true));
    expected.insert("missing".to_string(), CopperlaceValue::Null);

    assert_eq!(
        rules.render_rule_structured("origin").unwrap(),
        CopperlaceValue::Object(expected)
    );
}

#[test]
fn nested_objects_and_arrays_preserve_shape() {
    let rules = ruleset(
        r#"
        origin {
            nested {
                values = ["one", "two"]
            }
        }
        "#,
    );

    assert_eq!(
        rules
            .render_rule_structured("origin")
            .unwrap()
            .to_json_value(),
        serde_json::json!({
            "nested": {
                "values": ["one", "two"]
            }
        })
    );
}

#[test]
fn arrays_inside_structured_objects_render_as_arrays_not_choices() {
    let rules = ruleset(
        r#"
        origin {
            values = ["red", "blue"]
        }
        "#,
    );

    assert_eq!(
        rules
            .render_rule_structured("origin")
            .unwrap()
            .to_json_value(),
        serde_json::json!({
            "values": ["red", "blue"]
        })
    );
}

#[test]
fn named_list_rules_referenced_from_structured_text_remain_choices() {
    let rules = ruleset(
        r#"
        hero = ["Mia"]
        origin {
            name = "{hero}"
        }
        "#,
    );

    assert_eq!(
        rules
            .render_rule_structured("origin")
            .unwrap()
            .to_json_value(),
        serde_json::json!({
            "name": "Mia"
        })
    );
}

#[test]
fn structured_arrays_share_bindings_in_order() {
    let rules = ruleset(
        r#"
        hero = ["Mia"]
        fallback = ["Darcy"]
        origin {
            entries = [
                "{% name:hero %}{name}",
                "{name}",
                "{% name:fallback %}{name}"
            ]
        }
        "#,
    );

    assert_eq!(
        rules
            .render_rule_structured("origin")
            .unwrap()
            .to_json_value(),
        serde_json::json!({
            "entries": ["Mia", "Mia", "Mia"]
        })
    );
}

#[test]
fn structured_arrays_apply_overwrite_bindings_in_order() {
    let rules = ruleset(
        r#"
        first = ["Mia"]
        second = ["Darcy"]
        origin {
            entries = [
                "{% name:first %}{name}",
                "{% name:=second %}{name}",
                "{name}"
            ]
        }
        "#,
    );

    assert_eq!(
        rules
            .render_rule_structured("origin")
            .unwrap()
            .to_json_value(),
        serde_json::json!({
            "entries": ["Mia", "Darcy", "Darcy"]
        })
    );
}

#[test]
fn repeated_structured_renders_start_with_fresh_state() {
    let rules = ruleset(
        r#"
        first = ["Mia"]
        second = ["Darcy"]
        origin {
            before = "{% name:first %}{name}"
            overwrite = "{% name:=second %}{name}"
        }
        "#,
    );

    assert_eq!(
        object_field(&rules.render_rule_structured("origin").unwrap(), "before"),
        &CopperlaceValue::String("Mia".to_string())
    );
    assert_eq!(
        object_field(&rules.render_rule_structured("origin").unwrap(), "before"),
        &CopperlaceValue::String("Mia".to_string())
    );
}

#[test]
fn structured_render_uses_context_defaults_and_initial_context() {
    let rules = ruleset(
        r#"
        context {
            name = "Mia"
        }
        origin {
            greeting = "Hello {name}"
        }
        "#,
    );
    let mut context = RenderContext::new();
    context.insert("name".to_string(), "Lina".to_string());

    assert_eq!(
        rules
            .render_rule_structured_with_context("origin", context)
            .unwrap()
            .to_json_value(),
        serde_json::json!({
            "greeting": "Hello Lina"
        })
    );
}

#[test]
fn structured_object_fields_use_context_defaults_without_order_dependency() {
    let rules = ruleset(
        r#"
        name = ["Mia"]
        context {
            hero = "{name}"
        }
        origin {
            summary = "{hero} visits"
            visitor = "{hero}"
        }
        "#,
    );

    assert_eq!(
        rules
            .render_rule_structured("origin")
            .unwrap()
            .to_json_value(),
        serde_json::json!({
            "summary": "Mia visits",
            "visitor": "Mia"
        })
    );
}

#[test]
fn structured_render_supports_processors() {
    let rules = ruleset(
        r#"
        origin {
            title = "{name | trim | titlecase}"
        }
        "#,
    );
    let mut context = RenderContext::new();
    context.insert("name".to_string(), "  mia reed  ".to_string());

    assert_eq!(
        rules
            .render_rule_structured_with_context("origin", context)
            .unwrap()
            .to_json_value(),
        serde_json::json!({
            "title": "Mia Reed"
        })
    );
}

#[test]
fn structured_render_supports_custom_processors() {
    let value = hocon_rs::Config::parse_str::<hocon_rs::Value>(
        r#"
        origin {
            name = "{hero | surround}"
        }
        "#,
        None,
    )
    .unwrap();
    let mut processors = copperlace::ProcessorRegistry::new();
    processors.insert(
        "surround".to_string(),
        processor(|value: &str| Ok(format!("[{value}]"))),
    );
    let rules = RuleSet::from_config_with_processors(value, processors).unwrap();
    let mut context = RenderContext::new();
    context.insert("hero".to_string(), "Mia".to_string());

    assert_eq!(
        rules
            .render_rule_structured_with_context("origin", context)
            .unwrap()
            .to_json_value(),
        serde_json::json!({
            "name": "[Mia]"
        })
    );
}

#[test]
fn copperlace_structured_methods_render_loaded_config() {
    let copperlace = Copperlace::from_str(
        r#"
        origin {
            name = "{name}"
        }
        "#,
    )
    .unwrap();
    let mut context = RenderContext::new();
    context.insert("name".to_string(), "Mia".to_string());

    assert_eq!(
        copperlace
            .render_structured_with_context("origin", context)
            .unwrap()
            .to_json_value(),
        serde_json::json!({
            "name": "Mia"
        })
    );
    assert!(matches!(
        copperlace.render_structured("name"),
        Err(RenderError::UnknownRule(_))
    ));
}

#[test]
fn string_file_and_config_helpers_render_structured_values() {
    let mut context = RenderContext::new();
    context.insert("name".to_string(), "Mia".to_string());
    let config = r#"
        origin {
            name = "{name}"
        }
        "#;
    let value = hocon_rs::Config::parse_str::<hocon_rs::Value>(config, None).unwrap();
    let config_path =
        std::env::temp_dir().join(format!("copperlace-structured-{}.conf", std::process::id()));
    std::fs::write(&config_path, config).unwrap();

    assert_eq!(
        render_str_structured_with_context(config, "origin", context.clone())
            .unwrap()
            .to_json_value(),
        serde_json::json!({"name": "Mia"})
    );
    assert_eq!(
        render_config_rule_structured_with_context(value, "origin", context.clone())
            .unwrap()
            .to_json_value(),
        serde_json::json!({"name": "Mia"})
    );
    assert_eq!(
        render_file_structured_with_context(&config_path, "origin", context)
            .unwrap()
            .to_json_value(),
        serde_json::json!({"name": "Mia"})
    );
    assert_eq!(
        render_str_structured(r#"origin { name = "Lina" }"#, "origin")
            .unwrap()
            .to_json_value(),
        serde_json::json!({"name": "Lina"})
    );
    std::fs::write(&config_path, r#"origin { name = "Mia" }"#).unwrap();
    assert_eq!(
        render_file_structured(&config_path, "origin")
            .unwrap()
            .to_json_value(),
        serde_json::json!({"name": "Mia"})
    );

    let _ = std::fs::remove_file(config_path);
}

#[test]
fn inferred_render_returns_text_or_formatted_structured_json() {
    let rules = ruleset(
        r#"
        text = "Mia"
        choice = ["Lina"]
        origin {
            greeting = "Hello {name}"
        }
        "#,
    );
    let mut context = RenderContext::new();
    context.insert("name".to_string(), "Darcy".to_string());

    assert_eq!(rules.render_rule_inferred("text").unwrap(), "Mia");
    assert_eq!(rules.render_rule_inferred("choice").unwrap(), "Lina");
    let structured = rules
        .render_rule_inferred_with_context("origin", context)
        .unwrap();
    assert_eq!(structured, "{\n\t\"greeting\": \"Hello Darcy\"\n}");
}

#[test]
fn copperlace_and_helpers_render_inferred_strings() {
    let config = r#"
        text = "Mia"
        origin {
            greeting = "Hello {name}"
        }
        "#;
    let copperlace = Copperlace::from_str(config).unwrap();
    let mut context = RenderContext::new();
    context.insert("name".to_string(), "Lina".to_string());
    let config_path =
        std::env::temp_dir().join(format!("copperlace-inferred-{}.conf", std::process::id()));
    std::fs::write(&config_path, config).unwrap();

    assert_eq!(copperlace.render_inferred("text").unwrap(), "Mia");
    assert_eq!(
        copperlace
            .render_inferred_with_context("origin", context.clone())
            .unwrap(),
        "{\n\t\"greeting\": \"Hello Lina\"\n}"
    );
    assert_eq!(
        render_str_inferred_with_context(config, "origin", context.clone()).unwrap(),
        "{\n\t\"greeting\": \"Hello Lina\"\n}"
    );
    assert_eq!(
        render_file_inferred_with_context(&config_path, "origin", context).unwrap(),
        "{\n\t\"greeting\": \"Hello Lina\"\n}"
    );
    assert_eq!(render_str_inferred(config, "text").unwrap(), "Mia");
    assert_eq!(render_file_inferred(&config_path, "text").unwrap(), "Mia");

    let _ = std::fs::remove_file(config_path);
}

#[test]
fn compact_and_formatted_json_serialization_are_valid() {
    let value = render_str_structured(
        r#"
        origin {
            name = "Mia"
            tags = ["generated", "scene"]
        }
        "#,
        "origin",
    )
    .unwrap();

    let compact = value.to_compact_json().unwrap();
    assert_eq!(compact, r#"{"name":"Mia","tags":["generated","scene"]}"#);
    assert!(!compact.contains('\n'));
    assert!(!compact.contains('\t'));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&compact).unwrap(),
        value.to_json_value()
    );

    let formatted = value.to_formatted_json().unwrap();
    assert!(formatted.contains('\n'));
    assert!(formatted.contains('\t'));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&formatted).unwrap(),
        value.to_json_value()
    );
}

#[test]
fn text_apis_for_string_and_list_rules_stay_unchanged() {
    let rules = ruleset(
        r#"
        text = "Mia"
        choice = ["red", "blue"]
        "#,
    );

    assert_eq!(rules.render_rule("text").unwrap(), "Mia");
    assert!(["red", "blue"].contains(&rules.render_rule("choice").unwrap().as_str()));
}

#[test]
fn non_object_structured_targets_return_clear_errors() {
    let rules = ruleset(
        r#"
        text = "Mia"
        choice = ["red", "blue"]
        "#,
    );

    assert_eq!(
        rules.render_rule_structured("text"),
        Err(RenderError::UnsupportedStructuredTarget("text".to_string()))
    );
    assert_eq!(
        rules.render_rule_structured("choice"),
        Err(RenderError::UnsupportedStructuredTarget(
            "choice".to_string()
        ))
    );
}

#[test]
fn unknown_structured_rule_returns_unknown_rule() {
    let rules = ruleset(r#"origin { name = "Mia" }"#);

    assert_eq!(
        rules.render_rule_structured("missing"),
        Err(RenderError::UnknownRule("missing".to_string()))
    );
}

#[test]
fn structured_render_surfaces_text_leaf_errors() {
    assert_eq!(
        render_str_structured(r#"origin { value = "{missing}" }"#, "origin"),
        Err(ConfigError::Render(RenderError::UnknownRule(
            "missing".to_string()
        )))
    );
    assert_eq!(
        render_str_structured(
            r#"
            empty = []
            origin { value = "{empty}" }
            "#,
            "origin"
        ),
        Err(ConfigError::Render(RenderError::EmptyChoice))
    );
}

#[test]
fn structured_render_surfaces_processor_failures_and_cycles() {
    let value = hocon_rs::Config::parse_str::<hocon_rs::Value>(
        r#"
        origin {
            value = "{name | fail}"
        }
        "#,
        None,
    )
    .unwrap();
    let mut processors = copperlace::ProcessorRegistry::new();
    processors.insert(
        "fail".to_string(),
        processor(|_value: &str| Err("not allowed".to_string())),
    );
    let rules = RuleSet::from_config_with_processors(value, processors).unwrap();
    let mut context = RenderContext::new();
    context.insert("name".to_string(), "Mia".to_string());

    assert_eq!(
        rules.render_rule_structured_with_context("origin", context),
        Err(RenderError::ProcessorError {
            processor: "fail".to_string(),
            message: "not allowed".to_string(),
        })
    );

    let rules = ruleset(
        r#"
        a = "{b}"
        b = "{a}"
        origin { value = "{a}" }
        "#,
    );
    assert_eq!(
        rules.render_rule_structured("origin"),
        Err(RenderError::CircularRuleReference(vec![
            "a".to_string(),
            "b".to_string(),
            "a".to_string(),
        ]))
    );
}

#[test]
fn structured_compile_errors_cover_invalid_config_and_weighted_choices() {
    let invalid_root = hocon_rs::Value::String("not an object".to_string());
    assert!(matches!(
        RuleSet::from_config(invalid_root),
        Err(RenderError::InvalidConfigRoot)
    ));
    assert!(matches!(
        ruleset_result(
            r#"
            origin {
                values = [
                    { value = red, weight = 1 },
                    { nested = blue }
                ]
            }
            "#
        ),
        Err(RenderError::InvalidWeightedChoice(_))
    ));
    assert!(matches!(
        ruleset_result(r#"origin { value = "{name | missing_processor}" }"#),
        Err(RenderError::UnknownProcessor(_))
    ));
}
