# Debug License Detection Pipeline

This document describes how to use the license detection debugging script to analyze the ScanCode license detection pipeline in detail.

## Overview

The `debug_license_detection.py` script in the playground provides detailed instrumentation of the Python ScanCode license detection pipeline. It outputs information about each stage of the pipeline, which is essential for debugging license detection issues when porting to Rust or investigating unexpected behavior.

## Location

The script is located at:
```
reference/scancode-playground/debug_license_detection.py
```

## Usage

```bash
cd reference/scancode-playground
venv/bin/python debug_license_detection.py <file_path> [options]
```

### Options

- `--min-score=N` - Set minimum score threshold (default: 0)
- `--no-approximate` - Disable approximate matching stage

## Pipeline Stages

The script instruments the following stages:

### Stage 1: Query Building

Converts input text into a tokenized query broken into "runs" (chunks) based on heuristics like empty lines, long lines, and binary detection.

**Output includes:**
- File location and query string preview
- Total token count
- Query runs with start/end positions and matchable tokens

### Stage 2: Hash Matching

Performs SHA1 hash matching against known license rule hashes. If a hash match is found, the pipeline stops early with an exact match.

**Output includes:**
- Whether hash matches were found
- Matched rule identifiers and license expressions

### Stage 3: SPDX Identifier Matching

Parses `SPDX-License-Identifier:` expressions in the input text.

**Output includes:**
- SPDX identifier matches found
- Rule identifiers for detected expressions

### Stage 4: Exact Matching (Aho-Corasick)

Uses Aho-Corasick automaton for multi-pattern exact string matching.

**Output includes:**
- Exact matches found
- Matched rule identifiers

### Stage 5a: Candidate Selection

For approximate matching, pre-filters candidate rules using set/multiset intersection (bag-of-words approach).

**Output includes:**
- Total rules in index
- Query matchable tokens
- Top candidates with scores and intersection sizes

### Stage 5: Approximate Matching (Sequence)

Performs local sequence alignment (like diff) between query and candidate rules.

**Output includes:**
- Approximate matches found
- Score and coverage for each match

### Stage 6: Match Merging

Combines match fragments to the same rule that are in sequence.

**Output includes:**
- Match count before/after merge
- Details of merged matches

### Stage 7: Match Refinement (Filtering)

Applies multiple filter stages in order:

1. `filter_matches_missing_required_phrases` - Rules with required phrases
2. `filter_spurious_matches` - Remove spurious matches
3. `filter_below_rule_minimum_coverage` - Coverage thresholds
4. `filter_matches_to_spurious_single_token` - Single token matches
5. `filter_too_short_matches` - Minimum length
6. `filter_short_matches_scattered_on_too_many_lines` - Density check
7. `filter_invalid_matches_to_single_word_gibberish` - Gibberish filtering
8. `filter_contained_matches` - Remove matches contained in larger ones
9. `filter_overlapping_matches` - Handle overlapping matches
10. `filter_false_positive_matches` - False positive detection
11. `filter_false_positive_license_lists_matches` - License list filtering

**Output includes:**
- Match count before/after each filter
- Discarded matches with reasons

### Stage 8: Detection Assembly

Combines matches into `LicenseDetection` objects with final license expressions.

**Output includes:**
- Created detections
- License expressions and identifiers

## Example Output

```bash
$ cd reference/scancode-playground
$ venv/bin/python debug_license_detection.py apache-2.0.LICENSE

================================================================================
 LICENSE DETECTION DEBUG: apache-2.0.LICENSE
================================================================================
Time: 2026-03-01T23:07:46.499524

Loading license index...
Index loaded: 37545 rules

================================================================================
 STAGE 1: QUERY BUILDING
================================================================================
Location: apache-2.0.LICENSE
Query string: None...
Total tokens: 1584
Total lines tracked: 1584

--------------------------------------------------------------------------------
 Query Runs (chunks)
--------------------------------------------------------------------------------
Number of runs: 1

  Run 0:
    Start pos: 0, End pos: 1583
    Start line: 1, End line: 201
    Matchables count: 1584

================================================================================
 STAGE 2: HASH MATCHING
================================================================================
HASH MATCH FOUND: 1 match(es)
  - apache-2.0.LICENSE (license: apache-2.0)

*** HASH MATCH FOUND - stopping early (exact hash match) ***
```

## Use Cases

### Debugging Missing License Detections

If a license is not being detected when expected:

1. Run the debug script on the file
2. Check if hash matching fails (expected for modified licenses)
3. Check if SPDX identifiers are detected
4. Check if exact matching finds any matches
5. Check candidate selection scores in approximate matching
6. Check if matches are being filtered out in refinement stage

### Debugging Incorrect License Detections

If wrong licenses are being detected:

1. Run the debug script on the file
2. Check which stage produces the incorrect match
3. Examine the filter stages to see if matches should have been discarded
4. Check candidate scores in approximate matching

### Understanding Filter Behavior

To understand why matches are being filtered:

1. Run the debug script
2. Examine Stage 7 (Match Refinement)
3. Check each filter's before/after counts
4. Note which discarded matches are relevant

## Related Files

- `reference/scancode-toolkit/src/licensedcode/index.py` - Main matching pipeline
- `reference/scancode-toolkit/src/licensedcode/match.py` - Match merging and filtering
- `reference/scancode-toolkit/src/licensedcode/match_set.py` - Candidate selection
- `reference/scancode-toolkit/src/licensedcode/detection.py` - Detection assembly

## Environment Variables

The Python codebase has extensive tracing flags that can be enabled via environment variables:

- `SCANCODE_DEBUG_LICENSE` - Main license matching
- `SCANCODE_DEBUG_LICENSE_INDEX` - Index building
- `SCANCODE_DEBUG_LICENSE_DETECTION` - Detection assembly

These can provide additional detail beyond what the debug script outputs.
