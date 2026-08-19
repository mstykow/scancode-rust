// SPDX-FileCopyrightText: Provenant contributors
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::copyright::types::{PosTag, TreeLabel};
use crate::models::LineNumber;

fn make_token(value: &str, tag: PosTag, line: usize) -> Token {
    Token {
        value: value.to_string(),
        tag,
        start_line: LineNumber::new(line).unwrap(),
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
fn test_parse_with_expired_deadline_returns_without_reducing() {
    let tokens = vec![
        make_token("2020", PosTag::Yr, 1),
        make_token("-", PosTag::Dash, 1),
        make_token("2024", PosTag::Yr, 1),
    ];

    let result = parse_with_deadline(tokens, Some(Instant::now()));
    assert_eq!(result.len(), 3);
    assert!(result.iter().all(|n| n.tag().is_some()));
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

#[test]
fn test_match_pattern_repetition_is_greedy_and_backtracks_to_anchor() {
    let pattern = [
        TagMatcher::Tag(PosTag::Copy),
        TagMatcher::OneOrMoreTagOrLabel(&[PosTag::Nn], &[]),
        TagMatcher::Tag(PosTag::Reserved),
    ];
    let nodes: Vec<ParseNode> = [
        make_token("Copyright", PosTag::Copy, 1),
        make_token("a", PosTag::Nn, 1),
        make_token("b", PosTag::Nn, 1),
        make_token("c", PosTag::Nn, 1),
        make_token("reserved.", PosTag::Reserved, 2),
        make_token("tail", PosTag::Nn, 2),
    ]
    .into_iter()
    .map(ParseNode::Leaf)
    .collect();

    // Greedy over the three NN tokens, then backtracked so the anchor matches;
    // the trailing token stays outside the match.
    assert_eq!(match_pattern(&pattern, &nodes), Some(5));

    // Repetition needs at least one node.
    assert_eq!(match_pattern(&pattern, &nodes[..1]), None);
    assert_eq!(
        match_pattern(&pattern, &[nodes[0].clone(), nodes[4].clone()]),
        None
    );
}

#[test]
fn test_match_pattern_repetition_is_bounded() {
    let pattern = [
        TagMatcher::Tag(PosTag::Copy),
        TagMatcher::OneOrMoreTagOrLabel(&[PosTag::Nn], &[]),
        TagMatcher::Tag(PosTag::Reserved),
    ];
    let mut tokens = vec![make_token("Copyright", PosTag::Copy, 1)];
    tokens.extend((0..MAX_REPETITION + 1).map(|_| make_token("word", PosTag::Nn, 1)));
    tokens.push(make_token("reserved.", PosTag::Reserved, 1));
    let nodes: Vec<ParseNode> = tokens.into_iter().map(ParseNode::Leaf).collect();

    // A run longer than the cap leaves the anchor out of reach, so degenerate
    // input cannot collapse into one enormous statement.
    assert_eq!(match_pattern(&pattern, &nodes), None);
}

/// Reduce a synthesized node sequence the way [`parse`] does, so a rule that
/// consumes tree labels can be exercised without authoring text that happens to
/// lex into them.
#[cfg(test)]
fn reduce_nodes(mut nodes: Vec<ParseNode>) -> Vec<ParseNode> {
    use crate::copyright::grammar::GRAMMAR_RULES;
    for _ in 0..50 {
        let mut changed = false;
        for rule in GRAMMAR_RULES.iter() {
            if let Some(next) = try_apply_rule(rule, &nodes) {
                nodes = next;
                changed = true;
                break;
            }
        }
        if !changed {
            break;
        }
    }
    nodes
}

#[cfg(test)]
fn empty_tree(label: TreeLabel) -> ParseNode {
    ParseNode::Tree {
        label,
        children: vec![],
    }
}

// Reduce with an explicit rule set, so a property can be attributed to the rule
// under test instead of to whichever table rule happens to cover the same shape.
#[cfg(test)]
fn reduce_with(rules: &[GrammarRule], mut nodes: Vec<ParseNode>) -> Vec<ParseNode> {
    for _ in 0..50 {
        let mut changed = false;
        for rule in rules {
            if let Some(next) = try_apply_rule(rule, &nodes) {
                nodes = next;
                changed = true;
                break;
            }
        }
        if !changed {
            break;
        }
    }
    nodes
}

// Most rules ported from an upstream `<X|Y>+` element were written out at a fixed
// width instead. That is only safe because a rule whose own label is also its
// first element re-applies on the next pass of the fixpoint loop: the run grows
// one element per pass, so the fixed width is not a ceiling. Exercise the rule
// alone — against the whole table another rule covering the same shape would keep
// this green after the recursion broke.
#[test]
fn test_fixpoint_loop_grows_a_left_recursive_rule_past_its_fixed_width() {
    let rule = GrammarRule {
        label: TreeLabel::Copyright,
        pattern: &[
            TagMatcher::Label(TreeLabel::Copyright),
            TagMatcher::Label(TreeLabel::Name),
        ],
    };
    for count in 1..=8 {
        let mut nodes = vec![empty_tree(TreeLabel::Copyright)];
        nodes.extend((0..count).map(|_| empty_tree(TreeLabel::Name)));
        let reduced = reduce_with(std::slice::from_ref(&rule), nodes);
        assert_eq!(
            (reduced.len(), reduced.first().and_then(|n| n.label())),
            (1, Some(TreeLabel::Copyright)),
            "one width-2 left-recursive rule must absorb {count} names by iterating"
        );
    }
}

// The counterpart, and the reason #1364 existed: iteration cannot grow a rule
// whose repetition is followed by an anchor, because each pass needs the anchor
// adjacent to the run. Such a rule caps at its written width no matter how many
// passes run, which is why these have to be expressed with the repeating matcher.
#[test]
fn test_fixpoint_loop_cannot_grow_an_anchored_rule_past_its_fixed_width() {
    let anchored = GrammarRule {
        label: TreeLabel::Copyright,
        pattern: &[
            TagMatcher::Label(TreeLabel::Copyright),
            TagMatcher::Tag(PosTag::Nn),
            TagMatcher::Label(TreeLabel::AllRightReserved),
        ],
    };
    let repeating = GrammarRule {
        label: TreeLabel::Copyright,
        pattern: &[
            TagMatcher::Label(TreeLabel::Copyright),
            TagMatcher::OneOrMoreTagOrLabel(&[PosTag::Nn], &[]),
            TagMatcher::Label(TreeLabel::AllRightReserved),
        ],
    };
    let sequence = |middles: usize| {
        let mut nodes = vec![empty_tree(TreeLabel::Copyright)];
        nodes.extend(
            (0..middles)
                .map(|_| make_token("word", PosTag::Nn, 1))
                .map(ParseNode::Leaf),
        );
        nodes.push(empty_tree(TreeLabel::AllRightReserved));
        nodes
    };

    assert_eq!(
        reduce_with(std::slice::from_ref(&anchored), sequence(1)).len(),
        1,
        "the anchored rule still matches at its written width"
    );
    for middles in 2..=6 {
        assert!(
            reduce_with(std::slice::from_ref(&anchored), sequence(middles)).len() > 1,
            "iteration must not rescue an anchored rule at {middles} middles"
        );
        assert_eq!(
            reduce_with(std::slice::from_ref(&repeating), sequence(middles)).len(),
            1,
            "the repeating matcher must cover {middles} middles"
        );
    }
}

// The behavioural guarantee the two properties above add up to: whatever the
// table looks like, a copyright followed by a run longer than any single rule's
// written width still comes back as one node. This holds through whichever rule
// happens to cover it, so it survives a rule being rewritten and fails only if
// the coverage disappears altogether.
#[test]
fn test_grammar_absorbs_a_name_run_longer_than_any_single_rule() {
    for count in 1..=8 {
        let mut nodes = vec![empty_tree(TreeLabel::Copyright)];
        nodes.extend((0..count).map(|_| empty_tree(TreeLabel::Name)));
        let reduced = reduce_nodes(nodes);
        assert_eq!(
            (reduced.len(), reduced.first().and_then(|n| n.label())),
            (1, Some(TreeLabel::Copyright)),
            "a copyright followed by {count} names must reduce to one node, got: {reduced:?}"
        );
    }
}

// The rules anchored on `ALLRIGHTRESERVED` are written out at two middle elements
// each and, per the test above, iteration cannot grow them. Longer runs reach the
// anchor only through the `#99999` catch-all, whose alternative set is a superset
// of theirs and whose repetition is unbounded. Narrowing that set would resurrect
// the truncation for the whole family at once.
#[test]
fn test_all_rights_reserved_anchor_is_reachable_past_the_fixed_width() {
    for middles in 1..=10 {
        let mut nodes = vec![empty_tree(TreeLabel::Copyright)];
        nodes.extend(
            (0..middles)
                .map(|_| make_token("word", PosTag::Nn, 1))
                .map(ParseNode::Leaf),
        );
        nodes.push(empty_tree(TreeLabel::AllRightReserved));
        let reduced = reduce_nodes(nodes);
        assert_eq!(
            (reduced.len(), reduced.first().and_then(|n| n.label())),
            (1, Some(TreeLabel::Copyright)),
            "{middles} middle elements before the anchor must still reduce to one node, got: {reduced:?}"
        );
    }
}
