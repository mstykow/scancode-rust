//! Parser for Microsoft Update Manifest (.mum) files.
//!
//! Extracts Windows Update package metadata from .mum XML manifest files.
//!
//! # Supported Formats
//! - `*.mum` - Microsoft Update Manifest XML files
//!
//! # Implementation Notes
//! - Format: XML with assembly and package metadata
//! - Spec: Windows Update manifests

use crate::models::DatasourceId;
use std::fs;
use std::path::Path;

use log::warn;
use quick_xml::events::Event;
use quick_xml::reader::Reader;

use crate::models::PackageData;

use super::PackageParser;

const PACKAGE_TYPE: &str = "windows-update";

pub struct MicrosoftUpdateManifestParser;

impl PackageParser for MicrosoftUpdateManifestParser {
    const PACKAGE_TYPE: &'static str = PACKAGE_TYPE;

    fn is_match(path: &Path) -> bool {
        path.extension().is_some_and(|ext| ext == "mum")
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read .mum file {:?}: {}", path, e);
                return vec![PackageData {
                    package_type: Some(PACKAGE_TYPE.to_string()),
                    datasource_id: Some(DatasourceId::MicrosoftUpdateManifestMum),
                    ..Default::default()
                }];
            }
        };

        vec![parse_mum_xml(&content)]
    }
}

pub(crate) fn parse_mum_xml(content: &str) -> PackageData {
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);

    let mut name = None;
    let mut version = None;
    let mut description = None;
    let mut copyright = None;
    let mut homepage_url = None;

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e)) => {
                if e.name().as_ref() == b"assemblyIdentity" {
                    for attr in e.attributes().filter_map(|a| a.ok()) {
                        match attr.key.as_ref() {
                            b"name" => name = String::from_utf8(attr.value.to_vec()).ok(),
                            b"version" => version = String::from_utf8(attr.value.to_vec()).ok(),
                            _ => {}
                        }
                    }
                }
            }
            Ok(Event::Start(e)) => {
                if e.name().as_ref() == b"assembly" {
                    for attr in e.attributes().filter_map(|a| a.ok()) {
                        match attr.key.as_ref() {
                            b"description" => {
                                description = String::from_utf8(attr.value.to_vec()).ok()
                            }
                            b"copyright" => copyright = String::from_utf8(attr.value.to_vec()).ok(),
                            b"supportInformation" => {
                                homepage_url = String::from_utf8(attr.value.to_vec()).ok()
                            }
                            _ => {}
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                warn!(
                    "Error parsing XML at position {}: {}",
                    reader.buffer_position(),
                    e
                );
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    PackageData {
        package_type: Some(PACKAGE_TYPE.to_string()),
        name,
        version,
        description,
        homepage_url,
        copyright,
        datasource_id: Some(DatasourceId::MicrosoftUpdateManifestMum),
        ..Default::default()
    }
}

crate::register_parser!(
    "Microsoft Update Manifest .mum file",
    &["*.mum"],
    "windows-update",
    "",
    None,
);
