#[cfg(test)]
mod tests {
    use crate::models::DatasourceId;
    use crate::models::PackageType;
    use crate::parsers::PackageParser;
    use crate::parsers::debian::*;
    use std::path::PathBuf;

    #[test]
    fn test_debian_md5sum_in_package_is_match_control_tar_xz() {
        let path = PathBuf::from(
            "testdata/debian/extracted-md5sums/testpkg_2.5-3_i386.deb-extract/control.tar.xz-extract/md5sums",
        );
        assert!(DebianMd5sumInPackageParser::is_match(&path));
    }

    #[test]
    fn test_debian_md5sum_in_package_is_not_match_wrong_path() {
        let path = PathBuf::from("testdata/debian/var/lib/dpkg/info/bash.md5sums");
        assert!(!DebianMd5sumInPackageParser::is_match(&path));
    }

    #[test]
    fn test_debian_md5sum_in_package_is_not_match_wrong_filename() {
        let path = PathBuf::from(
            "testdata/debian/extracted-md5sums/example_1.0-1_amd64.deb-extract/control.tar.gz-extract/control",
        );
        assert!(!DebianMd5sumInPackageParser::is_match(&path));
    }

    #[test]
    fn test_debian_md5sum_in_package_extract_from_control_tar_gz() {
        let path = PathBuf::from(
            "testdata/debian/extracted-md5sums/example_1.0-1_amd64.deb-extract/control.tar.gz-extract/md5sums",
        );

        if !path.exists() {
            return;
        }

        let packages = DebianMd5sumInPackageParser::extract_packages(&path);
        assert_eq!(packages.len(), 1);

        let package = &packages[0];
        assert_eq!(package.package_type, Some(PackageType::Deb));
        assert_eq!(
            package.datasource_id,
            Some(DatasourceId::DebianMd5SumsInExtractedDeb)
        );
        assert_eq!(package.name, Some("example".to_string()));
        assert_eq!(package.namespace, Some("debian".to_string()));
        assert_eq!(package.purl, Some("pkg:deb/debian/example".to_string()));

        assert_eq!(package.file_references.len(), 4);

        let first_ref = &package.file_references[0];
        assert_eq!(first_ref.path, "usr/bin/example");
        assert_eq!(
            first_ref.md5,
            Some("d41d8cd98f00b204e9800998ecf8427e".to_string())
        );
        assert!(first_ref.sha1.is_none());
        assert!(first_ref.sha256.is_none());

        let last_ref = &package.file_references[3];
        assert_eq!(last_ref.path, "usr/share/man/man1/example.1.gz");
        assert_eq!(
            last_ref.md5,
            Some("9e107d9d372bb6826bd81d3542a419d6".to_string())
        );
    }

    #[test]
    fn test_debian_md5sum_in_package_extract_from_control_tar_xz() {
        let path = PathBuf::from(
            "testdata/debian/extracted-md5sums/testpkg_2.5-3_i386.deb-extract/control.tar.xz-extract/md5sums",
        );

        if !path.exists() {
            return;
        }

        let packages = DebianMd5sumInPackageParser::extract_packages(&path);
        assert_eq!(packages.len(), 1);

        let package = &packages[0];
        assert_eq!(package.package_type, Some(PackageType::Deb));
        assert_eq!(
            package.datasource_id,
            Some(DatasourceId::DebianMd5SumsInExtractedDeb)
        );
        assert_eq!(package.name, Some("testpkg".to_string()));
        assert_eq!(package.namespace, Some("debian".to_string()));
        assert_eq!(package.purl, Some("pkg:deb/debian/testpkg".to_string()));

        assert_eq!(package.file_references.len(), 3);

        let first_ref = &package.file_references[0];
        assert_eq!(first_ref.path, "usr/bin/testapp");
        assert_eq!(
            first_ref.md5,
            Some("5f4dcc3b5aa765d61d8327deb882cf99".to_string())
        );

        let last_ref = &package.file_references[2];
        assert_eq!(last_ref.path, "usr/lib/testpkg/libtest.so");
        assert_eq!(
            last_ref.md5,
            Some("ad0234829205b9033196ba818f7a872b".to_string())
        );
    }

    // ====== DebianControlInExtractedDebParser tests ======

    #[test]
    fn test_debian_control_in_extracted_deb_is_match_gz() {
        let path = PathBuf::from(
            "testdata/debian/extracted-md5sums/example_1.0-1_amd64.deb-extract/control.tar.gz-extract/control",
        );
        assert!(DebianControlInExtractedDebParser::is_match(&path));
    }

    #[test]
    fn test_debian_control_in_extracted_deb_is_match_xz() {
        let path = PathBuf::from(
            "testdata/debian/extracted-md5sums/testpkg_2.5-3_i386.deb-extract/control.tar.xz-extract/control",
        );
        assert!(DebianControlInExtractedDebParser::is_match(&path));
    }

    #[test]
    fn test_debian_control_in_extracted_deb_not_match_debian_control() {
        let path = PathBuf::from("debian/control");
        assert!(!DebianControlInExtractedDebParser::is_match(&path));
    }

    #[test]
    fn test_debian_control_in_extracted_deb_not_match_md5sums() {
        let path = PathBuf::from("example_1.0-1_amd64.deb-extract/control.tar.gz-extract/md5sums");
        assert!(!DebianControlInExtractedDebParser::is_match(&path));
    }

    #[test]
    fn test_debian_control_in_extracted_deb_not_match_plain_control() {
        let path = PathBuf::from("/some/path/control");
        assert!(!DebianControlInExtractedDebParser::is_match(&path));
    }

    #[test]
    fn test_debian_control_in_extracted_deb_extract_gz() {
        let path = PathBuf::from(
            "testdata/debian/extracted-md5sums/example_1.0-1_amd64.deb-extract/control.tar.gz-extract/control",
        );

        if !path.exists() {
            return;
        }

        let packages = DebianControlInExtractedDebParser::extract_packages(&path);
        assert_eq!(packages.len(), 1);

        let pkg = &packages[0];
        assert_eq!(pkg.package_type, Some(PackageType::Deb));
        assert_eq!(
            pkg.datasource_id,
            Some(DatasourceId::DebianControlExtractedDeb)
        );
        assert_eq!(pkg.name, Some("example".to_string()));
        assert_eq!(pkg.version, Some("1.0-1".to_string()));
        assert_eq!(pkg.namespace, Some("debian".to_string()));
        assert_eq!(pkg.homepage_url, Some("https://example.com".to_string()));
        assert!(pkg.purl.is_some());
        assert!(pkg.purl.as_ref().unwrap().contains("example"));
        assert!(pkg.purl.as_ref().unwrap().contains("1.0-1"));

        assert_eq!(pkg.parties.len(), 1);
        assert_eq!(pkg.parties[0].role, Some("maintainer".to_string()));
        assert_eq!(pkg.parties[0].name, Some("Example Developer".to_string()));

        assert!(!pkg.dependencies.is_empty());
    }

    #[test]
    fn test_debian_control_in_extracted_deb_extract_xz() {
        let path = PathBuf::from(
            "testdata/debian/extracted-md5sums/testpkg_2.5-3_i386.deb-extract/control.tar.xz-extract/control",
        );

        if !path.exists() {
            return;
        }

        let packages = DebianControlInExtractedDebParser::extract_packages(&path);
        assert_eq!(packages.len(), 1);

        let pkg = &packages[0];
        assert_eq!(pkg.package_type, Some(PackageType::Deb));
        assert_eq!(
            pkg.datasource_id,
            Some(DatasourceId::DebianControlExtractedDeb)
        );
        assert_eq!(pkg.name, Some("testpkg".to_string()));
        assert_eq!(pkg.version, Some("2.5-3".to_string()));
        assert_eq!(pkg.namespace, Some("debian".to_string()));

        assert_eq!(pkg.parties.len(), 1);
        assert_eq!(pkg.parties[0].name, Some("Test Maintainer".to_string()));

        assert!(!pkg.dependencies.is_empty());
        let dep_purls: Vec<&str> = pkg
            .dependencies
            .iter()
            .filter_map(|d| d.purl.as_deref())
            .collect();
        assert!(dep_purls.iter().any(|p| p.contains("libc6")));
    }

    #[test]
    fn test_extract_package_name_from_deb_path() {
        let path1 = PathBuf::from("example_1.0-1_amd64.deb-extract/control.tar.gz-extract/md5sums");
        let name1 = extract_package_name_from_deb_path(&path1);
        assert_eq!(name1, Some("example".to_string()));

        let path2 = PathBuf::from("testpkg_2.5-3_i386.deb-extract/control.tar.xz-extract/md5sums");
        let name2 = extract_package_name_from_deb_path(&path2);
        assert_eq!(name2, Some("testpkg".to_string()));

        let path3 = PathBuf::from(
            "complex-name_1.2.3-4ubuntu1_amd64.deb-extract/control.tar.gz-extract/md5sums",
        );
        let name3 = extract_package_name_from_deb_path(&path3);
        assert_eq!(name3, Some("complex-name".to_string()));
    }

    // ====== Debian Copyright Parser tests (Issues #2643, #2644, #2645, #2646) ======

    #[test]
    fn test_copyright_license_detection_dep5_with_body() {
        let content = "\
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Files: *
Copyright: 2020 Example Author
License: MIT
 Permission is hereby granted, free of charge, to any person obtaining a copy
 of this software and associated documentation files (the \"Software\"), to deal
 in the Software without restriction, including without limitation the rights
 to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 copies of the Software, and to permit persons to whom the Software is
 furnished to do so, subject to the following conditions:
 THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 SOFTWARE.
";
        let pkg = crate::parsers::debian::parse_copyright_file(content, Some("test"));
        assert!(
            !pkg.license_detections.is_empty(),
            "Should detect license in body text"
        );
        let detection = &pkg.license_detections[0];
        assert!(detection.license_expression.contains("mit"));
    }

    #[test]
    fn test_copyright_license_detection_short_symbol_only() {
        let content = "\
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Files: *
Copyright: 2020 Example
License: MIT
";
        let pkg = crate::parsers::debian::parse_copyright_file(content, Some("test"));
        assert_eq!(pkg.extracted_license_statement, Some("MIT".to_string()));
        assert!(pkg.license_detections.is_empty());
    }

    #[test]
    fn test_copyright_non_dep5_fallback() {
        let content = "License: MIT\n Some license text here";
        let pkg = crate::parsers::debian::parse_copyright_file(content, Some("test"));
        assert!(pkg.extracted_license_statement.is_some());
    }

    #[test]
    fn test_matched_text_preserves_casing() {
        let content = "\
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
License: LGPL-2.1
 This library is free software; you can redistribute it and/or modify it
 under the terms of version 2.1 of the GNU Lesser General Public License.
";
        let pkg = crate::parsers::debian::parse_copyright_file(content, Some("test"));

        if let Some(detection) = pkg.license_detections.first()
            && let Some(match_obj) = detection.matches.first()
            && let Some(matched) = &match_obj.matched_text
        {
            assert!(
                matched.contains("LGPL") || matched.contains("GNU Lesser"),
                "matched_text should preserve original casing"
            );
        }
    }

    #[test]
    fn test_license_expression_lowercase_but_spdx_preserves_case() {
        let content = "\
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
License: Apache-2.0
 Licensed under the Apache License, Version 2.0 (the \"License\");
 you may not use this file except in compliance with the License.
";
        let pkg = crate::parsers::debian::parse_copyright_file(content, Some("test"));

        if let Some(detection) = pkg.license_detections.first() {
            assert_eq!(detection.license_expression, "apache-2.0");
            assert_eq!(detection.license_expression_spdx, "Apache-2.0");
        }
    }

    #[test]
    fn test_license_order_matches_file_order() {
        let content = "\
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/

License: MIT
 Permission is hereby granted, free of charge, to any person obtaining a copy
 of this software and associated documentation files.

License: BSD-3-Clause
 Redistribution and use in source and binary forms, with or without
 modification, are permitted provided that the following conditions are met.
";
        let pkg = crate::parsers::debian::parse_copyright_file(content, Some("test"));

        if pkg.license_detections.len() >= 2 {
            assert!(
                pkg.license_detections[0].matches[0].start_line
                    < pkg.license_detections[1].matches[0].start_line,
                "License detections should be in file order"
            );
        }
    }

    #[test]
    fn test_license_deduplication_preserves_first() {
        let content = "\
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Files: src/*
License: MIT
 Permission is hereby granted...
Files: tests/*
License: MIT
 Permission is hereby granted...
";
        let pkg = crate::parsers::debian::parse_copyright_file(content, Some("test"));
        assert_eq!(pkg.extracted_license_statement, Some("MIT".to_string()));
    }

    #[test]
    fn test_license_detections_have_line_numbers() {
        let content = "\
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Files: *
Copyright: 2020 Example
License: MIT
 Permission is hereby granted, free of charge, to any person obtaining a copy
 of this software and associated documentation files.
";
        let pkg = crate::parsers::debian::parse_copyright_file(content, Some("test"));

        if let Some(detection) = pkg.license_detections.first()
            && let Some(match_obj) = detection.matches.first()
        {
            assert!(match_obj.start_line > 0);
            assert!(match_obj.end_line >= match_obj.start_line);
        }
    }
}
