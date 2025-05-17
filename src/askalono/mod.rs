#![allow(clippy::match_bool, clippy::useless_format)]
#![allow(unused_imports, dead_code)]

mod license;
mod ngram;
mod preproc;
mod store;
mod strategy;

pub use license::{LicenseType, TextData};
pub use store::{Match, Store};
pub use strategy::{ContainedResult, IdentifiedLicense, ScanMode, ScanResult, ScanStrategy};
