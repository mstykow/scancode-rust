//! Copyright detection orchestrator.
//!
//! Runs the full detection pipeline: text → numbered lines → candidate groups
//! → tokens → parse tree → walk tree → refine → filter junk → detections.
//!
//! The grammar currently builds lower-level structures (Name, Company,
//! YrRange, etc.) but does not yet produce top-level COPYRIGHT/AUTHOR tree
//! nodes. This detector handles both cases:
//! - If the grammar produces COPYRIGHT/AUTHOR nodes, use them directly.
//! - Otherwise, scan the flat node sequence for COPY/AUTH tokens and
//!   collect spans heuristically.

use super::candidates::collect_candidate_lines;
use super::lexer::get_tokens;
use super::parser::parse;
use super::refiner::{is_junk_copyright, refine_author, refine_copyright, refine_holder};
use super::types::{
    AuthorDetection, CopyrightDetection, HolderDetection, ParseNode, PosTag, Token, TreeLabel,
};

const NON_COPYRIGHT_LABELS: &[TreeLabel] = &[];
const NON_HOLDER_LABELS: &[TreeLabel] = &[TreeLabel::YrRange, TreeLabel::YrAnd];
const NON_HOLDER_LABELS_MINI: &[TreeLabel] = &[TreeLabel::YrRange, TreeLabel::YrAnd];

const NON_HOLDER_POS_TAGS: &[PosTag] = &[
    PosTag::Copy,
    PosTag::Yr,
    PosTag::YrPlus,
    PosTag::BareYr,
    PosTag::Email,
    PosTag::Url,
    PosTag::Holder,
    PosTag::Is,
    PosTag::Held,
];

const NON_HOLDER_POS_TAGS_MINI: &[PosTag] = &[
    PosTag::Copy,
    PosTag::Yr,
    PosTag::YrPlus,
    PosTag::BareYr,
    PosTag::Is,
    PosTag::Held,
];

const NON_AUTHOR_POS_TAGS: &[PosTag] = &[
    PosTag::Copy,
    PosTag::Yr,
    PosTag::YrPlus,
    PosTag::BareYr,
    PosTag::Auth,
    PosTag::Auth2,
    PosTag::Auths,
    PosTag::AuthDot,
    PosTag::Contributors,
    PosTag::Commit,
    PosTag::SpdxContrib,
    PosTag::Holder,
    PosTag::Is,
    PosTag::Held,
];

const NON_COPYRIGHT_POS_TAGS: &[PosTag] = &[];

/// Returns a tuple of (copyrights, holders, authors).
pub fn detect_copyrights_from_text(
    content: &str,
) -> (
    Vec<CopyrightDetection>,
    Vec<HolderDetection>,
    Vec<AuthorDetection>,
) {
    let mut copyrights = Vec::new();
    let mut holders = Vec::new();
    let mut authors = Vec::new();

    if content.is_empty() {
        return (copyrights, holders, authors);
    }

    let numbered_lines: Vec<(usize, String)> = content
        .lines()
        .enumerate()
        .map(|(i, line)| (i + 1, line.to_string()))
        .collect();

    if numbered_lines.is_empty() {
        return (copyrights, holders, authors);
    }

    let groups = collect_candidate_lines(numbered_lines);

    for group in groups {
        if group.is_empty() {
            continue;
        }

        let tokens = get_tokens(&group);
        if tokens.is_empty() {
            continue;
        }

        let tree = parse(tokens);

        let has_top_level_nodes = tree.iter().any(|n| {
            matches!(
                n.label(),
                Some(TreeLabel::Copyright) | Some(TreeLabel::Copyright2) | Some(TreeLabel::Author)
            )
        });

        if has_top_level_nodes {
            extract_from_tree_nodes(&tree, &mut copyrights, &mut holders, &mut authors);
        } else {
            extract_bare_copyrights(&tree, &mut copyrights, &mut holders);
            extract_from_spans(&tree, &mut copyrights, &mut holders, &mut authors);
            fix_truncated_contributors_authors(&tree, &mut authors);
            extract_orphaned_by_authors(&tree, &mut authors);
        }
        extract_holder_is_name(&tree, &mut copyrights, &mut holders);
    }

    (copyrights, holders, authors)
}

fn extract_from_tree_nodes(
    tree: &[ParseNode],
    copyrights: &mut Vec<CopyrightDetection>,
    holders: &mut Vec<HolderDetection>,
    authors: &mut Vec<AuthorDetection>,
) {
    let group_has_copyright = tree.iter().any(|n| {
        matches!(
            n.label(),
            Some(TreeLabel::Copyright) | Some(TreeLabel::Copyright2)
        )
    });

    let mut i = 0;
    while i < tree.len() {
        let node = &tree[i];
        let label = node.label();

        if matches!(
            label,
            Some(TreeLabel::Copyright) | Some(TreeLabel::Copyright2)
        ) {
            let prefix_token = get_orphaned_copy_prefix(tree, i);
            let (trailing_tokens, skip) = collect_trailing_orphan_tokens(node, tree, i + 1);

            if trailing_tokens.is_empty() {
                let has_holder =
                    build_holder_from_node(node, NON_HOLDER_LABELS, NON_HOLDER_POS_TAGS).is_some()
                        || build_holder_from_node(
                            node,
                            NON_HOLDER_LABELS_MINI,
                            NON_HOLDER_POS_TAGS_MINI,
                        )
                        .is_some();

                if !has_holder
                    && i + 1 < tree.len()
                    && matches!(tree[i + 1], ParseNode::Leaf(ref t) if t.tag == PosTag::Uni)
                    && has_name_tree_within(tree, i + 2, 2)
                {
                    let mut cr_tokens: Vec<&Token> = Vec::new();
                    if let Some(prefix) = prefix_token {
                        cr_tokens.push(prefix);
                    }
                    let node_leaves =
                        collect_filtered_leaves(node, NON_COPYRIGHT_LABELS, NON_COPYRIGHT_POS_TAGS);
                    let node_leaves = strip_all_rights_reserved(node_leaves);
                    cr_tokens.extend(&node_leaves);

                    let mut extra_skip = 0;
                    let mut j = i + 1;
                    while j < tree.len()
                        && !is_orphan_boundary(&tree[j])
                        && is_orphan_continuation(&tree[j])
                    {
                        let leaves = collect_all_leaves(&tree[j]);
                        cr_tokens.extend(leaves);
                        j += 1;
                        extra_skip += 1;
                    }
                    let cr_tokens = strip_all_rights_reserved(cr_tokens);
                    if let Some(det) = build_copyright_from_tokens(&cr_tokens) {
                        copyrights.push(det);
                    }

                    let mut holder_tokens: Vec<&Token> = Vec::new();
                    let node_holder_leaves =
                        collect_filtered_leaves(node, NON_HOLDER_LABELS, NON_HOLDER_POS_TAGS);
                    let node_holder_leaves = strip_all_rights_reserved(node_holder_leaves);
                    holder_tokens.extend(&node_holder_leaves);
                    let mut k = i + 1;
                    while k < j {
                        let leaves = collect_all_leaves(&tree[k]);
                        holder_tokens.extend(leaves);
                        k += 1;
                    }
                    let holder_tokens = strip_all_rights_reserved(holder_tokens);
                    if let Some(det) = build_holder_from_tokens(&holder_tokens) {
                        holders.push(det);
                    }

                    i += extra_skip;
                    i += 1;
                    continue;
                }

                if !has_holder
                    && i + 1 < tree.len()
                    && tree[i + 1].label() == Some(TreeLabel::Author)
                    && let Some((cr_det, h_det, skip)) =
                        merge_copyright_with_following_author(node, prefix_token, tree, i + 1)
                {
                    copyrights.push(cr_det);
                    if let Some(h) = h_det {
                        holders.push(h);
                    }
                    i += skip + 1;
                    i += 1;
                    continue;
                }

                let cr_ok =
                    if let Some(det) = build_copyright_from_node_with_prefix(node, prefix_token) {
                        copyrights.push(det);
                        true
                    } else {
                        false
                    };

                let holder = build_holder_from_node(node, NON_HOLDER_LABELS, NON_HOLDER_POS_TAGS);
                if let Some(det) = holder {
                    holders.push(det);
                } else if let Some(det) =
                    build_holder_from_node(node, NON_HOLDER_LABELS_MINI, NON_HOLDER_POS_TAGS_MINI)
                {
                    holders.push(det);
                }
                if cr_ok && let Some(det) = extract_author_from_copyright_node(node) {
                    authors.push(det);
                }
            } else {
                let mut cr_tokens: Vec<&Token> = Vec::new();
                if let Some(prefix) = prefix_token {
                    cr_tokens.push(prefix);
                }
                let node_leaves =
                    collect_filtered_leaves(node, NON_COPYRIGHT_LABELS, NON_COPYRIGHT_POS_TAGS);
                let node_leaves = strip_all_rights_reserved(node_leaves);
                cr_tokens.extend(&node_leaves);
                cr_tokens.extend(&trailing_tokens);
                let cr_tokens = strip_all_rights_reserved(cr_tokens);
                if let Some(det) = build_copyright_from_tokens(&cr_tokens) {
                    copyrights.push(det);
                }

                let mut holder_tokens: Vec<&Token> = Vec::new();
                let node_holder_leaves =
                    collect_filtered_leaves(node, NON_HOLDER_LABELS, NON_HOLDER_POS_TAGS);
                let node_holder_leaves = strip_all_rights_reserved(node_holder_leaves);
                holder_tokens.extend(&node_holder_leaves);
                holder_tokens.extend(&trailing_tokens);
                let holder_tokens = strip_all_rights_reserved(holder_tokens);
                if let Some(det) = build_holder_from_tokens(&holder_tokens) {
                    holders.push(det);
                } else {
                    let mut holder_tokens_mini: Vec<&Token> = Vec::new();
                    let node_holder_mini = collect_filtered_leaves(
                        node,
                        NON_HOLDER_LABELS_MINI,
                        NON_HOLDER_POS_TAGS_MINI,
                    );
                    let node_holder_mini = strip_all_rights_reserved(node_holder_mini);
                    holder_tokens_mini.extend(&node_holder_mini);
                    holder_tokens_mini.extend(&trailing_tokens);
                    let holder_tokens_mini = strip_all_rights_reserved(holder_tokens_mini);
                    if let Some(det) = build_holder_from_tokens(&holder_tokens_mini) {
                        holders.push(det);
                    }
                }

                i += skip;
            }
        } else if label == Some(TreeLabel::Author) {
            if let Some((det, skip)) = build_author_with_trailing(node, tree, i + 1) {
                authors.push(det);
                i += skip;
            } else if let Some(det) = build_author_from_node(node) {
                authors.push(det);
            }
        } else if let ParseNode::Leaf(token) = node
            && token.tag == PosTag::Copy
            && i + 1 < tree.len()
            && is_orphan_copy_name_match(&tree[i + 1])
        {
            let next = &tree[i + 1];
            let mut cr_tokens: Vec<&Token> = vec![token];
            let name_leaves =
                collect_filtered_leaves(next, NON_COPYRIGHT_LABELS, NON_COPYRIGHT_POS_TAGS);
            let name_leaves = strip_all_rights_reserved(name_leaves);
            cr_tokens.extend(&name_leaves);
            if let Some(det) = build_copyright_from_tokens(&cr_tokens) {
                copyrights.push(det);
            }

            let holder_leaves =
                collect_filtered_leaves(next, NON_HOLDER_LABELS, NON_HOLDER_POS_TAGS);
            let holder_leaves = strip_all_rights_reserved(holder_leaves);
            if let Some(det) = build_holder_from_tokens(&holder_leaves) {
                holders.push(det);
            } else {
                let holder_mini =
                    collect_filtered_leaves(next, NON_HOLDER_LABELS_MINI, NON_HOLDER_POS_TAGS_MINI);
                let holder_mini = strip_all_rights_reserved(holder_mini);
                if let Some(det) = build_holder_from_tokens(&holder_mini) {
                    holders.push(det);
                }
            }
            i += 2;
            continue;
        } else if let Some((det, skip)) = try_extract_orphaned_by_author(tree, i) {
            authors.push(det);
            i += skip;
        } else if let Some((det, skip)) = try_extract_date_by_author(tree, i) {
            authors.push(det);
            i += skip;
        } else if !group_has_copyright
            && let Some((det, skip)) = try_extract_by_name_email_author(tree, i)
        {
            authors.push(det);
            i += skip;
        }
        i += 1;
    }
}

fn merge_copyright_with_following_author<'a>(
    copyright_node: &'a ParseNode,
    prefix_token: Option<&'a Token>,
    tree: &'a [ParseNode],
    author_idx: usize,
) -> Option<(CopyrightDetection, Option<HolderDetection>, usize)> {
    let author_node = &tree[author_idx];
    if author_node.label() != Some(TreeLabel::Author) {
        return None;
    }

    let author_leaves = collect_all_leaves(author_node);

    let auth_token = author_leaves.iter().find(|t| {
        matches!(
            t.tag,
            PosTag::Auth | PosTag::Auth2 | PosTag::Auths | PosTag::AuthDot
        )
    })?;
    if auth_token.tag != PosTag::Auth {
        return None;
    }

    let has_email = author_leaves.iter().any(|t| t.tag == PosTag::Email);
    if !has_email {
        return None;
    }

    let cr_leaves_all = collect_all_leaves(copyright_node);
    let cr_last_line = cr_leaves_all.last().map(|t| t.start_line).unwrap_or(0);
    let author_first_line = auth_token.start_line;
    if author_first_line != cr_last_line + 1 {
        return None;
    }

    let mut cr_tokens: Vec<&Token> = Vec::new();
    if let Some(prefix) = prefix_token {
        cr_tokens.push(prefix);
    }
    let cr_leaves =
        collect_filtered_leaves(copyright_node, NON_COPYRIGHT_LABELS, NON_COPYRIGHT_POS_TAGS);
    let cr_leaves = strip_all_rights_reserved(cr_leaves);
    cr_tokens.extend(&cr_leaves);

    cr_tokens.extend(author_leaves.iter());

    let cr_det = build_copyright_from_tokens(&cr_tokens)?;

    let holder_tokens: Vec<&Token> = author_leaves
        .iter()
        .copied()
        .filter(|t| !NON_HOLDER_POS_TAGS.contains(&t.tag))
        .collect();
    let holder_tokens = strip_all_rights_reserved(holder_tokens);
    let h_det = build_holder_from_tokens(&holder_tokens);

    Some((cr_det, h_det, 0))
}

fn get_orphaned_copy_prefix(tree: &[ParseNode], idx: usize) -> Option<&Token> {
    if idx == 0 {
        return None;
    }
    if let ParseNode::Leaf(token) = &tree[idx - 1]
        && token.tag == PosTag::Copy
    {
        return Some(token);
    }
    None
}

fn is_orphan_continuation(node: &ParseNode) -> bool {
    match node {
        ParseNode::Leaf(token) => matches!(
            token.tag,
            PosTag::Of
                | PosTag::Van
                | PosTag::Uni
                | PosTag::Nn
                | PosTag::Nnp
                | PosTag::Caps
                | PosTag::Cc
                | PosTag::Cd
                | PosTag::Cds
                | PosTag::Comp
                | PosTag::Dash
                | PosTag::Pn
                | PosTag::MixedCap
                | PosTag::In
                | PosTag::To
                | PosTag::By
                | PosTag::Email
                | PosTag::Url
                | PosTag::Url2
                | PosTag::Linux
                | PosTag::Parens
        ),
        ParseNode::Tree { label, .. } => matches!(
            label,
            TreeLabel::Name
                | TreeLabel::NameEmail
                | TreeLabel::NameYear
                | TreeLabel::NameCaps
                | TreeLabel::Company
                | TreeLabel::AndCo
                | TreeLabel::DashCaps
        ),
    }
}

fn is_orphan_copy_name_match(node: &ParseNode) -> bool {
    match node.label() {
        Some(TreeLabel::NameYear) | Some(TreeLabel::NameEmail) | Some(TreeLabel::Company) => true,
        Some(TreeLabel::Name | TreeLabel::NameCaps) => {
            let leaves = collect_all_leaves(node);
            leaves
                .iter()
                .any(|t| matches!(t.tag, PosTag::Yr | PosTag::YrPlus | PosTag::BareYr))
        }
        _ => false,
    }
}

fn is_orphan_boundary(node: &ParseNode) -> bool {
    match node {
        ParseNode::Leaf(token) => matches!(
            token.tag,
            PosTag::EmptyLine
                | PosTag::Copy
                | PosTag::Auth
                | PosTag::Auth2
                | PosTag::Auths
                | PosTag::AuthDot
                | PosTag::Maint
                | PosTag::Contributors
                | PosTag::Commit
                | PosTag::SpdxContrib
                | PosTag::Junk
        ),
        ParseNode::Tree { label, .. } => matches!(
            label,
            TreeLabel::Copyright
                | TreeLabel::Copyright2
                | TreeLabel::Author
                | TreeLabel::AllRightReserved
        ),
    }
}

fn should_start_absorbing(copyright_node: &ParseNode, tree: &[ParseNode], start: usize) -> bool {
    if start >= tree.len() {
        return false;
    }
    let first = &tree[start];

    let strong_first = match first {
        ParseNode::Leaf(token) if token.tag == PosTag::Of || token.tag == PosTag::Van => {
            has_name_like_within(tree, start + 1, 2)
        }

        ParseNode::Tree { label, .. } => matches!(
            label,
            TreeLabel::Name
                | TreeLabel::NameEmail
                | TreeLabel::Company
                | TreeLabel::AndCo
                | TreeLabel::DashCaps
        ),
        _ => false,
    };

    if strong_first {
        return true;
    }

    if last_leaf_ends_with_comma(copyright_node) {
        let is_name_like_first = match first {
            ParseNode::Leaf(token) => matches!(
                token.tag,
                PosTag::Nnp | PosTag::Caps | PosTag::Comp | PosTag::Uni | PosTag::MixedCap
            ),
            _ => false,
        };
        if is_name_like_first {
            return has_company_signal_nearby(tree, start);
        }
    }

    let is_name_like_first = match first {
        ParseNode::Leaf(token) => matches!(
            token.tag,
            PosTag::Nnp | PosTag::Caps | PosTag::Cd | PosTag::Cds | PosTag::Comp | PosTag::MixedCap
        ),
        _ => false,
    };
    if is_name_like_first {
        return has_company_signal_nearby(tree, start);
    }

    false
}

fn has_name_tree_within(tree: &[ParseNode], start: usize, lookahead: usize) -> bool {
    let end = std::cmp::min(start + lookahead, tree.len());
    for node in &tree[start..end] {
        if let ParseNode::Tree { label, .. } = node
            && matches!(
                label,
                TreeLabel::Name | TreeLabel::Company | TreeLabel::NameEmail
            )
        {
            return true;
        }
    }
    false
}

fn has_name_like_within(tree: &[ParseNode], start: usize, lookahead: usize) -> bool {
    let end = std::cmp::min(start + lookahead, tree.len());
    for node in &tree[start..end] {
        match node {
            ParseNode::Leaf(token) => {
                if matches!(
                    token.tag,
                    PosTag::Uni | PosTag::Nnp | PosTag::Caps | PosTag::Comp
                ) {
                    return true;
                }
            }
            ParseNode::Tree { label, .. } => {
                if matches!(
                    label,
                    TreeLabel::Name | TreeLabel::Company | TreeLabel::NameEmail
                ) {
                    return true;
                }
            }
        }
    }
    false
}

fn has_company_signal_nearby(tree: &[ParseNode], start: usize) -> bool {
    let end = std::cmp::min(start + 3, tree.len());
    for node in &tree[start..end] {
        match node {
            ParseNode::Leaf(token) => {
                if matches!(token.tag, PosTag::Comp) {
                    return true;
                }
            }
            ParseNode::Tree { label, .. } => {
                if matches!(label, TreeLabel::Company) {
                    return true;
                }
            }
        }
    }
    false
}

fn last_leaf_ends_with_comma(node: &ParseNode) -> bool {
    let leaves = collect_all_leaves(node);
    leaves.last().is_some_and(|t| t.value.ends_with(','))
}

fn collect_trailing_orphan_tokens<'a>(
    copyright_node: &'a ParseNode,
    tree: &'a [ParseNode],
    start: usize,
) -> (Vec<&'a Token>, usize) {
    if !should_start_absorbing(copyright_node, tree, start) {
        return (Vec::new(), 0);
    }

    let mut tokens: Vec<&Token> = Vec::new();
    let mut j = start;

    while j < tree.len() {
        let node = &tree[j];

        if is_orphan_boundary(node) {
            break;
        }

        if !is_orphan_continuation(node) {
            break;
        }

        let leaves = collect_all_leaves(node);
        tokens.extend(leaves);
        j += 1;
    }

    let skip = j - start;
    (tokens, skip)
}

const AUTHOR_BY_KEYWORDS: &[&str] = &["originally", "contributed"];

fn is_line_initial_keyword(tree: &[ParseNode], idx: usize, keyword_line: usize) -> bool {
    if idx == 0 {
        return true;
    }
    let prev = &tree[idx - 1];
    match prev {
        ParseNode::Tree { label, .. } => {
            if matches!(
                label,
                TreeLabel::Copyright | TreeLabel::Copyright2 | TreeLabel::Author
            ) {
                return true;
            }
            let leaves = collect_all_leaves(prev);
            leaves.last().is_none_or(|t| t.start_line != keyword_line)
        }
        ParseNode::Leaf(token) => token.start_line != keyword_line,
    }
}

fn try_extract_orphaned_by_author(
    tree: &[ParseNode],
    idx: usize,
) -> Option<(AuthorDetection, usize)> {
    let node = &tree[idx];
    let (keyword, keyword_line) = match node {
        ParseNode::Leaf(token)
            if matches!(token.tag, PosTag::Junk | PosTag::Nn | PosTag::Auth2) =>
        {
            (token.value.to_lowercase(), token.start_line)
        }
        _ => return None,
    };

    if !AUTHOR_BY_KEYWORDS.contains(&keyword.as_str()) {
        return None;
    }

    if idx > 0 && !is_line_initial_keyword(tree, idx, keyword_line) {
        return None;
    }

    let by_idx = idx + 1;
    if by_idx >= tree.len() {
        return None;
    }
    match &tree[by_idx] {
        ParseNode::Leaf(token) if token.tag == PosTag::By => {}
        _ => return None,
    }

    let name_idx = by_idx + 1;
    if name_idx >= tree.len() {
        return None;
    }

    let mut author_tokens: Vec<&Token> = Vec::new();
    let mut consumed = name_idx - idx;

    let mut j = name_idx;
    while j < tree.len() {
        match &tree[j] {
            ParseNode::Tree {
                label:
                    TreeLabel::Name | TreeLabel::NameEmail | TreeLabel::NameYear | TreeLabel::Company,
                ..
            } => {
                let leaves = collect_filtered_leaves(
                    &tree[j],
                    &[TreeLabel::YrRange, TreeLabel::YrAnd],
                    NON_AUTHOR_POS_TAGS,
                );
                author_tokens.extend(leaves);
                consumed = j - idx;
                j += 1;
            }
            ParseNode::Leaf(token)
                if matches!(
                    token.tag,
                    PosTag::Nnp | PosTag::Nn | PosTag::Email | PosTag::Url
                ) =>
            {
                author_tokens.push(token);
                consumed = j - idx;
                j += 1;
            }
            _ => break,
        }
    }

    if author_tokens.is_empty() {
        return None;
    }

    let det = build_author_from_tokens(&author_tokens)?;
    Some((det, consumed))
}

fn try_extract_date_by_author(tree: &[ParseNode], idx: usize) -> Option<(AuthorDetection, usize)> {
    let node = &tree[idx];
    match node {
        ParseNode::Leaf(token) if token.tag == PosTag::By => {}
        _ => return None,
    }

    if idx == 0 {
        return None;
    }
    let prev_is_date = match &tree[idx - 1] {
        ParseNode::Leaf(token) => matches!(token.tag, PosTag::Yr | PosTag::BareYr),
        ParseNode::Tree { label, .. } => matches!(label, TreeLabel::YrRange | TreeLabel::YrAnd),
    };
    if !prev_is_date {
        return None;
    }

    let name_idx = idx + 1;
    if name_idx >= tree.len() {
        return None;
    }

    let mut author_tokens: Vec<&Token> = Vec::new();
    let mut consumed = name_idx - idx;

    let mut j = name_idx;
    while j < tree.len() {
        match &tree[j] {
            ParseNode::Tree {
                label:
                    TreeLabel::Name | TreeLabel::NameEmail | TreeLabel::NameYear | TreeLabel::Company,
                ..
            } => {
                let leaves = collect_filtered_leaves(
                    &tree[j],
                    &[TreeLabel::YrRange, TreeLabel::YrAnd],
                    NON_AUTHOR_POS_TAGS,
                );
                author_tokens.extend(leaves);
                consumed = j - idx;
                j += 1;
            }
            ParseNode::Leaf(token)
                if matches!(
                    token.tag,
                    PosTag::Nnp | PosTag::Nn | PosTag::Email | PosTag::Url
                ) =>
            {
                author_tokens.push(token);
                consumed = j - idx;
                j += 1;
            }
            _ => break,
        }
    }

    if author_tokens.is_empty() {
        return None;
    }

    let det = build_author_from_tokens(&author_tokens)?;
    Some((det, consumed))
}

fn try_extract_by_name_email_author(
    tree: &[ParseNode],
    idx: usize,
) -> Option<(AuthorDetection, usize)> {
    let by_token = match &tree[idx] {
        ParseNode::Leaf(token) if token.tag == PosTag::By => token,
        _ => return None,
    };

    let by_line = by_token.start_line;

    // Require at least 2 preceding tokens on the same line as "by".
    // This allows "for Linux by Erik" but blocks "Debianized by Norbert"
    // where a single verb before "by" indicates a contextual phrase.
    let mut same_line_preceding = 0;
    for j in (0..idx).rev() {
        let leaves = collect_all_leaves(&tree[j]);
        for leaf in &leaves {
            if leaf.start_line == by_line {
                same_line_preceding += 1;
            }
        }
    }
    if same_line_preceding < 2 {
        return None;
    }

    let name_idx = idx + 1;
    if name_idx >= tree.len() {
        return None;
    }

    let name_node = &tree[name_idx];
    match name_node.label() {
        Some(
            TreeLabel::NameYear | TreeLabel::NameEmail | TreeLabel::Name | TreeLabel::NameCaps,
        ) => {}
        _ => return None,
    }

    let all_leaves = collect_all_leaves(name_node);
    let has_email = all_leaves.iter().any(|t| t.tag == PosTag::Email);
    if !has_email {
        return None;
    }

    let author_tokens: Vec<&Token> = collect_filtered_leaves(
        name_node,
        &[TreeLabel::YrRange, TreeLabel::YrAnd],
        NON_AUTHOR_POS_TAGS,
    );

    let det = build_author_from_tokens(&author_tokens)?;
    Some((det, 1))
}

fn build_author_with_trailing(
    node: &ParseNode,
    tree: &[ParseNode],
    start: usize,
) -> Option<(AuthorDetection, usize)> {
    if start >= tree.len() {
        return None;
    }
    match &tree[start] {
        ParseNode::Leaf(token) if matches!(token.tag, PosTag::Email | PosTag::Url) => {}
        _ => return None,
    }

    let all_leaves = collect_all_leaves(node);
    let last_leaf = all_leaves.last()?;
    let last_is_email_with_comma =
        matches!(last_leaf.tag, PosTag::Email | PosTag::Url) && last_leaf.value.ends_with(',');
    if !last_is_email_with_comma {
        return None;
    }

    let mut author_tokens: Vec<&Token> = collect_filtered_leaves(
        node,
        &[TreeLabel::YrRange, TreeLabel::YrAnd],
        NON_AUTHOR_POS_TAGS,
    );

    let mut j = start;
    while j < tree.len() {
        match &tree[j] {
            ParseNode::Leaf(token)
                if matches!(token.tag, PosTag::Email | PosTag::Url | PosTag::Cc) =>
            {
                if !NON_AUTHOR_POS_TAGS.contains(&token.tag) {
                    author_tokens.push(token);
                }
                j += 1;
            }
            _ => break,
        }
    }

    let skip = j - start;
    if skip == 0 {
        return None;
    }
    let det = build_author_from_tokens(&author_tokens)?;
    Some((det, skip))
}

fn extract_author_from_copyright_node(node: &ParseNode) -> Option<AuthorDetection> {
    let all_leaves = collect_all_leaves(node);
    if all_leaves.len() < 2 {
        return None;
    }

    let auth_idx = all_leaves.iter().position(|t| {
        matches!(
            t.tag,
            PosTag::Auth | PosTag::Auth2 | PosTag::Auths | PosTag::AuthDot
        )
    })?;

    // Only extract if the auth token is on a DIFFERENT line than the preceding
    // token — prevents "OProfile authors" from being extracted as an author.
    if auth_idx > 0 && all_leaves[auth_idx].start_line == all_leaves[auth_idx - 1].start_line {
        return None;
    }

    let auth_line = all_leaves[auth_idx].start_line;
    let after_auth = &all_leaves[auth_idx + 1..];

    let has_name_on_same_line = after_auth.iter().any(|t| {
        t.start_line == auth_line
            && !NON_AUTHOR_POS_TAGS.contains(&t.tag)
            && !matches!(t.tag, PosTag::Email | PosTag::Url)
    });
    if !has_name_on_same_line {
        return None;
    }

    let has_email = after_auth.iter().any(|t| t.tag == PosTag::Email);
    if !has_email {
        return None;
    }

    let author_tokens: Vec<&Token> = after_auth
        .iter()
        .copied()
        .filter(|t| !NON_AUTHOR_POS_TAGS.contains(&t.tag))
        .collect();

    build_author_from_tokens(&author_tokens)
}

fn extract_orphaned_by_authors(tree: &[ParseNode], authors: &mut Vec<AuthorDetection>) {
    let mut i = 0;
    while i < tree.len() {
        if let Some((det, skip)) = try_extract_orphaned_by_author(tree, i) {
            authors.push(det);
            i += skip;
        } else if let Some((det, skip)) = try_extract_date_by_author(tree, i) {
            authors.push(det);
            i += skip;
        }
        i += 1;
    }
}

fn fix_truncated_contributors_authors(tree: &[ParseNode], authors: &mut Vec<AuthorDetection>) {
    let all_leaves: Vec<&Token> = tree.iter().flat_map(collect_all_leaves).collect();

    // Fix existing authors truncated before "contributors"
    for author in authors.iter_mut() {
        if !author.author.ends_with("and its") && !author.author.ends_with("and her") {
            continue;
        }
        let author_line = author.end_line;
        let has_trailing_contributors = all_leaves.iter().any(|t| {
            t.tag == PosTag::Contributors
                && t.start_line == author_line
                && t.value.to_ascii_lowercase().starts_with("contributor")
        });
        if has_trailing_contributors {
            author.author.push_str(" contributors");
        }
    }

    // Detect "developed/written by ... contributors" pattern directly from tokens.
    // extract_from_spans fails on this when the span extends too far past
    // "contributors" into non-author text.
    let mut i = 0;
    while i < all_leaves.len() {
        let token = all_leaves[i];
        if token.tag == PosTag::Auth2 && i + 1 < all_leaves.len() {
            let next = all_leaves[i + 1];
            if next.tag == PosTag::By {
                let name_start = i + 2;
                let mut end = name_start;
                let mut found_contributors = false;
                while end < all_leaves.len() {
                    let t = all_leaves[end];
                    if t.tag == PosTag::Contributors {
                        found_contributors = true;
                        end += 1;
                        break;
                    }
                    if matches!(
                        t.tag,
                        PosTag::EmptyLine
                            | PosTag::Junk
                            | PosTag::Copy
                            | PosTag::Auth
                            | PosTag::Auth2
                            | PosTag::Auths
                            | PosTag::Maint
                    ) {
                        break;
                    }
                    end += 1;
                }
                if found_contributors && end > name_start {
                    let name_tokens: Vec<&Token> = all_leaves[name_start..end]
                        .iter()
                        .copied()
                        .filter(|t| !NON_AUTHOR_POS_TAGS.contains(&t.tag))
                        .collect();
                    if !name_tokens.is_empty() {
                        let name_str = normalize_whitespace(&tokens_to_string(&name_tokens));
                        let refined = refine_author(&name_str);
                        if let Some(mut author_text) = refined {
                            if !author_text.ends_with("contributors") {
                                author_text.push_str(" contributors");
                            }
                            let already_detected = authors.iter().any(|a| a.author == author_text);
                            if !already_detected && !is_junk_copyright(&author_text) {
                                authors.push(AuthorDetection {
                                    author: author_text,
                                    start_line: all_leaves[name_start].start_line,
                                    end_line: all_leaves[end - 1].start_line,
                                });
                            }
                        }
                    }
                    i = end;
                    continue;
                }
            }
        }
        i += 1;
    }
}

fn extract_holder_is_name(
    tree: &[ParseNode],
    copyrights: &mut Vec<CopyrightDetection>,
    holders: &mut Vec<HolderDetection>,
) {
    let mut i = 0;
    while i < tree.len() {
        if let ParseNode::Leaf(token) = &tree[i]
            && token.tag == PosTag::Holder
            && i + 2 < tree.len()
            && let ParseNode::Leaf(is_token) = &tree[i + 1]
            && is_token.tag == PosTag::Is
            && matches!(
                tree[i + 2].label(),
                Some(TreeLabel::Name)
                    | Some(TreeLabel::NameEmail)
                    | Some(TreeLabel::NameYear)
                    | Some(TreeLabel::NameCaps)
                    | Some(TreeLabel::Company)
            )
        {
            let name_leaves =
                collect_filtered_leaves(&tree[i + 2], NON_COPYRIGHT_LABELS, NON_COPYRIGHT_POS_TAGS);
            let name_leaves_stripped = strip_all_rights_reserved(name_leaves);
            let mut cr_tokens: Vec<&Token> = vec![token, is_token];
            cr_tokens.extend(&name_leaves_stripped);
            if let Some(det) = build_copyright_from_tokens(&cr_tokens) {
                copyrights.push(det);
            }

            let holder_leaves =
                collect_filtered_leaves(&tree[i + 2], NON_HOLDER_LABELS, NON_HOLDER_POS_TAGS);
            let holder_leaves = strip_all_rights_reserved(holder_leaves);
            if let Some(det) = build_holder_from_tokens(&holder_leaves) {
                holders.push(det);
            }
            i += 3;
            continue;
        }
        i += 1;
    }
}

fn build_copyright_from_node_with_prefix(
    node: &ParseNode,
    prefix: Option<&Token>,
) -> Option<CopyrightDetection> {
    let leaves = collect_filtered_leaves(node, NON_COPYRIGHT_LABELS, NON_COPYRIGHT_POS_TAGS);
    let filtered = strip_all_rights_reserved(leaves);
    let mut all_tokens: Vec<&Token> = Vec::new();
    if let Some(prefix_token) = prefix {
        all_tokens.push(prefix_token);
    }
    all_tokens.extend(filtered);
    build_copyright_from_tokens(&all_tokens)
}

/// Handle "bare copyright" pattern: a Copy leaf followed by a NameYear/Name/Company
/// tree without a wrapping Copyright tree.
/// Also handles "Portions/Parts (c) ..." by including a preceding Portions token.
fn extract_bare_copyrights(
    tree: &[ParseNode],
    copyrights: &mut Vec<CopyrightDetection>,
    holders: &mut Vec<HolderDetection>,
) {
    let mut i = 0;
    while i < tree.len() {
        if let ParseNode::Leaf(token) = &tree[i]
            && token.tag == PosTag::Copy
            && i + 1 < tree.len()
        {
            let next = &tree[i + 1];
            if matches!(
                next.label(),
                Some(TreeLabel::NameYear)
                    | Some(TreeLabel::Name)
                    | Some(TreeLabel::NameEmail)
                    | Some(TreeLabel::NameCaps)
                    | Some(TreeLabel::Company)
            ) {
                let portions_prefix = if i > 0
                    && let ParseNode::Leaf(prev) = &tree[i - 1]
                    && prev.tag == PosTag::Portions
                {
                    Some(prev)
                } else {
                    None
                };

                let mut cr_tokens: Vec<&Token> = Vec::new();
                if let Some(prefix) = portions_prefix {
                    cr_tokens.push(prefix);
                }
                cr_tokens.push(token);
                let name_leaves =
                    collect_filtered_leaves(next, NON_COPYRIGHT_LABELS, NON_COPYRIGHT_POS_TAGS);
                let name_leaves = strip_all_rights_reserved(name_leaves);
                cr_tokens.extend(&name_leaves);
                if let Some(det) = build_copyright_from_tokens(&cr_tokens) {
                    copyrights.push(det);
                }

                let holder_leaves =
                    collect_filtered_leaves(next, NON_HOLDER_LABELS, NON_HOLDER_POS_TAGS);
                let holder_leaves = strip_all_rights_reserved(holder_leaves);
                if let Some(det) = build_holder_from_tokens(&holder_leaves) {
                    holders.push(det);
                } else {
                    let holder_mini = collect_filtered_leaves(
                        next,
                        NON_HOLDER_LABELS_MINI,
                        NON_HOLDER_POS_TAGS_MINI,
                    );
                    let holder_mini = strip_all_rights_reserved(holder_mini);
                    if let Some(det) = build_holder_from_tokens(&holder_mini) {
                        holders.push(det);
                    }
                }
                i += 2;
                continue;
            }
        }
        i += 1;
    }
}

fn extract_from_spans(
    tree: &[ParseNode],
    copyrights: &mut Vec<CopyrightDetection>,
    holders: &mut Vec<HolderDetection>,
    authors: &mut Vec<AuthorDetection>,
) {
    let all_leaves: Vec<&Token> = tree.iter().flat_map(collect_all_leaves).collect();

    if all_leaves.is_empty() {
        return;
    }

    let mut i = 0;
    while i < all_leaves.len() {
        let token = all_leaves[i];

        if token.tag == PosTag::Copy || token.tag == PosTag::SpdxContrib {
            // Skip Copy tokens preceded by Portions — already handled by
            // extract_bare_copyrights with the prefix included.
            if token.tag == PosTag::Copy && i > 0 && all_leaves[i - 1].tag == PosTag::Portions {
                i += 1;
                continue;
            }
            let start = i;
            i += 1;
            while i < all_leaves.len() && is_copyright_span_token(all_leaves[i]) {
                if all_leaves[i].tag == PosTag::Copy && i > start + 1 {
                    break;
                }
                i += 1;
            }

            let span = &all_leaves[start..i];
            if span.len() > 1 {
                let filtered = strip_all_rights_reserved_slice(span);
                if let Some(det) = build_copyright_from_tokens(&filtered) {
                    copyrights.push(det);
                }

                let holder_tokens: Vec<&Token> = span
                    .iter()
                    .copied()
                    .filter(|t| !NON_HOLDER_POS_TAGS.contains(&t.tag))
                    .collect();
                if let Some(det) = build_holder_from_tokens(&holder_tokens) {
                    holders.push(det);
                } else {
                    let holder_tokens_mini: Vec<&Token> = span
                        .iter()
                        .copied()
                        .filter(|t| !NON_HOLDER_POS_TAGS_MINI.contains(&t.tag))
                        .collect();
                    if let Some(det) = build_holder_from_tokens(&holder_tokens_mini) {
                        holders.push(det);
                    }
                }
            }
        } else if matches!(
            token.tag,
            PosTag::Auth
                | PosTag::Auth2
                | PosTag::Auths
                | PosTag::AuthDot
                | PosTag::Maint
                | PosTag::Contributors
                | PosTag::Commit
                | PosTag::SpdxContrib
        ) {
            let start = i;
            i += 1;
            while i < all_leaves.len() && is_author_span_token(all_leaves[i]) {
                i += 1;
            }

            let span = &all_leaves[start..i];
            if span.len() > 1 {
                let author_tokens: Vec<&Token> = span
                    .iter()
                    .copied()
                    .filter(|t| !NON_AUTHOR_POS_TAGS.contains(&t.tag))
                    .collect();
                if let Some(det) = build_author_from_tokens(&author_tokens) {
                    authors.push(det);
                }
            }
        } else {
            i += 1;
        }
    }
}

fn is_copyright_span_token(token: &Token) -> bool {
    !matches!(token.tag, PosTag::EmptyLine | PosTag::Junk)
}

fn is_author_span_token(token: &Token) -> bool {
    !matches!(
        token.tag,
        PosTag::EmptyLine | PosTag::Junk | PosTag::Copy | PosTag::SpdxContrib
    )
}

fn collect_all_leaves(node: &ParseNode) -> Vec<&Token> {
    let mut result = Vec::new();
    collect_all_leaves_inner(node, &mut result);
    result
}

fn collect_all_leaves_inner<'a>(node: &'a ParseNode, result: &mut Vec<&'a Token>) {
    match node {
        ParseNode::Leaf(token) => result.push(token),
        ParseNode::Tree { children, .. } => {
            for child in children {
                collect_all_leaves_inner(child, result);
            }
        }
    }
}

// ─── Detection builders from tree nodes ──────────────────────────────────────

fn build_holder_from_node(
    node: &ParseNode,
    ignored_labels: &[TreeLabel],
    ignored_pos_tags: &[PosTag],
) -> Option<HolderDetection> {
    let leaves = collect_filtered_leaves(node, ignored_labels, ignored_pos_tags);
    let filtered = strip_all_rights_reserved(leaves);
    build_holder_from_tokens(&filtered)
}

fn build_author_from_node(node: &ParseNode) -> Option<AuthorDetection> {
    let leaves = collect_filtered_leaves(
        node,
        &[TreeLabel::YrRange, TreeLabel::YrAnd],
        NON_AUTHOR_POS_TAGS,
    );
    build_author_from_tokens(&leaves)
}

// ─── Detection builders from token slices ────────────────────────────────────

fn build_copyright_from_tokens(tokens: &[&Token]) -> Option<CopyrightDetection> {
    if tokens.is_empty() {
        return None;
    }
    let node_string = normalize_whitespace(&tokens_to_string(tokens));
    let refined = refine_copyright(&node_string)?;
    if is_junk_copyright(&refined) {
        return None;
    }
    Some(CopyrightDetection {
        copyright: refined,
        start_line: tokens.first().map(|t| t.start_line).unwrap_or(0),
        end_line: tokens.last().map(|t| t.start_line).unwrap_or(0),
    })
}

fn build_holder_from_tokens(tokens: &[&Token]) -> Option<HolderDetection> {
    if tokens.is_empty() {
        return None;
    }
    let node_string = normalize_whitespace(&tokens_to_string(tokens));
    let refined = refine_holder(&node_string)?;
    if is_junk_copyright(&refined) {
        return None;
    }
    Some(HolderDetection {
        holder: refined,
        start_line: tokens.first().map(|t| t.start_line).unwrap_or(0),
        end_line: tokens.last().map(|t| t.start_line).unwrap_or(0),
    })
}

fn build_author_from_tokens(tokens: &[&Token]) -> Option<AuthorDetection> {
    if tokens.is_empty() {
        return None;
    }
    let node_string = normalize_whitespace(&tokens_to_string(tokens));
    let refined = refine_author(&node_string)?;
    if is_junk_copyright(&refined) {
        return None;
    }
    Some(AuthorDetection {
        author: refined,
        start_line: tokens.first().map(|t| t.start_line).unwrap_or(0),
        end_line: tokens.last().map(|t| t.start_line).unwrap_or(0),
    })
}

// ─── Shared helpers ──────────────────────────────────────────────────────────

fn collect_filtered_leaves<'a>(
    node: &'a ParseNode,
    ignored_labels: &[TreeLabel],
    ignored_pos_tags: &[PosTag],
) -> Vec<&'a Token> {
    let mut result = Vec::new();
    collect_filtered_leaves_inner(node, ignored_labels, ignored_pos_tags, &mut result);
    result
}

fn collect_filtered_leaves_inner<'a>(
    node: &'a ParseNode,
    ignored_labels: &[TreeLabel],
    ignored_pos_tags: &[PosTag],
    result: &mut Vec<&'a Token>,
) {
    match node {
        ParseNode::Leaf(token) => {
            if !ignored_pos_tags.contains(&token.tag) {
                result.push(token);
            }
        }
        ParseNode::Tree { label, children } => {
            if ignored_labels.contains(label) {
                return;
            }
            for child in children {
                collect_filtered_leaves_inner(child, ignored_labels, ignored_pos_tags, result);
            }
        }
    }
}

fn strip_all_rights_reserved(leaves: Vec<&Token>) -> Vec<&Token> {
    strip_all_rights_reserved_slice(&leaves)
}

fn strip_all_rights_reserved_slice<'a>(leaves: &[&'a Token]) -> Vec<&'a Token> {
    let mut filtered: Vec<&Token> = Vec::with_capacity(leaves.len());

    for &token in leaves {
        if token.tag == PosTag::Reserved {
            if filtered.len() >= 2
                && filtered[filtered.len() - 1].tag == PosTag::Right
                && matches!(
                    filtered[filtered.len() - 2].tag,
                    PosTag::Nn | PosTag::Caps | PosTag::Nnp
                )
            {
                filtered.truncate(filtered.len() - 2);
            } else if filtered.len() >= 3
                && matches!(
                    filtered[filtered.len() - 1].tag,
                    PosTag::Nn | PosTag::Caps | PosTag::Nnp
                )
                && filtered[filtered.len() - 2].tag == PosTag::Right
                && matches!(
                    filtered[filtered.len() - 3].tag,
                    PosTag::Nn | PosTag::Caps | PosTag::Nnp
                )
            {
                filtered.truncate(filtered.len() - 3);
            }
        } else {
            filtered.push(token);
        }
    }

    filtered
}

fn tokens_to_string(tokens: &[&Token]) -> String {
    tokens
        .iter()
        .map(|t| t.value.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── End-to-end pipeline tests ────────────────────────────────────

    #[test]
    fn test_detect_copyright_with_email() {
        let (c, h, _a) = detect_copyrights_from_text(
            "Copyright (c) 2009 Masayuki Hatta (mhatta) <mhatta@debian.org>",
        );
        assert_eq!(c.len(), 1, "Should detect one copyright, got: {:?}", c);
        assert_eq!(
            c[0].copyright,
            "Copyright (c) 2009 Masayuki Hatta (mhatta) <mhatta@debian.org>"
        );
        assert_eq!(h.len(), 1, "Should detect one holder, got: {:?}", h);
        assert_eq!(h[0].holder, "Masayuki Hatta");
    }

    #[test]
    fn test_detect_empty_input() {
        let (c, h, a) = detect_copyrights_from_text("");
        assert!(c.is_empty());
        assert!(h.is_empty());
        assert!(a.is_empty());
    }

    #[test]
    fn test_detect_no_copyright() {
        let (c, h, a) = detect_copyrights_from_text("This is just some random code.");
        assert!(c.is_empty());
        assert!(h.is_empty());
        assert!(a.is_empty());
    }

    #[test]
    fn test_detect_simple_copyright() {
        let (c, h, _a) = detect_copyrights_from_text("Copyright 2024 Acme Inc.");
        assert!(!c.is_empty(), "Should detect copyright");
        assert!(
            c[0].copyright.contains("Copyright"),
            "Copyright text: {}",
            c[0].copyright
        );
        assert!(
            c[0].copyright.contains("2024"),
            "Should contain year: {}",
            c[0].copyright
        );
        assert_eq!(c[0].start_line, 1);
        assert!(!h.is_empty(), "Should detect holder");
    }

    #[test]
    fn test_detect_copyright_c_symbol() {
        let (c, h, _a) = detect_copyrights_from_text("Copyright (c) 2020-2024 Foo Bar");
        assert!(!c.is_empty(), "Should detect copyright with (c)");
        assert_eq!(c[0].copyright, "Copyright (c) 2020-2024 Foo Bar");
        assert!(!h.is_empty(), "Should detect holder");
    }

    #[test]
    fn test_detect_copyright_c_symbol_with_all_rights_reserved() {
        let (c, _, _) = detect_copyrights_from_text(
            "Copyright (c) 1999-2002 Zend Technologies Ltd. All rights reserved.",
        );
        assert_eq!(
            c[0].copyright,
            "Copyright (c) 1999-2002 Zend Technologies Ltd."
        );
    }

    #[test]
    fn test_detect_copyright_unicode_symbol() {
        let (c, _, _) = detect_copyrights_from_text(
            "/* Copyright \u{00A9} 2000 ACME, Inc., All Rights Reserved */",
        );
        assert!(!c.is_empty(), "Should detect copyright with \u{00A9}");
        assert!(
            c[0].copyright.starts_with("Copyright"),
            "Should start with Copyright, got: {}",
            c[0].copyright
        );
    }

    #[test]
    fn test_detect_copyright_c_no_all_rights() {
        let (c, _, _) = detect_copyrights_from_text("Copyright (c) 2009 Google");
        assert!(!c.is_empty());
        assert_eq!(c[0].copyright, "Copyright (c) 2009 Google");
    }

    #[test]
    fn test_detect_copyright_c_multiline() {
        let input =
            "Copyright (c) 2001 by the TTF2PT1 project\nCopyright (c) 2001 by Sergey Babkin";
        let (c, _, _) = detect_copyrights_from_text(input);
        assert_eq!(c.len(), 2, "Should detect two copyrights, got: {:?}", c);
        assert_eq!(c[0].copyright, "Copyright (c) 2001 by the TTF2PT1 project");
        assert_eq!(c[1].copyright, "Copyright (c) 2001 by Sergey Babkin");
    }

    #[test]
    fn test_detect_multiline_copyright() {
        let text = "Copyright 2024\n  Acme Corporation\n  All rights reserved.";
        let (c, _h, _a) = detect_copyrights_from_text(text);
        assert!(!c.is_empty(), "Should detect multiline copyright");
    }

    #[test]
    fn test_detect_author() {
        let (c, h, a) = detect_copyrights_from_text("Written by John Doe");
        // "Written" is tagged Auth2, triggering author span extraction.
        assert!(c.is_empty(), "Should not detect copyright");
        assert!(h.is_empty(), "Should not detect holder");
        assert_eq!(a.len(), 1, "Should detect one author, got: {:?}", a);
        assert_eq!(a[0].author, "John Doe");
        assert_eq!(a[0].start_line, 1);
        assert_eq!(a[0].end_line, 1);
    }

    #[test]
    fn test_detect_junk_filtered() {
        let (c, _h, _a) = detect_copyrights_from_text("Copyright (c)");
        // "Copyright (c)" alone is junk.
        assert!(
            c.is_empty(),
            "Bare 'Copyright (c)' should be filtered as junk"
        );
    }

    #[test]
    fn test_detect_multiple_copyrights() {
        let text = "Copyright 2020 Foo Inc.\n\n\n\nCopyright 2024 Bar Corp.";
        let (c, h, _a) = detect_copyrights_from_text(text);
        assert!(
            c.len() >= 2,
            "Should detect two copyrights, got {}: {:?}",
            c.len(),
            c
        );
        assert!(
            h.len() >= 2,
            "Should detect two holders, got {}: {:?}",
            h.len(),
            h
        );
    }

    #[test]
    fn test_detect_spdx_copyright() {
        let (c, _h, _a) = detect_copyrights_from_text("SPDX-FileCopyrightText: 2024 Example Corp");
        assert!(!c.is_empty(), "Should detect SPDX copyright");
        // The refiner normalizes SPDX-FileCopyrightText to Copyright.
        assert!(
            c[0].copyright.contains("Copyright"),
            "Should normalize to Copyright: {}",
            c[0].copyright
        );
    }

    #[test]
    fn test_detect_line_numbers() {
        let text = "Some header\nCopyright 2024 Acme Inc.\nSome footer";
        let (c, _h, _a) = detect_copyrights_from_text(text);
        assert!(!c.is_empty(), "Should detect copyright");
        assert_eq!(c[0].start_line, 2, "Copyright should be on line 2");
    }

    #[test]
    fn test_detect_copyright_year_range() {
        let (c, h, _a) = detect_copyrights_from_text("Copyright 2020-2024 Foo Corp.");
        assert_eq!(c.len(), 1, "Should detect one copyright, got: {:?}", c);
        assert_eq!(c[0].copyright, "Copyright 2020-2024 Foo Corp.");
        assert_eq!(c[0].start_line, 1);
        assert_eq!(c[0].end_line, 1);
        assert_eq!(h.len(), 1, "Should detect one holder, got: {:?}", h);
        assert_eq!(h[0].holder, "Foo Corp.");
        assert_eq!(h[0].start_line, 1);
    }

    #[test]
    fn test_detect_copyright_unicode_holder() {
        let (c, h, _a) = detect_copyrights_from_text("Copyright 2024 François Müller");
        assert!(!c.is_empty(), "Should detect copyright, got: {:?}", c);
        assert!(
            c[0].copyright.contains("François Müller"),
            "Copyright should preserve Unicode names: {}",
            c[0].copyright
        );
        assert!(!h.is_empty(), "Should detect Unicode holder: {:?}", h);
        assert!(
            h[0].holder.contains("Müller") || h[0].holder.contains("François"),
            "Holder should preserve original Unicode name: {}",
            h[0].holder
        );
    }

    #[test]
    fn test_detect_copyright_and_author_same_text() {
        // Adjacent lines are grouped into one candidate, so the author
        // span gets absorbed into the copyright group. Separating them
        // with blank lines produces independent candidate groups.
        let text = "Copyright 2024 Acme Inc.\n\n\n\nWritten by Jane Smith";
        let (c, h, a) = detect_copyrights_from_text(text);
        assert_eq!(c.len(), 1, "Should detect one copyright, got: {:?}", c);
        assert_eq!(c[0].copyright, "Copyright 2024 Acme Inc.");
        assert_eq!(c[0].start_line, 1);
        assert_eq!(h.len(), 1, "Should detect one holder, got: {:?}", h);
        assert_eq!(h[0].holder, "Acme Inc.");
        assert_eq!(a.len(), 1, "Should detect one author, got: {:?}", a);
        assert_eq!(a[0].author, "Jane Smith");
        assert_eq!(a[0].start_line, 5);
    }

    #[test]
    fn test_detect_author_written_by() {
        let (_c, _h, a) = detect_copyrights_from_text("Written by Jane Smith");
        assert_eq!(a.len(), 1, "Should detect one author, got: {:?}", a);
        assert_eq!(a[0].author, "Jane Smith");
        assert_eq!(a[0].start_line, 1);
        assert_eq!(a[0].end_line, 1);
    }

    #[test]
    fn test_detect_author_maintained_by() {
        let (_c, _h, a) = detect_copyrights_from_text("Maintained by Bob Jones");
        assert_eq!(a.len(), 1, "Should detect one author, got: {:?}", a);
        assert_eq!(a[0].author, "Bob Jones");
        assert_eq!(a[0].start_line, 1);
        assert_eq!(a[0].end_line, 1);
    }

    #[test]
    fn test_detect_author_authors_keyword() {
        let (_c, _h, a) = detect_copyrights_from_text("Authors John Smith");
        assert_eq!(
            a.len(),
            1,
            "Should detect author from 'Authors', got: {:?}",
            a
        );
        assert!(
            a[0].author.contains("John Smith"),
            "Author: {}",
            a[0].author
        );
    }

    #[test]
    fn test_detect_author_contributors_keyword() {
        let (_c, _h, a) = detect_copyrights_from_text("Contributors Jane Doe");
        assert_eq!(
            a.len(),
            1,
            "Should detect author from 'Contributors', got: {:?}",
            a
        );
        assert!(a[0].author.contains("Jane Doe"), "Author: {}", a[0].author);
    }

    #[test]
    fn test_detect_author_spdx_contributor() {
        let (_c, _h, a) = detect_copyrights_from_text("SPDX-FileContributor: Alice Johnson");
        assert_eq!(
            a.len(),
            1,
            "Should detect author from SPDX-FileContributor, got: {:?}",
            a
        );
        assert!(
            a[0].author.contains("Alice Johnson"),
            "Author: {}",
            a[0].author
        );
    }

    #[test]
    fn test_detect_copyright_with_company() {
        let (c, h, _a) = detect_copyrights_from_text("Copyright (c) 2024 Google LLC");
        assert_eq!(c.len(), 1, "Should detect one copyright, got: {:?}", c);
        assert_eq!(c[0].copyright, "Copyright (c) 2024 Google LLC");
        assert_eq!(c[0].start_line, 1);
        assert_eq!(h.len(), 1, "Should detect one holder, got: {:?}", h);
        assert_eq!(h[0].holder, "Google LLC");
        assert_eq!(h[0].start_line, 1);
    }

    #[test]
    fn test_detect_copyright_all_rights_reserved() {
        let (c, h, _a) =
            detect_copyrights_from_text("Copyright 2024 Apple Inc. All rights reserved.");
        assert_eq!(c.len(), 1, "Should detect one copyright, got: {:?}", c);
        assert_eq!(
            c[0].copyright, "Copyright 2024 Apple Inc.",
            "All rights reserved should be stripped from copyright text"
        );
        assert_eq!(c[0].start_line, 1);
        assert_eq!(h.len(), 1, "Should detect one holder, got: {:?}", h);
        assert_eq!(h[0].holder, "Apple Inc.");
        assert_eq!(h[0].start_line, 1);
    }

    // ── strip_all_rights_reserved ────────────────────────────────────

    #[test]
    fn test_strip_all_rights_reserved_basic() {
        let tokens = [
            Token {
                value: "Copyright".to_string(),
                tag: PosTag::Copy,
                start_line: 1,
            },
            Token {
                value: "2024".to_string(),
                tag: PosTag::Yr,
                start_line: 1,
            },
            Token {
                value: "Acme".to_string(),
                tag: PosTag::Nnp,
                start_line: 1,
            },
            Token {
                value: "All".to_string(),
                tag: PosTag::Nn,
                start_line: 1,
            },
            Token {
                value: "Rights".to_string(),
                tag: PosTag::Right,
                start_line: 1,
            },
            Token {
                value: "Reserved".to_string(),
                tag: PosTag::Reserved,
                start_line: 1,
            },
        ];
        let refs: Vec<&Token> = tokens.iter().collect();
        let result = strip_all_rights_reserved(refs);
        assert_eq!(result.len(), 3, "Should strip All Rights Reserved");
        assert_eq!(result[0].value, "Copyright");
        assert_eq!(result[1].value, "2024");
        assert_eq!(result[2].value, "Acme");
    }

    // ── collect_filtered_leaves ──────────────────────────────────────

    #[test]
    fn test_collect_filtered_leaves_filters_pos_tags() {
        let node = ParseNode::Tree {
            label: TreeLabel::Copyright,
            children: vec![
                ParseNode::Leaf(Token {
                    value: "Copyright".to_string(),
                    tag: PosTag::Copy,
                    start_line: 1,
                }),
                ParseNode::Leaf(Token {
                    value: "2024".to_string(),
                    tag: PosTag::Yr,
                    start_line: 1,
                }),
                ParseNode::Leaf(Token {
                    value: "Acme".to_string(),
                    tag: PosTag::Nnp,
                    start_line: 1,
                }),
            ],
        };
        // Filter out Copy and Yr tags.
        let leaves = collect_filtered_leaves(&node, &[], &[PosTag::Copy, PosTag::Yr]);
        assert_eq!(leaves.len(), 1);
        assert_eq!(leaves[0].value, "Acme");
    }

    #[test]
    fn test_collect_filtered_leaves_filters_tree_labels() {
        let node = ParseNode::Tree {
            label: TreeLabel::Copyright,
            children: vec![
                ParseNode::Leaf(Token {
                    value: "Copyright".to_string(),
                    tag: PosTag::Copy,
                    start_line: 1,
                }),
                ParseNode::Tree {
                    label: TreeLabel::YrRange,
                    children: vec![ParseNode::Leaf(Token {
                        value: "2024".to_string(),
                        tag: PosTag::Yr,
                        start_line: 1,
                    })],
                },
                ParseNode::Leaf(Token {
                    value: "Acme".to_string(),
                    tag: PosTag::Nnp,
                    start_line: 1,
                }),
            ],
        };
        // Filter out YrRange tree label.
        let leaves = collect_filtered_leaves(&node, &[TreeLabel::YrRange], &[]);
        assert_eq!(leaves.len(), 2);
        assert_eq!(leaves[0].value, "Copyright");
        assert_eq!(leaves[1].value, "Acme");
    }

    #[test]
    fn test_detect_copyright_url_trailing_slash() {
        let input = "Copyright (c) 2007 Free Software Foundation, Inc. http://fsf.org/";
        let (c, h, _a) = detect_copyrights_from_text(input);
        assert_eq!(c.len(), 1, "Should detect one copyright, got: {:?}", c);
        assert_eq!(
            c[0].copyright, "Copyright (c) 2007 Free Software Foundation, Inc. http://fsf.org",
            "Should strip trailing URL slash"
        );
        assert_eq!(h.len(), 1, "Should detect one holder, got: {:?}", h);
        assert_eq!(h[0].holder, "Free Software Foundation, Inc.");
    }

    #[test]
    fn test_detect_copyright_url_angle_brackets_trailing_slash() {
        let input = "Copyright \u{00A9} 2007 Free Software Foundation, Inc. <http://fsf.org/>";
        let (c, _h, _a) = detect_copyrights_from_text(input);
        assert_eq!(c.len(), 1, "Should detect one copyright, got: {:?}", c);
        assert_eq!(
            c[0].copyright, "Copyright (c) 2007 Free Software Foundation, Inc. http://fsf.org",
            "Should strip angle brackets and trailing URL slash"
        );
    }

    #[test]
    fn test_detect_copyright_url_slash_full_file() {
        let content =
            std::fs::read_to_string("testdata/copyright-golden/copyrights/afferogplv3-AfferoGPLv")
                .unwrap();
        let (c, _h, _a) = detect_copyrights_from_text(&content);
        assert!(!c.is_empty(), "Should detect copyright");
        assert!(
            c.iter().any(|cr| cr.copyright
                == "Copyright (c) 2007 Free Software Foundation, Inc. http://fsf.org"),
            "Should strip trailing URL slash"
        );
    }

    #[test]
    fn test_contributed_by_with_latin1_diacritics() {
        let content = std::fs::read("testdata/copyright-golden/authors/strverscmp.c").unwrap();
        let text = crate::utils::file::decode_bytes_to_string(&content);
        let (_c, _h, a) = detect_copyrights_from_text(&text);
        assert!(
            a.iter()
                .any(|a| a.author.contains("Jean-Fran\u{00e7}ois Bignolles")),
            "Should detect author with preserved diacritics, got: {:?}",
            a
        );
    }

    #[test]
    fn test_contributed_by_with_utf8_diacritics() {
        let content = std::fs::read("testdata/copyright-golden/authors/strverscmp2.c").unwrap();
        let text = crate::utils::file::decode_bytes_to_string(&content);
        let (_c, _h, a) = detect_copyrights_from_text(&text);
        assert!(
            a.iter()
                .any(|a| a.author.contains("Jean-Fran\u{00e7}ois Bignolles")),
            "Should detect author with preserved diacritics, got: {:?}",
            a
        );
    }

    #[test]
    fn test_date_by_author() {
        let content = "\
Copyright (c) 1998 Softweyr LLC.  All rights reserved.
strtok_r, from Berkeley strtok
Oct 13, 1998 by Wes Peters <wes@softweyr.com>";
        let (_c, _h, a) = detect_copyrights_from_text(content);
        assert!(
            a.iter().any(|a| a.author.contains("Wes Peters")),
            "Should detect Wes Peters as author, got: {:?}",
            a
        );
    }

    #[test]
    fn test_oprofile_authors_copyright() {
        let content = " * @remark Copyright 2002 OProfile authors
 * @remark Read the file COPYING
 *
 * @Modifications Daniel Hansel
 * Modified by Aravind Menon for Xen
 * These modifications are:
 * Copyright (C) 2005 Hewlett-Packard Co.";
        let (c, h, _a) = detect_copyrights_from_text(content);
        assert!(
            c.iter()
                .any(|cr| cr.copyright == "Copyright 2002 OProfile authors"),
            "Should detect 'Copyright 2002 OProfile authors', got: {:?}",
            c
        );
        assert!(
            h.iter().any(|h| h.holder == "OProfile authors"),
            "Should detect 'OProfile authors' holder, got: {:?}",
            h
        );
    }

    #[test]
    fn test_originally_by_author() {
        let content = "\
#   Copyright 1996-2006 Free Software Foundation, Inc.
#   Taken from GNU libtool, 2001
#   Originally by Gordon Matzigkeit <gord@gnu.ai.mit.edu>, 1996";
        let (_c, _h, a) = detect_copyrights_from_text(content);
        assert!(
            a.iter().any(|a| a.author.contains("Gordon Matzigkeit")),
            "Should detect Gordon Matzigkeit as author, got: {:?}",
            a
        );
    }

    #[test]
    fn test_by_name_email_author_full_file() {
        let content = std::fs::read_to_string(
            "testdata/copyright-golden/authors/author_var_route_c-var_route_c.c",
        )
        .unwrap();
        let (_c, _h, a) = detect_copyrights_from_text(&content);
        assert!(
            a.iter()
                .any(|a| a.author.contains("Jennifer Bray of Origin")),
            "Should detect Jennifer Bray, got: {:?}",
            a
        );
        assert!(
            a.iter().any(|a| a.author.contains("Erik Schoenfelder")),
            "Should detect Erik Schoenfelder, got: {:?}",
            a
        );
        assert!(
            a.iter().any(|a| a.author.contains("Simon Leinen")),
            "Should detect Simon Leinen, got: {:?}",
            a
        );
    }

    #[test]
    fn test_author_uc_contributors() {
        let content =
            std::fs::read_to_string("testdata/copyright-golden/authors/author_uc-LICENSE").unwrap();
        let (_c, _h, a) = detect_copyrights_from_text(&content);
        assert!(
            a.iter()
                .any(|a| a.author == "UC Berkeley and its contributors"),
            "Should detect 'UC Berkeley and its contributors', got: {:?}",
            a
        );
        assert!(
            a.iter().any(|a| a
                .author
                .contains("University of California, Berkeley and its contributors")),
            "Should detect 'University of California, Berkeley and its contributors', got: {:?}",
            a
        );
    }

    #[test]
    fn test_multiline_two_copyrights_adjacent_lines() {
        let input = "\tCopyright 1988, 1989 by Carnegie Mellon University\n\tCopyright 1989\tTGV, Incorporated\n";
        let (c, h, _a) = detect_copyrights_from_text(input);
        assert!(
            c.iter().any(|cr| cr.copyright.contains("Carnegie Mellon")),
            "Should detect CMU copyright"
        );
        assert!(
            c.iter().any(|cr| cr.copyright.contains("TGV")),
            "Should detect TGV copyright, got: {:?}",
            c
        );
        assert!(
            h.iter().any(|hr| hr.holder.contains("TGV")),
            "Should detect TGV holder, got: {:?}",
            h
        );
    }

    #[test]
    fn test_multiline_copyright_after_created_line() {
        let input = "// Created: Sun Feb  9 10:06:01 2003 by faith@dict.org\n// Copyright 2003, 2004 Rickard E. Faith (faith@dict.org)\n";
        let (c, h, _a) = detect_copyrights_from_text(input);
        assert!(
            c.iter().any(|cr| cr.copyright.contains("Rickard")),
            "Should detect Faith copyright, got: {:?}",
            c
        );
        assert!(
            h.iter().any(|hr| hr.holder.contains("Faith")),
            "Should detect Faith holder, got: {:?}",
            h
        );
    }

    #[test]
    fn test_co_maintainer_no_false_author() {
        let content = std::fs::read_to_string(
            "testdata/copyright-golden/copyrights/misco4/co-maintainer.txt",
        )
        .unwrap();
        let (_c, _h, a) = detect_copyrights_from_text(&content);
        assert!(
            !a.iter().any(|a| a.author.contains("Norbert Tretkowski")),
            "Should NOT detect Norbert Tretkowski (Debianized by), got: {:?}",
            a
        );
    }

    #[test]
    fn test_auth_nl_copyright_not_author() {
        // When "Copyright (C) YEAR" is followed by "Author: Name <email>" on the next line,
        // the Author name should be absorbed into the copyright, not treated as a standalone author.
        let input =
            "* Copyright (C) 2016-2018\n* Author: Matt Ranostay <matt.ranostay@konsulko.com>";
        let (c, h, a) = detect_copyrights_from_text(input);
        assert!(
            c.iter().any(|cr| cr.copyright.contains("Matt Ranostay")),
            "Should detect copyright with Matt Ranostay, got: {:?}",
            c
        );
        assert!(
            h.iter().any(|hr| hr.holder.contains("Matt Ranostay")),
            "Should detect Matt Ranostay as holder, got: {:?}",
            h
        );
        // The expected output has NO author entries
        assert!(
            a.is_empty(),
            "Should NOT detect authors (Author: is part of copyright), got: {:?}",
            a
        );
    }

    #[test]
    fn test_notice_file_multiple_copyrights() {
        let text = "   Copyright (C) 1997, 2002, 2005 Free Software Foundation, Inc.\n\
                    * Copyright (C) 2005 Jens Axboe <axboe@suse.de>\n\
                    * Copyright (C) 2006 Alan D. Brunelle <Alan.Brunelle@hp.com>\n\
                    * Copyright (C) 2006 Jens Axboe <axboe@kernel.dk>\n\
                    * Copyright (C) 2006. Bob Jenkins (bob_jenkins@burtleburtle.net)\n\
                    * Copyright (C) 2009 Jozsef Kadlecsik (kadlec@blackhole.kfki.hu)\n\
                    * Copyright IBM Corp. 2008\n\
                    # Copyright (c) 2005 SUSE LINUX Products GmbH, Nuernberg, Germany.\n\
                    # Copyright (c) 2005 Silicon Graphics, Inc.";
        let (c, _h, _a) = detect_copyrights_from_text(text);
        let cr_texts: Vec<&str> = c.iter().map(|cr| cr.copyright.as_str()).collect();
        assert!(
            c.len() >= 9,
            "Should detect at least 9 copyrights, got {}: {:?}",
            c.len(),
            cr_texts
        );
    }

    #[test]
    fn test_doc_doc_no_overabsorb() {
        let input = "are copyrighted by Douglas C. Schmidt and his research group at Washington University, University of California, Irvine, and Vanderbilt University, Copyright (c) 1993-2008, all rights reserved.";
        let (c, _h, _a) = detect_copyrights_from_text(input);
        assert!(
            c.iter().any(|cr| cr.copyright == "copyrighted by Douglas C. Schmidt and his research group at Washington University"),
            "Should stop at Washington University, got: {:?}", c
        );
    }

    #[test]
    fn test_academy_copyright() {
        let input = "Copyright (c) 2006 Academy of Motion Picture Arts and Sciences";
        let (c, h, _a) = detect_copyrights_from_text(input);
        assert!(
            c.iter().any(|cr| cr.copyright
                == "Copyright (c) 2006 Academy of Motion Picture Arts and Sciences"),
            "Should detect Academy copyright, got: {:?}",
            c
        );
        assert!(
            h.iter()
                .any(|hr| hr.holder == "Academy of Motion Picture Arts and Sciences"),
            "Should detect Academy holder, got: {:?}",
            h
        );
    }

    #[test]
    fn test_define_copyright() {
        let input = "#define COPYRIGHT       \"Copyright (c) 1999-2008 LSI Corporation\"\n#define MODULEAUTHOR    \"LSI Corporation\"";
        let (c, h, a) = detect_copyrights_from_text(input);
        assert!(
            c.iter()
                .any(|cr| cr.copyright == "(c) 1999-2008 LSI Corporation"),
            "Should detect '(c) 1999-2008 LSI Corporation', got: {:?}",
            c
        );
        assert!(
            h.iter().any(|h| h.holder == "LSI Corporation"),
            "Should detect holder, got: {:?}",
            h
        );
        assert!(
            a.iter().any(|a| a.author == "LSI Corporation"),
            "Should detect author from MODULEAUTHOR, got: {:?}",
            a
        );
    }

    #[test]
    fn test_parts_copyright_prefix() {
        let input = " * Parts (C) 1999 David Airlie, airlied@linux.ie";
        let (c, h, _a) = detect_copyrights_from_text(input);
        assert_eq!(c.len(), 1, "Should detect one copyright, got: {:?}", c);
        assert_eq!(
            c[0].copyright,
            "Parts (c) 1999 David Airlie, airlied@linux.ie"
        );
        assert_eq!(h.len(), 1, "Should detect one holder, got: {:?}", h);
        assert_eq!(h[0].holder, "David Airlie");
    }
}
