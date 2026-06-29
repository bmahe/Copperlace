use super::error::RenderError;
use super::nodes::{
    BindMode, BindNode, ProcessorPipelineNode, RuleCallNode, TextGeneratorNode, VecNode,
};
use super::processor::ProcessorRegistry;

pub(crate) fn template_to_node(
    template: &str,
    processors: &ProcessorRegistry,
) -> Result<Box<dyn TextGeneratorNode>, RenderError> {
    let mut nodes: Vec<Box<dyn TextGeneratorNode>> = Vec::new();
    let mut literal = String::new();
    let mut chars = template.char_indices().peekable();

    while let Some((index, character)) = chars.next() {
        match character {
            '\\' => match chars.peek() {
                Some((_, next_character)) if matches!(next_character, '{' | '}') => {
                    literal.push(*next_character);
                    chars.next();
                }
                _ => literal.push(character),
            },
            '{' => {
                if !literal.is_empty() {
                    nodes.push(Box::new(std::mem::take(&mut literal)));
                }

                if let Some((_, '%')) = chars.peek() {
                    chars.next();
                    let statement_start = index + character.len_utf8() + '%'.len_utf8();
                    let mut statement_end = None;
                    let mut previous_percent_index = None;
                    for (statement_index, statement_character) in chars.by_ref() {
                        if statement_character == '}'
                            && let Some(percent_index) = previous_percent_index
                        {
                            statement_end = Some(percent_index);
                            break;
                        }
                        previous_percent_index = if statement_character == '%' {
                            Some(statement_index)
                        } else {
                            None
                        };
                    }

                    let Some(statement_end) = statement_end else {
                        return Err(RenderError::InvalidExpression(
                            "unmatched opening statement delimiter in template".to_string(),
                        ));
                    };

                    let statement = template[statement_start..statement_end].trim();
                    nodes.push(statement_to_node(statement, processors)?);
                } else {
                    let expression_start = index + character.len_utf8();
                    let mut expression_end = None;
                    for (expression_index, expression_character) in chars.by_ref() {
                        if expression_character == '}' {
                            expression_end = Some(expression_index);
                            break;
                        }
                    }

                    let Some(expression_end) = expression_end else {
                        return Err(RenderError::InvalidExpression(
                            "unmatched opening brace in template".to_string(),
                        ));
                    };

                    let expression = template[expression_start..expression_end].trim();
                    nodes.push(expression_to_node(expression, processors)?);
                }
            }
            '%' => {
                if let Some((_, '}')) = chars.peek() {
                    return Err(RenderError::InvalidExpression(
                        "unmatched closing statement delimiter in template".to_string(),
                    ));
                } else {
                    literal.push(character);
                }
            }
            '}' => {
                return Err(RenderError::InvalidExpression(
                    "unmatched closing brace in template".to_string(),
                ));
            }
            _ => literal.push(character),
        }
    }

    if !literal.is_empty() {
        nodes.push(Box::new(literal));
    }

    Ok(Box::new(VecNode::new(nodes)))
}

fn statement_to_node(
    statement: &str,
    processors: &ProcessorRegistry,
) -> Result<Box<dyn TextGeneratorNode>, RenderError> {
    let (base_expression, processor_names) = parse_pipeline(statement, processors)?;

    if let Some((name, source)) = base_expression.split_once(":=") {
        return bind_node(
            statement,
            name,
            source,
            processor_names,
            BindMode::Overwrite,
        );
    }

    if let Some((name, source)) = base_expression.split_once(':') {
        return bind_node(
            statement,
            name,
            source,
            processor_names,
            BindMode::IfMissing,
        );
    }

    Err(RenderError::InvalidExpression(statement.to_string()))
}

fn expression_to_node(
    expression: &str,
    processors: &ProcessorRegistry,
) -> Result<Box<dyn TextGeneratorNode>, RenderError> {
    let (base_expression, processor_names) = parse_pipeline(expression, processors)?;

    if base_expression.contains(':') {
        return Err(RenderError::InvalidExpression(expression.to_string()));
    }

    let name = base_expression.trim();
    if name.is_empty() {
        return Err(RenderError::InvalidExpression(expression.to_string()));
    }
    Ok(pipeline_node(
        Box::new(RuleCallNode::new(name.to_string())),
        processor_names,
    ))
}

fn parse_pipeline<'a>(
    expression: &'a str,
    processors: &ProcessorRegistry,
) -> Result<(&'a str, Vec<String>), RenderError> {
    let mut parts = expression.split('|').map(str::trim);
    let base_expression = parts
        .next()
        .filter(|base_expression| !base_expression.is_empty())
        .ok_or_else(|| RenderError::InvalidExpression(expression.to_string()))?;
    let processor_names = parts
        .map(|processor_name| {
            if processor_name.is_empty() {
                return Err(RenderError::InvalidExpression(expression.to_string()));
            }
            if !processors.contains_key(processor_name) {
                return Err(RenderError::UnknownProcessor(processor_name.to_string()));
            }
            Ok(processor_name.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok((base_expression, processor_names))
}

fn bind_node(
    expression: &str,
    name: &str,
    source: &str,
    processor_names: Vec<String>,
    mode: BindMode,
) -> Result<Box<dyn TextGeneratorNode>, RenderError> {
    let name = name.trim();
    let source = source.trim();
    if name.is_empty() || source.is_empty() {
        return Err(RenderError::InvalidExpression(expression.to_string()));
    }
    let node = pipeline_node(
        Box::new(RuleCallNode::new(source.to_string())),
        processor_names,
    );
    Ok(Box::new(BindNode::new(name.to_string(), node, mode)))
}

fn pipeline_node(
    node: Box<dyn TextGeneratorNode>,
    processors: Vec<String>,
) -> Box<dyn TextGeneratorNode> {
    if processors.is_empty() {
        node
    } else {
        Box::new(ProcessorPipelineNode::new(node, processors))
    }
}
