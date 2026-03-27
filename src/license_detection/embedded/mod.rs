//! Embedded license index loading.
//!
//! This module provides support for loading a build-time generated license
//! index that is embedded in the binary. This eliminates the runtime dependency
//! on the ScanCode rules directory.
//!
//! The embedded artifact is generated during the build process and contains
//! the complete pre-built license index including Aho-Corasick automatons.

pub mod index;
