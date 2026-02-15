//! Golden tests for copyright detection.
//!
//! These tests load YAML expected outputs (copied from the Python ScanCode test
//! suite into `testdata/copyright-golden/`), run our Rust copyright detection on
//! the corresponding input files, and compare the results.
//!
//! The expected output files are owned by this repo so we can adjust them for
//! intentional differences (e.g., unicode name preservation, bug fixes).

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use rayon::prelude::*;
    use serde::Deserialize;

    use super::super::detect_copyrights;

    /// Expected output structure matching Python ScanCode's YAML test format.
    /// Extra YAML fields (holders_summary, copyrights_summary, expected_failures)
    /// are silently ignored by serde's default behavior.
    #[derive(Debug, Deserialize, Default)]
    struct ExpectedOutput {
        what: Option<Vec<String>>,
        copyrights: Option<Vec<String>>,
        holders: Option<Vec<String>>,
        authors: Option<Vec<String>>,
    }

    struct FieldDiff {
        field: String,
        missing: Vec<String>,
        extra: Vec<String>,
    }

    impl FieldDiff {
        fn is_match(&self) -> bool {
            self.missing.is_empty() && self.extra.is_empty()
        }
    }

    impl std::fmt::Display for FieldDiff {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            writeln!(f, "  Field: {}", self.field)?;
            if !self.missing.is_empty() {
                writeln!(f, "    Missing (expected but not found):")?;
                for m in &self.missing {
                    writeln!(f, "      - {:?}", m)?;
                }
            }
            if !self.extra.is_empty() {
                writeln!(f, "    Extra (found but not expected):")?;
                for e in &self.extra {
                    writeln!(f, "      + {:?}", e)?;
                }
            }
            Ok(())
        }
    }

    fn compare_field(field: &str, expected: &[String], actual: &[String]) -> FieldDiff {
        let expected_set: BTreeSet<&str> = expected.iter().map(|s| s.as_str()).collect();
        let actual_set: BTreeSet<&str> = actual.iter().map(|s| s.as_str()).collect();

        let missing: Vec<String> = expected_set
            .difference(&actual_set)
            .map(|s| s.to_string())
            .collect();
        let extra: Vec<String> = actual_set
            .difference(&expected_set)
            .map(|s| s.to_string())
            .collect();

        FieldDiff {
            field: field.to_string(),
            missing,
            extra,
        }
    }

    /// Discover all YAML test files in a directory (recursively).
    fn find_yaml_files(dir: &Path) -> Vec<PathBuf> {
        let mut yamls = Vec::new();
        if !dir.is_dir() {
            return yamls;
        }
        collect_yaml_files_recursive(dir, &mut yamls);
        yamls.sort();
        yamls
    }

    fn collect_yaml_files_recursive(dir: &Path, yamls: &mut Vec<PathBuf>) {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_yaml_files_recursive(&path, yamls);
            } else if path.extension().is_some_and(|ext| ext == "yml") {
                yamls.push(path);
            }
        }
    }

    /// Derive the input file path from a YAML expected output path.
    /// The convention is: input file = YAML path with `.yml` extension removed.
    fn input_path_from_yaml(yaml_path: &Path) -> PathBuf {
        let stem = yaml_path.to_string_lossy();
        let input = stem
            .strip_suffix(".yml")
            .expect("YAML path must end in .yml");
        PathBuf::from(input)
    }

    /// Run golden tests for all YAML files in the given test directory.
    ///
    /// `known_failures` lists relative YAML paths (from `test_dir`) that are
    /// expected to fail. They are reported but do not cause the test to panic.
    fn run_golden_test(test_dir: &str, known_failures: &[&str]) {
        let dir = PathBuf::from(test_dir);
        if !dir.is_dir() {
            eprintln!("Skipping golden test: directory not found: {}", test_dir);
            return;
        }

        let yaml_files = find_yaml_files(&dir);
        if yaml_files.is_empty() {
            eprintln!("Skipping golden test: no YAML files in {}", test_dir);
            return;
        }

        let known_set: BTreeSet<&str> = known_failures.iter().copied().collect();

        // Pre-filter to testable files and count skipped
        let mut test_cases: Vec<(&PathBuf, ExpectedOutput)> = Vec::new();
        let mut skipped = 0usize;

        for yaml_path in &yaml_files {
            let input_path = input_path_from_yaml(yaml_path);
            if !input_path.is_file() {
                skipped += 1;
                continue;
            }

            let yaml_content = match fs::read_to_string(yaml_path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let expected: ExpectedOutput = match serde_yaml::from_str(&yaml_content) {
                Ok(e) => e,
                Err(_) => continue,
            };

            let what_fields: Vec<String> = expected.what.clone().unwrap_or_default();
            let has_check = what_fields.iter().any(|w| w == "copyrights")
                || what_fields.iter().any(|w| w == "holders")
                || what_fields.iter().any(|w| w == "authors");

            if !has_check {
                skipped += 1;
                continue;
            }

            test_cases.push((yaml_path, expected));
        }

        let total = test_cases.len();
        let passed_count = AtomicUsize::new(0);
        let failures: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

        // Run detection in parallel across all test cases
        test_cases.par_iter().for_each(|(yaml_path, expected)| {
            let input_path = input_path_from_yaml(yaml_path);
            let relative_path = yaml_path
                .strip_prefix(&dir)
                .unwrap_or(yaml_path)
                .to_string_lossy()
                .to_string();

            let what_fields: Vec<String> = expected.what.clone().unwrap_or_default();
            let check_copyrights = what_fields.iter().any(|w| w == "copyrights");
            let check_holders = what_fields.iter().any(|w| w == "holders");
            let check_authors = what_fields.iter().any(|w| w == "authors");

            let content = match fs::read(&input_path) {
                Ok(bytes) => crate::utils::file::decode_bytes_to_string(&bytes),
                Err(e) => {
                    failures.lock().unwrap().push((
                        relative_path,
                        format!("Input read error: {:?}\n  Error: {}", input_path, e),
                    ));
                    return;
                }
            };

            // Run detection
            let (copyrights, holders, authors) = detect_copyrights(&content);

            // Extract string values
            let actual_copyrights: Vec<String> =
                copyrights.iter().map(|c| c.copyright.clone()).collect();
            let actual_holders: Vec<String> = holders.iter().map(|h| h.holder.clone()).collect();
            let actual_authors: Vec<String> = authors.iter().map(|a| a.author.clone()).collect();

            // Compare requested fields
            let mut field_diffs: Vec<FieldDiff> = Vec::new();

            if check_copyrights {
                let expected_copyrights = expected.copyrights.as_deref().unwrap_or(&[]);
                let diff = compare_field("copyrights", expected_copyrights, &actual_copyrights);
                if !diff.is_match() {
                    field_diffs.push(diff);
                }
            }

            if check_holders {
                let expected_holders = expected.holders.as_deref().unwrap_or(&[]);
                let diff = compare_field("holders", expected_holders, &actual_holders);
                if !diff.is_match() {
                    field_diffs.push(diff);
                }
            }

            if check_authors {
                let expected_authors = expected.authors.as_deref().unwrap_or(&[]);
                let diff = compare_field("authors", expected_authors, &actual_authors);
                if !diff.is_match() {
                    field_diffs.push(diff);
                }
            }

            if field_diffs.is_empty() {
                passed_count.fetch_add(1, Ordering::Relaxed);
            } else {
                let mut failure_msg = format!("FAIL: {}", relative_path);
                for diff in &field_diffs {
                    failure_msg.push_str(&format!("\n{}", diff));
                }
                failures.lock().unwrap().push((relative_path, failure_msg));
            }
        });

        let passed = passed_count.load(Ordering::Relaxed);
        let mut failures = failures.into_inner().unwrap();
        failures.sort_by(|a, b| a.0.cmp(&b.0));

        let mut expected_failure_paths: Vec<String> = Vec::new();
        let mut unexpected_failure_paths: Vec<String> = Vec::new();
        let mut unexpected_failure_msgs: Vec<String> = Vec::new();

        for (rel_path, msg) in &failures {
            if known_set.contains(rel_path.as_str()) {
                expected_failure_paths.push(rel_path.clone());
            } else {
                unexpected_failure_paths.push(rel_path.clone());
                unexpected_failure_msgs.push(msg.clone());
            }
        }

        let failed_set: BTreeSet<&str> = failures.iter().map(|(p, _)| p.as_str()).collect();
        let mut newly_passing: Vec<&str> = known_failures
            .iter()
            .filter(|kf| !failed_set.contains(**kf))
            .copied()
            .collect();
        newly_passing.sort();

        let pass_rate = if total > 0 {
            (passed as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        let effective_total = total - expected_failure_paths.len();
        let effective_pass_rate = if effective_total > 0 {
            (passed as f64 / effective_total as f64) * 100.0
        } else {
            0.0
        };

        // Print summary report
        eprintln!("\n{}", "=".repeat(60));
        eprintln!("Copyright Golden Test Report");
        eprintln!("{}", "=".repeat(60));
        eprintln!(
            "Total: {} | Passed: {} | Expected Failures: {} | Unexpected Failures: {} | Skipped: {}",
            total,
            passed,
            expected_failure_paths.len(),
            unexpected_failure_paths.len(),
            skipped
        );
        eprintln!(
            "Pass rate: {:.1}% (excluding expected failures: {:.1}%)",
            pass_rate, effective_pass_rate
        );

        if !expected_failure_paths.is_empty() {
            eprintln!(
                "\n--- Expected Failures ({}) ---",
                expected_failure_paths.len()
            );
            for ef in &expected_failure_paths {
                eprintln!("  - {}", ef);
            }
        }

        if newly_passing.is_empty() {
            eprintln!("\n--- Newly Passing (remove from known_failures) ---");
            eprintln!("  (none)");
        } else {
            eprintln!(
                "\n--- Newly Passing (remove from known_failures) ({}) ---",
                newly_passing.len()
            );
            for np in &newly_passing {
                eprintln!("  - {}", np);
            }
        }

        if !unexpected_failure_msgs.is_empty() {
            eprintln!(
                "\n--- Unexpected Failures ({}) ---\n",
                unexpected_failure_paths.len()
            );
            for (i, failure) in unexpected_failure_msgs.iter().enumerate() {
                eprintln!(
                    "[{}/{}] {}\n",
                    i + 1,
                    unexpected_failure_paths.len(),
                    failure
                );
            }
        }

        eprintln!("\n{}\n", "=".repeat(60));

        assert!(
            unexpected_failure_paths.is_empty(),
            "{}/{} golden tests had UNEXPECTED failures ({:.1}% pass rate). \
             See failure details above.",
            unexpected_failure_paths.len(),
            total,
            pass_rate
        );
    }

    #[test]
    fn test_golden_copyrights() {
        run_golden_test(
            "testdata/copyright-golden/copyrights",
            &[
                "debian_multi_names_on_one_line-libgdata.copyright.yml",
                "libc6_i686-libc_i.copyright.yml",
                "misco2/copyrighted-complex.txt.yml",
                "misco2/gost.txt.yml",
                "misco3/complex-multiline-copyright.txt.yml",
                "misco3/intractable-copyright-in-LICENSE.txt.yml",
                "misco3/intractable-copyright-in-LICENSE2.txt.yml",
                "misco4/linux-copyrights/drivers/media/radio/radio-gemtek.c.yml",
                "misco4/linux-copyrights/drivers/media/radio/radio-rtrack2.c.yml",
                "misco4/linux-copyrights/drivers/net/macvlan.c.yml",
                "misco4/linux3/rewrited.txt.yml",
                "misco4/linux7/name-before.txt.yml",
                "misco4/more-linux/following.txt.yml",
                "misco4/more-linux/ilia.txt.yml",
                "name_before_c-c.c.yml",
                "openoffice_org_report_builder_bin.copyright.yml",
                "partial_detection.txt.yml",
                "partial_detection_mit.txt.yml",
            ],
        );
    }

    #[test]
    fn test_golden_holders() {
        run_golden_test("testdata/copyright-golden/holders", &[]);
    }

    #[test]
    fn test_golden_authors() {
        run_golden_test(
            "testdata/copyright-golden/authors",
            &["author_russ_c-c.c.yml"],
        );
    }

    #[test]
    fn test_golden_ics() {
        run_golden_test(
            "testdata/copyright-golden/ics",
            &[
                "apache-xml/NOTICE.yml",
                "blktrace-btt/NOTICE.yml",
                "blktrace/NOTICE.yml",
                "bluetooth-bluez-audio/gateway.c.yml",
                "bluetooth-bluez-test/NOTICE.yml",
                "bluetooth-bluez-tools/NOTICE.yml",
                "bluetooth-bluez/NOTICE.yml",
                "bluetooth-glib-gio-xdgmime/xdgmimealias.h.yml",
                "bluetooth-glib-glib/gconvert.c.yml",
                "bzip2/manual.html.yml",
                "chromium-chrome-browser-resources/about_credits.html.yml",
                "chromium-chrome-common-extensions-docs-examples-apps-hello-python-httplib2/__init__.py.yml",
                "chromium-chrome-common-extensions-docs-examples-extensions-benchmark-jquery/jquery-1.4.2.min.js.yml",
                "chromium-net-base/x509_cert_types_mac_unittest.cc.yml",
                "dbus/acinclude.m4.yml",
                "dhcpcd/dhcpcd.c.yml",
                "freetype-src-autofit/afindic.c.yml",
                "freetype-src-autofit/afindic.h.yml",
                "guava/guava.ipr.yml",
                "hyphenation/README.yml",
                "hyphenation/hyphen.c.yml",
                "hyphenation/hyphen.h.yml",
                "iptables-extensions/libxt_LED.c.yml",
                "iptables-extensions/libxt_time.c.yml",
                "iptables-extensions/libxt_u32.c.yml",
                "iptables-iptables/ip6tables-standalone.c.yml",
                "iptables-iptables/xtables.c.yml",
                "jpeg/configure.yml",
                "kernel-headers-original-linux/ethtool.h.yml",
                "kernel-headers-original-linux/posix_acl.h.yml",
                "kernel-headers-original-linux/spinlock_api_smp.h.yml",
                "libvpx-examples-includes-geshi-docs/geshi-doc.html.yml",
                "libvpx-examples-includes-geshi-docs/geshi-doc.txt.yml",
                "markdown-bin/markdown.yml",
                "mesa3d-docs/license.html.yml",
                "netperf/MODULE_LICENSE_HP.yml",
                "netperf/netperf.c.yml",
                "netperf/netserver.c.yml",
                "opencv-cv-src/cvsmooth.cpp.yml",
                "opencv-ml-src/mlsvm.cpp.yml",
                "opencv/NOTICE.yml",
                "oprofile-daemon-liblegacy/p_module.h.yml",
                "oprofile-utils/opcontrol.yml",
                "ppp-pppd-plugins/winbind.c.yml",
                "proguard-src-proguard-gui/GUIResources.properties.yml",
                "qemu-distrib-sdl-1.2.12-src-cdrom-osf/SDL_syscdrom.c.yml",
                "qemu-distrib-sdl-1.2.12-src-joystick-os2/joyos2.h.yml",
                "qemu-pc-bios-bochs-bios/rombios.c.yml",
                "qemu-pc-bios-vgabios/README.yml",
                "qemu-pc-bios-vgabios/vgabios.c.yml",
                "qemu/device_tree.c.yml",
                "qemu/migration-exec.c.yml",
                "quake-quake-src-QW-client/cd_linux.c.yml",
                "quake-quake-src-QW-client/exitscrn.txt.yml",
                "quake-quake-src-WinQuake/cl_input.cpp.yml",
                "quake-quake-src-WinQuake/menu.cpp.yml",
                "quake-quake-src-WinQuake/mpdosock.h.yml",
                "sonivox-docs/JET_Authoring_Guidelines.html.yml",
                "sonivox-docs/JET_Creator_User_Manual.html.yml",
                "sonivox-docs/JET_Programming_Manual.html.yml",
                "speex/NOTICE.yml",
                "strace-strace-linux-s390/syscallent.h.yml",
                "svox-pico-lib/picofftsg.c.yml",
                "svox-pico-lib/picoos.c.yml",
                "tcpdump/print-sctp.c.yml",
                "tcpdump/print-snmp.c.yml",
                "webrtc-src-modules-audio_processing-aec-main-source/aec_rdft.c.yml",
                "webrtc/NOTICE.yml",
            ],
        );
    }
}
