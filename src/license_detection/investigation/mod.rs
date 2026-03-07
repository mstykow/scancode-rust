//! Investigation tests for license detection issues.
//!
//! These tests are used to investigate and debug specific detection issues
//! by comparing Rust behavior against the Python reference implementation.

#[cfg(test)]
mod gpl_mit_regression_test;

#[cfg(test)]
mod gpl_mpl_test;

#[cfg(test)]
mod ietf_regression_test;

#[cfg(test)]
mod plantuml_test;

#[cfg(test)]
mod something_html_test;

#[cfg(test)]
mod unknown_readme_test;

#[cfg(test)]
mod unknown_cigna_test;

#[cfg(test)]
mod unknown_citrix_test;

#[cfg(test)]
mod unknown_qt_commercial_test;

#[cfg(test)]
mod unknown_scea_test;

#[cfg(test)]
mod unknown_ucware_test;

#[cfg(test)]
mod rule_118_test;

#[cfg(test)]
mod ar_er_debug_test;

#[cfg(test)]
mod or_expression_pipeline_test;

#[cfg(test)]
mod duplicate_license_test;

#[cfg(test)]
mod gpl_412_test;

#[cfg(test)]
mod aladdin_md5_test;

#[cfg(test)]
mod gfdl_11_candidate_test;

#[cfg(test)]
mod flex_readme_test;

#[cfg(test)]
mod gfdl_scoring_debug_test;
