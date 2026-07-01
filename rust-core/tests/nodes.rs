use copperlace::render::{
    BindMode, BindNode, ChoiceNode, ProcessorPipelineNode, RenderError, RenderState, RuleCallNode,
    RuleSet, TextGeneratorNode, UnsupportedValueNode, VariableNode, VecNode, WeightedChoiceNode,
};

fn ruleset(config: &str) -> RuleSet {
    let value = hocon_rs::Config::parse_str::<hocon_rs::Value>(config, None).unwrap();
    RuleSet::from_config(value).unwrap()
}

fn empty_ruleset() -> RuleSet {
    ruleset("anchor = \"value\"")
}

#[test]
fn string_node_renders_literal_value() {
    let rules = empty_ruleset();
    let mut state = RenderState::new(&rules);
    let node = "literal".to_string();

    assert_eq!(node.generate_text(&mut state).unwrap(), "literal");
}

#[test]
fn variable_node_reads_value_bound_by_bind_node() {
    let rules = empty_ruleset();
    let mut state = RenderState::new(&rules);
    let bind = BindNode::new(
        "hero".to_string(),
        Box::new("Mia".to_string()),
        BindMode::IfMissing,
    );
    let variable = VariableNode::new("hero".to_string());

    assert_eq!(bind.generate_text(&mut state).unwrap(), "");
    assert_eq!(variable.generate_text(&mut state).unwrap(), "Mia");
}

#[test]
fn variable_node_returns_unknown_rule_when_unbound() {
    let rules = empty_ruleset();
    let mut state = RenderState::new(&rules);
    let variable = VariableNode::new("hero".to_string());

    assert_eq!(
        variable.generate_text(&mut state),
        Err(RenderError::UnknownRule("hero".to_string()))
    );
}

#[test]
fn rule_call_node_renders_named_rule() {
    let rules = ruleset("name = [\"Mia\"]");
    let mut state = RenderState::new(&rules);
    let node = RuleCallNode::new("name".to_string());

    assert_eq!(node.generate_text(&mut state).unwrap(), "Mia");
}

#[test]
fn rule_call_node_renders_and_caches_context_default() {
    let rules = ruleset(
        r#"
        name = ["Mia"]
        context = {
            hero = "{name}"
        }
        "#,
    );
    let mut state = RenderState::new(&rules);
    let node = RuleCallNode::new("hero".to_string());

    assert_eq!(node.generate_text(&mut state).unwrap(), "Mia");
    assert_eq!(node.generate_text(&mut state).unwrap(), "Mia");
}

#[test]
fn bind_node_if_missing_preserves_existing_value() {
    let rules = empty_ruleset();
    let mut state = RenderState::new(&rules);
    let first = BindNode::new(
        "hero".to_string(),
        Box::new("Mia".to_string()),
        BindMode::IfMissing,
    );
    let second = BindNode::new(
        "hero".to_string(),
        Box::new("Darcy".to_string()),
        BindMode::IfMissing,
    );
    let variable = VariableNode::new("hero".to_string());

    first.generate_text(&mut state).unwrap();
    second.generate_text(&mut state).unwrap();

    assert_eq!(variable.generate_text(&mut state).unwrap(), "Mia");
}

#[test]
fn bind_node_overwrite_replaces_existing_value() {
    let rules = empty_ruleset();
    let mut state = RenderState::new(&rules);
    let first = BindNode::new(
        "hero".to_string(),
        Box::new("Mia".to_string()),
        BindMode::IfMissing,
    );
    let second = BindNode::new(
        "hero".to_string(),
        Box::new("Darcy".to_string()),
        BindMode::Overwrite,
    );
    let variable = VariableNode::new("hero".to_string());

    first.generate_text(&mut state).unwrap();
    second.generate_text(&mut state).unwrap();

    assert_eq!(variable.generate_text(&mut state).unwrap(), "Darcy");
}

#[test]
fn choice_node_renders_one_child() {
    let rules = empty_ruleset();
    let mut state = RenderState::new(&rules);
    let node = ChoiceNode::new(vec![Box::new("Mia".to_string())]);

    assert_eq!(node.generate_text(&mut state).unwrap(), "Mia");
}

#[test]
fn choice_node_returns_empty_choice_for_no_children() {
    let rules = empty_ruleset();
    let mut state = RenderState::new(&rules);
    let node = ChoiceNode::new(Vec::new());

    assert_eq!(
        node.generate_text(&mut state),
        Err(RenderError::EmptyChoice)
    );
}

#[test]
fn choice_node_unique_selection_uses_each_child_once() {
    let rules = empty_ruleset();
    let mut state = RenderState::new(&rules);
    let node = ChoiceNode::new(vec![Box::new("Mia".to_string())]);

    assert_eq!(
        node.generate_unique_text("hero", &mut state).unwrap(),
        "Mia"
    );
    assert_eq!(
        node.generate_unique_text("hero", &mut state),
        Err(RenderError::ExhaustedUniqueChoice("hero".to_string()))
    );
}

#[test]
fn weighted_choice_node_renders_positive_weight_child() {
    let rules = empty_ruleset();
    let mut state = RenderState::new(&rules);
    let node = WeightedChoiceNode::new(vec![
        (
            Box::new("Mia".to_string()) as Box<dyn TextGeneratorNode>,
            0.0,
        ),
        (
            Box::new("Darcy".to_string()) as Box<dyn TextGeneratorNode>,
            2.5,
        ),
    ])
    .unwrap();

    assert_eq!(node.generate_text(&mut state).unwrap(), "Darcy");
}

#[test]
fn weighted_choice_node_unique_selection_uses_positive_unused_children() {
    let rules = empty_ruleset();
    let mut state = RenderState::new(&rules);
    let node = WeightedChoiceNode::new(vec![
        (
            Box::new("Mia".to_string()) as Box<dyn TextGeneratorNode>,
            1.0,
        ),
        (
            Box::new("Darcy".to_string()) as Box<dyn TextGeneratorNode>,
            0.0,
        ),
    ])
    .unwrap();

    assert_eq!(
        node.generate_unique_text("hero", &mut state).unwrap(),
        "Mia"
    );
    assert_eq!(
        node.generate_unique_text("hero", &mut state),
        Err(RenderError::ExhaustedUniqueChoice("hero".to_string()))
    );
}

#[test]
fn weighted_choice_node_rejects_all_zero_weights() {
    let node = WeightedChoiceNode::new(vec![
        (
            Box::new("Mia".to_string()) as Box<dyn TextGeneratorNode>,
            0.0,
        ),
        (
            Box::new("Darcy".to_string()) as Box<dyn TextGeneratorNode>,
            0.0,
        ),
    ]);

    assert!(matches!(node, Err(RenderError::InvalidWeightedChoice(_))));
}

#[test]
fn vec_node_concatenates_children() {
    let rules = ruleset("name = [\"Mia\"]");
    let mut state = RenderState::new(&rules);
    let node = VecNode::new(vec![
        Box::new("Hello ".to_string()),
        Box::new(RuleCallNode::new("name".to_string())),
    ]);

    assert_eq!(node.generate_text(&mut state).unwrap(), "Hello Mia");
}

#[test]
fn processor_pipeline_node_applies_processors_in_order() {
    let rules = empty_ruleset();
    let mut state = RenderState::new(&rules);
    let node = ProcessorPipelineNode::new(
        Box::new("  mIA  ".to_string()),
        vec![
            "trim".to_string(),
            "lowercase".to_string(),
            "capitalize".to_string(),
        ],
    );

    assert_eq!(node.generate_text(&mut state).unwrap(), "Mia");
}

#[test]
fn unsupported_value_node_returns_unsupported_value_error() {
    let rules = empty_ruleset();
    let mut state = RenderState::new(&rules);
    let node = UnsupportedValueNode::new("object".to_string());

    assert_eq!(
        node.generate_text(&mut state),
        Err(RenderError::UnsupportedValue("object".to_string()))
    );
}
