# License Detection Golden Test Data

This directory contains test files and expected outputs for the license detection golden test suite.

## Directory Structure

```
license-golden/
├── single-license/     # Simple single-license detection tests
│   ├── mit.txt         # Input: MIT license text
│   ├── mit.txt.expected # Expected JSON output from Python ScanCode
│   ├── apache-2.0.txt
│   ├── apache-2.0.txt.expected
│   └── ...
├── multi-license/      # Multiple licenses in one file
│   ├── ffmpeg-LICENSE.md
│   ├── ffmpeg-LICENSE.md.expected
│   └── ...
├── spdx-lid/           # SPDX-License-Identifier headers
│   ├── license
│   ├── license.expected
│   └── ...
├── hash-match/         # Exact whole-file hash matches
│   ├── query.txt
│   ├── query.txt.expected
│   └── ...
├── seq-match/          # Sequence alignment (partial/modified licenses)
│   ├── partial.txt
│   ├── partial.txt.expected
│   └── ...
├── unknown/            # Unknown license detection
│   ├── unknown.txt
│   ├── unknown.txt.expected
│   └── ...
├── false-positive/     # Cases that should NOT match
│   ├── false-positive-gpl3.txt
│   ├── false-positive-gpl3.txt.expected
│   └── ...
└── reference/          # License references ("See COPYING", etc.)
    ├── see-copying.txt
    ├── see-copying.txt.expected
    └── ...
```

## Expected File Format

Each `.expected` file is a JSON file containing the output from Python ScanCode with the `--license` flag. The format should include:

```json
{
  "license_detections": [
    {
      "license_expression": "mit",
      "license_expression_spdx": "MIT",
      "matches": [
        {
          "license_expression": "mit",
          "matcher": "1-hash",
          "score": 100.0,
          "match_coverage": 100.0,
          "rule_relevance": 100,
          "start_line": 1,
          "end_line": 21
        }
      ],
      "detection_log": ["perfect-detection"]
    }
  ]
}
```

## Generating Expected Files

To generate expected files from Python ScanCode:

```bash
cd reference/scancode-toolkit
scancode --license --license-text \
    path/to/input/file.txt \
    --json path/to/output/file.txt.expected
```

## Adding New Test Cases

1. Add the input file to the appropriate category directory
2. Generate the expected output using Python ScanCode
3. Run `cargo test license_detection_golden_test` to verify the test
