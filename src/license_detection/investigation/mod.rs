//! Investigation tests for license detection issues.
//!
//! These tests are used to investigate and debug specific detection issues
//! by comparing Rust behavior against the Python reference implementation.

#[cfg(test)]
mod gpl_lgpl_complex_test;
#[cfg(test)]
mod gpl_mpl_test;
#[cfg(test)]
mod here_proprietary_test;
#[cfg(test)]
mod ijg_test;
#[cfg(test)]
mod kde_licenses_test;

#[cfg(test)]
mod plantuml_test;
#[cfg(test)]
mod something_html_test;
