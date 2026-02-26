use crate::copyright::line_tracking::{LineNumberIndex, PreparedLineCache};
use crate::copyright::types::{CopyrightDetection, HolderDetection};

pub(super) fn run_phase_primary_extractions(
    content: &str,
    raw_lines: &[&str],
    groups: &[Vec<(usize, String)>],
    line_number_index: &LineNumberIndex,
    prepared_cache: &mut PreparedLineCache<'_>,
    copyrights: &mut Vec<CopyrightDetection>,
    holders: &mut Vec<HolderDetection>,
) {
    super::extract_midline_c_year_holder_with_leading_acronym_from_raw_lines(
        raw_lines, copyrights, holders,
    );
    super::extend_copyrights_with_authors_blocks(raw_lines, copyrights, holders);
    super::extend_year_only_copyrights_with_trailing_text(raw_lines, copyrights, holders);

    super::merge_year_only_copyrights_with_following_author_colon_lines(
        prepared_cache,
        copyrights,
        holders,
    );
    super::extract_licensed_material_of_company_bare_c_year_lines(
        prepared_cache,
        copyrights,
        holders,
    );

    super::drop_shadowed_and_or_holders(holders);
    super::drop_shadowed_prefix_holders(holders);
    super::drop_shadowed_acronym_extended_holders(holders);
    super::drop_shadowed_prefix_copyrights(copyrights);
    super::drop_shadowed_c_sign_variants(copyrights);
    super::drop_shadowed_year_prefixed_holders(holders);

    super::merge_multiline_person_year_copyright_continuations(
        raw_lines,
        prepared_cache,
        copyrights,
        holders,
    );

    super::extract_mso_document_properties_copyrights(content, copyrights, holders);
    super::expand_portions_copyright_variants(copyrights);
    super::expand_year_only_copyrights_with_by_name_prefix(prepared_cache, copyrights, holders);
    super::expand_year_only_copyrights_with_read_the_suffix(prepared_cache, copyrights, holders);
    super::merge_multiline_obfuscated_name_year_copyright_pairs(
        raw_lines,
        prepared_cache,
        copyrights,
        holders,
    );
    super::add_modify_suffix_holders(raw_lines, prepared_cache, holders);
    super::drop_shadowed_prefix_bare_c_copyrights_same_span(copyrights);

    super::apply_javadoc_company_metadata(content, line_number_index, copyrights, holders);
    super::apply_european_community_copyright(content, line_number_index, copyrights, holders);
    super::extract_html_entity_year_range_copyrights(content, line_number_index, copyrights);
    super::extract_copr_lines(groups, copyrights, holders);
    super::extract_standalone_c_holder_year_lines(groups, copyrights, holders);
    super::extract_c_years_then_holder_lines(groups, copyrights, holders);
    super::extract_copyright_c_years_holder_lines(groups, copyrights, holders);
    super::extract_c_holder_without_year_lines(content, groups, copyrights, holders);
    super::extract_three_digit_copyright_year_lines(content, copyrights, holders);
    super::extract_copyrighted_by_lines(content, copyrights, holders);
    super::extract_c_word_year_lines(content, copyrights, holders);
    super::extract_are_c_year_holder_lines(content, copyrights, holders);
    super::extract_bare_c_by_holder_lines(content, copyrights, holders);
    super::extract_trailing_bare_c_year_range_suffixes(groups, copyrights);
    super::extract_common_year_only_lines(groups, copyrights);
    super::extract_embedded_bare_c_year_suffixes(groups, copyrights);
    super::extract_repeated_embedded_bare_c_year_suffixes(groups, copyrights);
    super::extract_lowercase_username_angle_email_copyrights(groups, copyrights, holders);
    super::extract_lowercase_username_paren_email_copyrights(groups, copyrights, holders);
    super::extract_copyright_c_year_comma_name_angle_email_lines(groups, copyrights, holders);
    super::extract_c_year_range_by_name_comma_email_lines(groups, copyrights, holders);
    super::extract_copyright_years_by_name_then_paren_email_next_line(content, copyrights, holders);
    super::extract_copyright_years_by_name_paren_email_lines(groups, copyrights, holders);
    super::extract_copyright_year_name_with_of_lines(groups, copyrights, holders);
    super::extract_line_ending_copyright_then_by_holder(content, copyrights, holders);
    super::extract_changelog_timestamp_copyrights_from_content(content, copyrights, holders);
    super::drop_url_extended_prefix_duplicates(copyrights);

    super::drop_obfuscated_email_year_only_copyrights(content, copyrights, holders);
    super::extract_glide_3dfx_copyright_notice(content, copyrights);
    super::extract_spdx_filecopyrighttext_c_without_year(content, copyrights, holders);
    super::extract_html_meta_name_copyright_content(content, copyrights, holders);
    super::extract_html_anchor_copyright_url(content, line_number_index, copyrights, holders);
    super::extract_angle_bracket_year_name_copyrights(groups, copyrights, holders);
    super::extract_html_icon_class_copyrights(content, line_number_index, copyrights, holders);
    super::extract_added_the_copyright_year_for_lines(content, copyrights, holders);
    super::extract_copyright_by_without_year_lines(groups, copyrights, holders);
    super::extract_copyright_notice_paren_year_lines(groups, copyrights, holders);
    super::extract_copyright_year_c_holder_mid_sentence_lines(groups, copyrights, holders);
    super::extract_javadoc_author_copyright_lines(groups, copyrights, holders);
    super::extract_xml_copyright_tag_c_lines(content, line_number_index, copyrights, holders);
    super::extract_copyright_its_authors_lines(groups, copyrights, holders);
    super::extract_copyright_year_c_name_angle_email_lines(groups, copyrights, holders);
    super::extract_us_government_year_placeholder_copyrights(groups, copyrights, holders);
    super::extract_holder_is_name_paren_email_lines(content, copyrights, holders);
}
