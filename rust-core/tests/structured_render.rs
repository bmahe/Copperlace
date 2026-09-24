use std::collections::BTreeMap;

use copperlace::{
    ConfigError, Copperlace, CopperlaceNumber, CopperlaceValue, RenderContext, RenderError,
    RenderOptions, RuleSet, processor, render_config_rule_structured_with_context,
    render_file_inferred, render_file_inferred_with_context, render_file_structured,
    render_file_structured_with_context, render_str_inferred, render_str_inferred_with_context,
    render_str_structured, render_str_structured_with_context, ruleset_from_str,
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
fn structured_render_keeps_context_array_defaults_as_text_choices() {
    let rules = ruleset(
        r#"
        context {
            mood = ["calm"]
        }
        origin {
            mood = "{mood}"
        }
        "#,
    );

    assert_eq!(
        rules
            .render_rule_structured("origin")
            .unwrap()
            .to_json_value(),
        serde_json::json!({
            "mood": "calm"
        })
    );
}

#[test]
fn structured_text_leaves_share_unique_choice_state() {
    let rules = ruleset(
        r#"
        hero = [Mia, Lina]
        origin {
            first = "{hero!}"
            second = "{hero!}"
        }
        "#,
    );

    let value = rules.render_rule_structured("origin").unwrap();
    let CopperlaceValue::String(first) = object_field(&value, "first") else {
        panic!("expected string");
    };
    let CopperlaceValue::String(second) = object_field(&value, "second") else {
        panic!("expected string");
    };

    assert_ne!(first, second);
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
fn structured_render_preserves_large_unsigned_integers() {
    let value = render_str_structured(
        r#"
        origin {
            id = 18446744073709551615
        }
        "#,
        "origin",
    )
    .unwrap();

    assert_eq!(
        value.to_json_value(),
        serde_json::json!({
            "id": 18446744073709551615_u64
        })
    );
    assert_eq!(
        value.to_compact_json().unwrap(),
        r#"{"id":18446744073709551615}"#
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
fn structured_arrays_of_records_do_not_trigger_weighted_choice_validation() {
    let value = render_str_structured(
        r#"
        origin {
            sizes = [
                { value = "S", label = "small" },
                { weight = 2, label = "medium" }
            ]
        }
        "#,
        "origin",
    )
    .unwrap();

    assert_eq!(
        value.to_json_value(),
        serde_json::json!({
            "sizes": [
                { "label": "small", "value": "S" },
                { "label": "medium", "weight": 2 }
            ]
        })
    );
}

#[test]
fn dotted_text_rules_for_invalid_weighted_structured_arrays_are_unsupported() {
    let rules = ruleset(
        r#"
        origin {
            sizes = [
                { value = "S", label = "small" }
            ]
        }
        "#,
    );

    assert_eq!(
        rules.render_rule("origin.sizes"),
        Err(RenderError::UnsupportedValue("array".to_string()))
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
fn structured_render_uses_limited_recursion_options() {
    let rules = ruleset(
        r#"
        origin {
            value = "{part}"
        }
        part = "x{part}"
        "#,
    );

    assert_eq!(
        rules
            .render_rule_structured_with_options(
                "origin",
                RenderOptions {
                    max_recursion_depth: 1,
                },
            )
            .unwrap()
            .to_compact_json()
            .unwrap(),
        r#"{"value":"xx"}"#
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
            origin = [
                    { value = red, weight = 1 },
                    { nested = blue }
            ]
            "#
        ),
        Err(RenderError::InvalidWeightedChoice(_))
    ));
    assert!(matches!(
        ruleset_result(r#"origin { value = "{name | missing_processor}" }"#),
        Err(RenderError::UnknownProcessor(_))
    ));
}

#[test]
fn structured_text_leaf_can_concatenate_for_loop_output() {
    let rules = ruleset(
        r#"
        items = [apple, pear]
        origin {
          summary = """{% for item in items %}{item};{% endfor %}"""
          preserved = [one, two]
        }
        "#,
    );

    let rendered = rules.render_rule_structured("origin").unwrap();
    assert_eq!(
        object_field(&rendered, "summary"),
        &CopperlaceValue::String("apple;pear;".to_string())
    );
    assert_eq!(
        object_field(&rendered, "preserved"),
        &CopperlaceValue::Array(vec![
            CopperlaceValue::String("one".to_string()),
            CopperlaceValue::String("two".to_string()),
        ])
    );
}

#[test]
fn structured_array_loop_emits_typed_values_in_source_order() {
    let config = r#"
        items = [
            { name = "Mia" },
            { name = "Lina" }
        ]
        origin {
            entries = [
                "before",
                {% for item in items %}
                {
                    name = "{item.name}",
                    index = "{loop.index}",
                    first = "{loop.first}",
                    quantity = 3,
                    active = true,
                    missing = null
                }
                {% endfor %},
                "after"
            ]
        }
    "#;

    let rendered = render_str_structured(config, "origin").unwrap();
    assert_eq!(
        rendered.to_json_value(),
        serde_json::json!({
            "entries": [
                "before",
                {
                    "name": "Mia", "index": "1", "first": "true",
                    "quantity": 3, "active": true, "missing": null
                },
                {
                    "name": "Lina", "index": "2", "first": "false",
                    "quantity": 3, "active": true, "missing": null
                },
                "after"
            ]
        })
    );

    let rules = ruleset_from_str(config).unwrap();
    let copperlace::StructuredNode::Object(document) = rules.structured_document() else {
        panic!("expected root object");
    };
    let copperlace::StructuredNode::Object(origin) = document.get("origin").unwrap() else {
        panic!("expected origin object");
    };
    let copperlace::StructuredNode::Array(entries) = origin.get("entries").unwrap() else {
        panic!("expected array");
    };
    assert!(entries[0].value_node().is_some());
    assert_eq!(entries[1].iteration().unwrap().1, "items");
}

#[test]
fn structured_source_loops_leave_quoted_templates_and_comments_untouched() {
    let rendered = render_str_structured(
        r#"
        items = ["Mia"]
        origin {
            quoted = "{% for item in items %}{item}{% endfor %}"
            # {% for ignored in items %} this is only a comment {% endfor %}
            entries = [
                {% for item in items %}"{item}"{% endfor %}
            ]
        }
        "#,
        "origin",
    )
    .unwrap();

    assert_eq!(
        rendered.to_json_value(),
        serde_json::json!({"quoted": "Mia", "entries": ["Mia"]})
    );
}

#[test]
fn structured_array_loops_can_nest_and_restore_outer_metadata() {
    let config = r#"
        groups = [
            { name = "A", children = ["one", "two"] },
            { name = "B", children = ["three"] }
        ]
        origin {
            groups = [
                {% for group in groups %}
                {
                    name = "{group.name}",
                    outer_index = "{loop.index}",
                    children = [
                        {% for child in group.children %}
                        "{loop.index}:{child}"
                        {% endfor %}
                    ]
                }
                {% endfor %}
            ]
        }
    "#;

    assert_eq!(
        render_str_structured(config, "origin")
            .unwrap()
            .to_json_value(),
        serde_json::json!({
            "groups": [
                {"name": "A", "outer_index": "1", "children": ["1:one", "2:two"]},
                {"name": "B", "outer_index": "2", "children": ["1:three"]}
            ]
        })
    );
}

#[test]
fn structured_array_loops_allow_empty_sources_and_report_invalid_sources() {
    assert_eq!(
        render_str_structured(
            r#"
            items = []
            origin {
                entries = [
                    {% for item in items %}"{item}"{% endfor %}
                ]
            }
            "#,
            "origin"
        )
        .unwrap()
        .to_json_value(),
        serde_json::json!({"entries": []})
    );

    let rules = ruleset_from_str(
        r#"
        items = "one"
        origin {
            entries = [
                {% for item in items %}"{item}"{% endfor %}
            ]
        }
        "#,
    )
    .unwrap();
    assert!(matches!(
        rules.render_rule_structured("origin"),
        Err(RenderError::UnsupportedIterationSource { source, value_type })
            if source == "items" && value_type == "string"
    ));

    assert!(matches!(
        ruleset_from_str(
            r#"
            items = ["one"]
            origin {
                entries = {% for item in items %}"{item}"{% endfor %}
            }
            "#
        ),
        Err(ConfigError::Render(RenderError::InvalidExpression(_)))
    ));

    assert!(matches!(
        ruleset_from_str(
            r#"
            items = ["one"]
            origin {
                entries = [
                    {% for item in items %}"{item}"
                ]
            }
            "#
        ),
        Err(ConfigError::Parse(_))
    ));
}

#[test]
fn structured_array_loops_render_from_files() {
    let path = std::env::temp_dir().join(format!(
        "copperlace-structured-loop-{}.conf",
        std::process::id()
    ));
    std::fs::write(
        &path,
        r#"
        items = ["one", "two"]
        origin {
            entries = [
                {% for item in items %}"{loop.index}:{item}"{% endfor %}
            ]
        }
        "#,
    )
    .unwrap();

    assert_eq!(
        render_file_structured(&path, "origin")
            .unwrap()
            .to_json_value(),
        serde_json::json!({"entries": ["1:one", "2:two"]})
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn structured_loops_render_from_each_conf_include_form() {
    let directory = std::env::temp_dir().join(format!(
        "copperlace-include-loops-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let fragment = directory.join("fragment.conf");
    std::fs::write(
        &fragment,
        r#"
        from_include = ${shared}
        origin {
            entries = [
                {% for item in items %}"{loop.index}:{item}"{% endfor %}
            ]
        }
        "#,
    )
    .unwrap();

    for (name, directive) in [
        (
            "classpath",
            "include classpath(\"fragment.conf\")".to_string(),
        ),
        ("bare", "include \"fragment.conf\"".to_string()),
        ("file", format!("include file(\"{}\")", fragment.display())),
    ] {
        let root = directory.join(format!("{name}.conf"));
        std::fs::write(
            &root,
            format!(
                "shared = ready\nitems = [one, two]\norigin {{ label = \"{{from_include}}\" }}\n{directive}\n"
            ),
        )
        .unwrap();
        assert_eq!(
            render_file_structured(&root, "origin")
                .unwrap()
                .to_json_value(),
            serde_json::json!({"entries": ["1:one", "2:two"], "label": "ready"}),
            "{name} include"
        );
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn nested_and_extensionless_includes_keep_hocon_merging() {
    let directory = std::env::temp_dir().join(format!(
        "copperlace-nested-loops-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("inner.conf"),
        r#"origin.entries = [{% for item in items %}"{item}"{% endfor %}]"#,
    )
    .unwrap();
    std::fs::write(
        directory.join("fragment.conf"),
        "include classpath(\"inner.conf\")\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("fragment.json"),
        r#"{"origin":{"extra":true}}"#,
    )
    .unwrap();
    let root = directory.join("root.conf");
    std::fs::write(
        &root,
        "items = [one, two]\ninclude classpath(\"fragment\")\n",
    )
    .unwrap();

    assert_eq!(
        render_file_structured(&root, "origin")
            .unwrap()
            .to_json_value(),
        serde_json::json!({"entries": ["one", "two"], "extra": true})
    );
    std::fs::write(
        directory.join("root.json"),
        r#"{"origin":{"from_root_json":"kept"}}"#,
    )
    .unwrap();
    assert_eq!(
        render_file_structured(directory.join("root"), "origin")
            .unwrap()
            .to_json_value(),
        serde_json::json!({
            "entries": ["one", "two"],
            "extra": true,
            "from_root_json": "kept"
        })
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn nested_object_include_keeps_relative_substitutions() {
    let directory = std::env::temp_dir().join(format!(
        "copperlace-object-include-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("detail.conf"),
        r#"
        entries = [{% for item in items %}"{item}"{% endfor %}]
        copied = ${label}
        "#,
    )
    .unwrap();
    let root = directory.join("root.conf");
    std::fs::write(
        &root,
        r#"
        items = [one]
        origin {
            label = ready
            include classpath("detail.conf")
        }
        "#,
    )
    .unwrap();
    assert_eq!(
        render_file_structured(&root, "origin")
            .unwrap()
            .to_json_value(),
        serde_json::json!({"label": "ready", "copied": "ready", "entries": ["one"]})
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn included_loops_preserve_optional_required_and_cycle_errors() {
    let directory = std::env::temp_dir().join(format!(
        "copperlace-include-errors-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let root = directory.join("root.conf");
    std::fs::write(
        &root,
        r#"
        include classpath("missing.conf")
        # include required(classpath("also-missing.conf"))
        # __copperlace_internal_0_ forces another marker namespace
        items = [one]
        origin.note = "include classpath(\"missing.conf\")"
        origin.entries = [{% for item in items %}"{item}"{% endfor %}]
        "#,
    )
    .unwrap();
    assert_eq!(
        render_file_structured(&root, "origin")
            .unwrap()
            .to_json_value(),
        serde_json::json!({
            "entries": ["one"],
            "note": "include classpath(\"missing.conf\")"
        })
    );

    std::fs::write(&root, "include required(classpath(\"missing.conf\"))\n").unwrap();
    assert!(matches!(
        render_file_structured(&root, "origin"),
        Err(ConfigError::Parse(message)) if message.contains("missing.conf")
    ));

    std::fs::write(&root, "include classpath(\"cycle.conf\")\n").unwrap();
    std::fs::write(
        directory.join("cycle.conf"),
        "include classpath(\"root.conf\")\n",
    )
    .unwrap();
    assert!(matches!(
        render_file_structured(&root, "origin"),
        Err(ConfigError::Parse(message)) if message.contains("include cycle")
    ));
    std::fs::remove_dir_all(directory).unwrap();
}
