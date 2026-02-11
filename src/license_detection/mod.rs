//! License Detection Engine
//!
//! This module provides license detection capabilities by analyzing text content
//! and matching it against known license patterns.

mod expression;
pub mod index;
mod models;
mod query;
pub mod rules;
mod spans;
mod tokenize;

pub use models::{License, LicenseMatch, Rule};

/// License detection engine - placeholder for Phase 2 implementation
#[derive(Debug, Clone)]
pub struct LicenseDetectionEngine;

impl LicenseDetectionEngine {
    /// Create a new license detection engine
    pub fn new() -> Self {
        Self
    }
}

impl Default for LicenseDetectionEngine {
    fn default() -> Self {
        Self::new()
    }
}
