//! Bottom-up grammar parser for copyright detection.
//!
//! Applies grammar rules to a sequence of POS-tagged tokens, building
//! a parse tree by replacing matched token/node spans with tree nodes.
//! Uses a single-pass approach matching Python's pygmars `loop=1` behavior.

use super::grammar::{GRAMMAR_RULES, GrammarRule, TagMatcher};
use super::types::{ParseNode, Token};

/// Parse a sequence of POS-tagged tokens into a parse tree.
///
/// Applies grammar rules bottom-up: scans the node sequence for patterns
/// that match a rule, replaces the matched span with a tree node, and
/// continues until no more rules fire (fixpoint).
///
/// Returns the final sequence of `ParseNode` (mix of leaf tokens and
/// tree nodes).
pub fn parse(tokens: Vec<Token>) -> Vec<ParseNode> {
    if tokens.is_empty() {
        return Vec::new();
    }

    let mut nodes: Vec<ParseNode> = tokens.into_iter().map(ParseNode::Leaf).collect();

    // Iterate until fixpoint (no rules fire in a full pass).
    // Safety bound to prevent infinite loops.
    let max_iterations = 50;
    for _ in 0..max_iterations {
        let mut changed = false;

        for rule in GRAMMAR_RULES.iter() {
            if let Some(new_nodes) = try_apply_rule(rule, &nodes) {
                nodes = new_nodes;
                changed = true;
                // After a rule fires, restart from the first rule
                // (greedy: always try earliest/highest-priority rules first).
                break;
            }
        }

        if !changed {
            break;
        }
    }

    nodes
}

/// Try to apply a single grammar rule to the node sequence.
/// Returns `Some(new_nodes)` if the rule matched somewhere, `None` otherwise.
fn try_apply_rule(rule: &GrammarRule, nodes: &[ParseNode]) -> Option<Vec<ParseNode>> {
    let pattern_len = rule.pattern.len();
    if pattern_len == 0 || nodes.len() < pattern_len {
        return None;
    }

    // Scan for the first position where the pattern matches.
    for start in 0..=(nodes.len() - pattern_len) {
        if matches_at(rule, nodes, start) {
            // Build the replacement tree node.
            let matched: Vec<ParseNode> = nodes[start..start + pattern_len].to_vec();
            let tree_node = ParseNode::Tree {
                label: rule.label,
                children: matched,
            };

            // Construct new node sequence: before + tree_node + after.
            let mut new_nodes = Vec::with_capacity(nodes.len() - pattern_len + 1);
            new_nodes.extend_from_slice(&nodes[..start]);
            new_nodes.push(tree_node);
            new_nodes.extend_from_slice(&nodes[start + pattern_len..]);

            return Some(new_nodes);
        }
    }

    None
}

/// Check if a rule's pattern matches the node sequence at position `start`.
fn matches_at(rule: &GrammarRule, nodes: &[ParseNode], start: usize) -> bool {
    for (i, matcher) in rule.pattern.iter().enumerate() {
        if !matcher_matches(matcher, &nodes[start + i]) {
            return false;
        }
    }
    true
}

/// Check if a single `TagMatcher` matches a single `ParseNode`.
fn matcher_matches(matcher: &TagMatcher, node: &ParseNode) -> bool {
    match matcher {
        TagMatcher::Tag(expected_tag) => node.tag() == Some(*expected_tag),

        TagMatcher::Label(expected_label) => node.label() == Some(*expected_label),

        TagMatcher::AnyTag(tags) => {
            if let Some(node_tag) = node.tag() {
                tags.contains(&node_tag)
            } else {
                false
            }
        }

        TagMatcher::AnyLabel(labels) => {
            if let Some(node_label) = node.label() {
                labels.contains(&node_label)
            } else {
                false
            }
        }

        TagMatcher::AnyTagOrLabel(tags, labels) => {
            if let Some(node_tag) = node.tag()
                && tags.contains(&node_tag)
            {
                return true;
            }
            if let Some(node_label) = node.label()
                && labels.contains(&node_label)
            {
                return true;
            }
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copyright::types::{PosTag, TreeLabel};

    fn make_token(value: &str, tag: PosTag, line: usize) -> Token {
        Token {
            value: value.to_string(),
            tag,
            start_line: line,
        }
    }

    #[test]
    fn test_parse_empty() {
        let result = parse(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_single_token() {
        let tokens = vec![make_token("hello", PosTag::Nn, 1)];
        let result = parse(tokens);
        assert_eq!(result.len(), 1);
        assert!(result[0].tag().is_some());
    }

    #[test]
    fn test_parse_year_range() {
        let tokens = vec![
            make_token("2020", PosTag::Yr, 1),
            make_token("-", PosTag::Dash, 1),
            make_token("2024", PosTag::Yr, 1),
        ];
        let result = parse(tokens);
        // Should be reduced to a single YR-RANGE node.
        assert_eq!(result.len(), 1, "result: {result:?}");
        assert_eq!(result[0].label(), Some(TreeLabel::YrRange));
    }

    #[test]
    fn test_parse_year_comma_year() {
        let tokens = vec![
            make_token("2020", PosTag::Yr, 1),
            make_token(",", PosTag::Cc, 1),
            make_token("2024", PosTag::Yr, 1),
        ];
        let result = parse(tokens);
        assert_eq!(result.len(), 1, "result: {result:?}");
        assert_eq!(result[0].label(), Some(TreeLabel::YrRange));
    }

    #[test]
    fn test_parse_preserves_unmatched() {
        let tokens = vec![
            make_token("hello", PosTag::Nn, 1),
            make_token("world", PosTag::Nn, 1),
        ];
        let result = parse(tokens);
        // Two NN tokens — no grammar rule matches NN NN, so both preserved.
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_matcher_tag() {
        let node = ParseNode::Leaf(make_token("2024", PosTag::Yr, 1));
        assert!(matcher_matches(&TagMatcher::Tag(PosTag::Yr), &node));
        assert!(!matcher_matches(&TagMatcher::Tag(PosTag::Nn), &node));
    }

    #[test]
    fn test_matcher_label() {
        let node = ParseNode::Tree {
            label: TreeLabel::YrRange,
            children: vec![],
        };
        assert!(matcher_matches(
            &TagMatcher::Label(TreeLabel::YrRange),
            &node
        ));
        assert!(!matcher_matches(&TagMatcher::Label(TreeLabel::Name), &node));
    }

    #[test]
    fn test_matcher_any_tag() {
        let node = ParseNode::Leaf(make_token("2024", PosTag::Yr, 1));
        assert!(matcher_matches(
            &TagMatcher::AnyTag(&[PosTag::Yr, PosTag::BareYr]),
            &node
        ));
        assert!(!matcher_matches(
            &TagMatcher::AnyTag(&[PosTag::Nn, PosTag::Cc]),
            &node
        ));
    }
}
