//! Rule loading and orchestration.

pub mod legalese;
pub mod loader;
#[cfg(test)]
mod loader_test;
pub mod thresholds;

pub use loader::*;
