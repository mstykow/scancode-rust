use std::io::{self, Write};
use std::path::PathBuf;

use sha1::{Digest, Sha1};

use crate::models::{FileInfo, FileType, Output};

use super::shared::{sorted_files, xml_escape};
use super::{EMPTY_SHA1, OutputWriteConfig, SPDX_DOCUMENT_NOTICE};

pub(crate) fn write_spdx_tag_value(
    output: &Output,
    writer: &mut dyn Write,
    config: &OutputWriteConfig,
) -> io::Result<()> {
    let package_name = primary_package_name(output, config);

    let files = spdx_files(output);
    if files.is_empty() {
        writeln!(writer, "# No results for package '{}'.", package_name)?;
        return Ok(());
    }

    let document_namespace = format!("http://spdx.org/spdxdocs/{}", package_name);
    let package_verification_code = spdx_package_verification_code(&files);

    writeln!(writer, "## Document Information")?;
    writeln!(writer, "SPDXVersion: SPDX-2.2")?;
    writeln!(writer, "DataLicense: CC0-1.0")?;
    writeln!(writer, "SPDXID: SPDXRef-DOCUMENT")?;
    writeln!(writer, "DocumentName: SPDX Document created by Provenant")?;
    writeln!(writer, "DocumentNamespace: {}", document_namespace)?;
    writeln!(
        writer,
        "DocumentComment: <text>{}</text>",
        SPDX_DOCUMENT_NOTICE
    )?;
    writeln!(writer, "## Creation Information")?;
    writeln!(writer, "## Package Information")?;

    writeln!(writer, "PackageName: {}", package_name)?;
    writeln!(writer, "SPDXID: SPDXRef-001")?;
    writeln!(writer, "PackageDownloadLocation: NOASSERTION")?;
    writeln!(writer, "FilesAnalyzed: true")?;
    writeln!(
        writer,
        "PackageVerificationCode: {}",
        package_verification_code
    )?;
    writeln!(writer, "PackageLicenseConcluded: NOASSERTION")?;
    writeln!(writer, "PackageLicenseInfoFromFiles: NONE")?;
    writeln!(writer, "PackageLicenseDeclared: NOASSERTION")?;
    writeln!(writer, "PackageCopyrightText: NONE")?;
    writeln!(writer, "## File Information")?;

    let mut file_index = 1usize;
    for file in files {
        let sha1 = file.sha1.as_deref().unwrap_or(EMPTY_SHA1);
        writeln!(writer, "FileName: ./{}", file.path)?;
        writeln!(writer, "SPDXID: SPDXRef-{}", file_index)?;
        writeln!(writer, "FileChecksum: SHA1: {}", sha1)?;
        writeln!(writer, "LicenseConcluded: NOASSERTION")?;
        let has_license_detections = !file.license_detections.is_empty();
        writeln!(
            writer,
            "LicenseInfoInFile: {}",
            if has_license_detections {
                "NOASSERTION"
            } else {
                "NONE"
            }
        )?;

        if file.copyrights.is_empty() {
            writeln!(writer, "FileCopyrightText: NONE")?;
        } else {
            let text = file
                .copyrights
                .iter()
                .map(|c| c.copyright.clone())
                .collect::<Vec<_>>()
                .join("\\n");
            writeln!(writer, "FileCopyrightText: {}", text)?;
        }

        writeln!(writer)?;
        file_index += 1;
    }

    Ok(())
}

pub(crate) fn write_spdx_rdf_xml(
    output: &Output,
    writer: &mut dyn Write,
    config: &OutputWriteConfig,
) -> io::Result<()> {
    let package_name_raw = primary_package_name(output, config);

    let files = spdx_files(output);
    if files.is_empty() {
        writeln!(
            writer,
            "<!-- No results for package '{}'. -->",
            package_name_raw
        )?;
        return Ok(());
    }

    let package_name = xml_escape(&package_name_raw);
    let package_verification_code = spdx_package_verification_code(&files);
    let created_raw = output
        .headers
        .first()
        .map(|h| h.start_timestamp.as_str())
        .unwrap_or("1970-01-01T00:00:00Z");
    let created = xml_escape(created_raw);

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" xmlns:rdfs=\"http://www.w3.org/2000/01/rdf-schema#\" xmlns:spdx=\"http://spdx.org/rdf/terms#\">\n");

    xml.push_str("  <spdx:Package rdf:about=\"#SPDXRef-001\">\n");
    xml.push_str("    <spdx:filesAnalyzed rdf:datatype=\"http://www.w3.org/2001/XMLSchema#boolean\">true</spdx:filesAnalyzed>\n");
    xml.push_str(
        "    <spdx:downloadLocation rdf:resource=\"http://spdx.org/rdf/terms#noassertion\"/>\n",
    );
    xml.push_str(
        "    <spdx:licenseConcluded rdf:resource=\"http://spdx.org/rdf/terms#noassertion\"/>\n",
    );
    xml.push_str(
        "    <spdx:licenseDeclared rdf:resource=\"http://spdx.org/rdf/terms#noassertion\"/>\n",
    );
    xml.push_str(
        "    <spdx:licenseInfoFromFiles rdf:resource=\"http://spdx.org/rdf/terms#none\"/>\n",
    );
    xml.push_str("    <spdx:packageVerificationCode><spdx:PackageVerificationCode><spdx:packageVerificationCodeValue>");
    xml.push_str(&package_verification_code);
    xml.push_str("</spdx:packageVerificationCodeValue></spdx:PackageVerificationCode></spdx:packageVerificationCode>\n");

    for (idx, file) in files.iter().enumerate() {
        let file_id = idx + 1usize;
        xml.push_str("    <spdx:relationship><spdx:Relationship>");
        xml.push_str("<spdx:relationshipType rdf:resource=\"http://spdx.org/rdf/terms#relationshipType_contains\"/>");
        xml.push_str("<spdx:relatedSpdxElement><spdx:File rdf:about=\"#SPDXRef-");
        xml.push_str(&file_id.to_string());
        xml.push_str("\">");
        xml.push_str(
            "<spdx:licenseConcluded rdf:resource=\"http://spdx.org/rdf/terms#noassertion\"/>",
        );
        xml.push_str("<spdx:licenseInfoInFile rdf:resource=\"http://spdx.org/rdf/terms#none\"/>");
        xml.push_str("<spdx:checksum><spdx:Checksum><spdx:algorithm rdf:resource=\"http://spdx.org/rdf/terms#checksumAlgorithm_sha1\"/>");
        xml.push_str("<spdx:checksumValue>");
        xml.push_str(&xml_escape(file.sha1.as_deref().unwrap_or(EMPTY_SHA1)));
        xml.push_str("</spdx:checksumValue></spdx:Checksum></spdx:checksum>");
        xml.push_str("<spdx:fileName>");
        xml.push_str(&xml_escape(&format!("./{}", file.path)));
        xml.push_str("</spdx:fileName>");
        xml.push_str("<spdx:copyrightText>");
        if file.copyrights.is_empty() {
            xml.push_str("NONE");
        } else {
            xml.push_str(&xml_escape(
                &file
                    .copyrights
                    .iter()
                    .map(|c| c.copyright.clone())
                    .collect::<Vec<_>>()
                    .join("\\n"),
            ));
        }
        xml.push_str("</spdx:copyrightText>");
        xml.push_str(
            "</spdx:File></spdx:relatedSpdxElement></spdx:Relationship></spdx:relationship>\n",
        );
    }

    xml.push_str("    <spdx:copyrightText>NONE</spdx:copyrightText>\n");
    xml.push_str("    <spdx:name>");
    xml.push_str(&package_name);
    xml.push_str("</spdx:name>\n");
    xml.push_str("  </spdx:Package>\n");

    xml.push_str("  <spdx:SpdxDocument rdf:about=\"#SPDXRef-DOCUMENT\">\n");
    xml.push_str("    <spdx:dataLicense rdf:resource=\"http://spdx.org/licenses/CC0-1.0\"/>\n");
    xml.push_str("    <rdfs:comment>");
    xml.push_str(&xml_escape(SPDX_DOCUMENT_NOTICE));
    xml.push_str("</rdfs:comment>\n");
    xml.push_str("    <spdx:name>SPDX Document created by Provenant</spdx:name>\n");
    xml.push_str("    <spdx:specVersion>SPDX-2.2</spdx:specVersion>\n");
    xml.push_str("    <spdx:creationInfo><spdx:CreationInfo><spdx:created>");
    xml.push_str(&created);
    xml.push_str("</spdx:created></spdx:CreationInfo></spdx:creationInfo>\n");
    xml.push_str("  </spdx:SpdxDocument>\n");

    xml.push_str("</rdf:RDF>\n");
    writer.write_all(xml.as_bytes())
}

fn primary_package_name(output: &Output, config: &OutputWriteConfig) -> String {
    if let Some(scanned_path) = &config.scanned_path {
        let path = PathBuf::from(scanned_path);
        if let Some(name) = path.file_name().and_then(|n| n.to_str())
            && !name.is_empty()
        {
            return sanitize_spdx_package_name(name);
        }
    }

    output
        .packages
        .first()
        .and_then(|p| p.name.clone())
        .map(|name| sanitize_spdx_package_name(&name))
        .unwrap_or_else(|| "provenant-analyzed-package".to_string())
}

fn sanitize_spdx_package_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "provenant-analyzed-package".to_string()
    } else {
        out
    }
}

fn spdx_files(output: &Output) -> Vec<&FileInfo> {
    sorted_files(&output.files)
        .into_iter()
        .filter(|f| f.file_type == FileType::File)
        .collect()
}

fn spdx_package_verification_code(files: &[&FileInfo]) -> String {
    let mut file_sha1s = files
        .iter()
        .map(|f| f.sha1.clone().unwrap_or_else(|| EMPTY_SHA1.to_string()))
        .collect::<Vec<_>>();
    file_sha1s.sort_unstable();

    let mut hasher = Sha1::new();
    for sha1 in file_sha1s {
        hasher.update(sha1.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}
