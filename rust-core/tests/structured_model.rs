use std::collections::BTreeMap;

use copperlace::{CopperlaceNumber, CopperlaceValue, RenderError, RuleSet, StructuredNode};

fn ruleset(config: &str) -> RuleSet {
    let value = hocon_rs::Config::parse_str::<hocon_rs::Value>(config, None).unwrap();
    RuleSet::from_config(value).unwrap()
}

fn root_object(rules: &RuleSet) -> &BTreeMap<String, StructuredNode> {
    let StructuredNode::Object(values) = rules.structured_document() else {
        panic!("expected root object");
    };
    values
}

#[test]
fn top_level_list_compiles_to_structured_array_and_text_choice() {
    let rules = ruleset(
        r#"
        origin = ["red", "blue"]
        "#,
    );

    let root = root_object(&rules);
    let StructuredNode::Array(values) = root.get("origin").unwrap() else {
        panic!("expected structured array");
    };
    assert_eq!(values.len(), 2);
    assert!(matches!(values[0], StructuredNode::Text(_)));
    assert!(matches!(values[1], StructuredNode::Text(_)));

    let output = rules.render_rule("origin").unwrap();
    assert!(["red", "blue"].contains(&output.as_str()));
}

#[test]
fn object_values_compile_to_structured_objects_and_dotted_text_rules() {
    let rules = ruleset(
        r#"
        origin {
            title = "Scene"
            nested {
                mood = "Quiet"
            }
        }
        "#,
    );

    let root = root_object(&rules);
    let StructuredNode::Object(origin) = root.get("origin").unwrap() else {
        panic!("expected structured object");
    };
    assert!(matches!(
        origin.get("title").unwrap(),
        StructuredNode::Text(_)
    ));
    assert!(matches!(
        origin.get("nested").unwrap(),
        StructuredNode::Object(_)
    ));

    assert_eq!(rules.render_rule("origin.title").unwrap(), "Scene");
    assert_eq!(rules.render_rule("origin.nested.mood").unwrap(), "Quiet");
    assert_eq!(
        rules.render_rule("origin"),
        Err(RenderError::UnsupportedValue("object".to_string()))
    );
}

#[test]
fn structured_arrays_inside_objects_do_not_compile_as_choices() {
    let rules = ruleset(
        r#"
        origin {
            entries = [
                { value = "common", weight = 1 },
                { value = "rare", weight = 2 }
            ]
        }
        "#,
    );

    let root = root_object(&rules);
    let StructuredNode::Object(origin) = root.get("origin").unwrap() else {
        panic!("expected structured object");
    };
    let StructuredNode::Array(entries) = origin.get("entries").unwrap() else {
        panic!("expected structured array");
    };
    assert_eq!(entries.len(), 2);
    assert!(matches!(entries[0], StructuredNode::Object(_)));
    assert!(matches!(entries[1], StructuredNode::Object(_)));
}

#[test]
fn structured_scalars_compile_to_native_scalar_nodes() {
    let rules = ruleset(
        r#"
        origin {
            count = 3
            ratio = 2.5
            active = true
            missing = null
        }
        "#,
    );

    let root = root_object(&rules);
    let StructuredNode::Object(origin) = root.get("origin").unwrap() else {
        panic!("expected structured object");
    };
    assert!(matches!(
        origin.get("count").unwrap(),
        StructuredNode::Number(CopperlaceNumber::Integer(3))
    ));
    assert!(matches!(
        origin.get("ratio").unwrap(),
        StructuredNode::Number(CopperlaceNumber::Float(2.5))
    ));
    assert!(matches!(
        origin.get("active").unwrap(),
        StructuredNode::Boolean(true)
    ));
    assert!(matches!(
        origin.get("missing").unwrap(),
        StructuredNode::Null
    ));
}

#[test]
fn structured_text_leaves_use_text_generator_nodes() {
    let rules = ruleset(
        r#"
        origin {
            title = "Hello {name}"
        }
        "#,
    );

    let root = root_object(&rules);
    let StructuredNode::Object(origin) = root.get("origin").unwrap() else {
        panic!("expected structured object");
    };
    assert!(matches!(
        origin.get("title").unwrap(),
        StructuredNode::Text(_)
    ));
    assert_eq!(
        rules
            .render_rule_with_context(
                "origin.title",
                [("name".to_string(), "Mia".to_string())].into()
            )
            .unwrap(),
        "Hello Mia"
    );
}

#[test]
fn copperlace_value_converts_to_json_values() {
    let mut nested = BTreeMap::new();
    nested.insert(
        "array".to_string(),
        CopperlaceValue::Array(vec![
            CopperlaceValue::String("Mia".to_string()),
            CopperlaceValue::Number(CopperlaceNumber::Integer(3)),
            CopperlaceValue::Number(CopperlaceNumber::Float(2.5)),
            CopperlaceValue::Boolean(true),
            CopperlaceValue::Null,
        ]),
    );
    let value = CopperlaceValue::Object(nested);

    assert_eq!(
        value.to_json_value(),
        serde_json::json!({
            "array": ["Mia", 3, 2.5, true, null]
        })
    );
    assert_eq!(
        value.into_json_value(),
        serde_json::json!({
            "array": ["Mia", 3, 2.5, true, null]
        })
    );
}
