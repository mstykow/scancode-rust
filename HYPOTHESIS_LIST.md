# Hypothesis List for 90 Failing Golden Tests

## Active Hypotheses

### H1: QueryRun Splitting Disabled
- **Impact**: ~25 tests (missing detections)
- **Root cause**: QueryRun splitting is disabled, files with 4+ blank lines between licenses don't get separate matches
- **Investigation**: Compare Python vs Rust on files with multiple license sections
- **Status**: PENDING

### H2: Multi-Occurrence Deduplication
- **Impact**: ~25 tests (missing detections of same license at different locations)
- **Root cause IDENTIFIED**: Containment filtering removes 100% Aho matches when covered by larger seq match
- **Example**: flex-readme.txt has 3 Aho matches, but 1 seq match covers them all
- **Python behavior**: Produces 3 separate detections (TBD how)
- **Status**: INVESTIGATED - Need to understand Python's detection grouping

### H3: Required Phrase Validation Missing
- **Impact**: ~20 tests (CC-BY-SA detected as CC-BY-NC-SA)
- **Root cause IDENTIFIED**: NOT required phrases - candidate tie-breaking issue
- **Actual issue**: Sequence matcher gives same scores to SA and NC-SA rules
- **Status**: INVESTIGATED - Need better candidate ranking

### H4: German Text Character Normalization
- **Impact**: ~15 tests
- **Root cause IDENTIFIED**: German "ß" not normalized to "ss" in tokenization
- **Example**: "gemäß" doesn't match "gemass" in GPL rules
- **Fix READY**: Add German character normalization in tokenize.rs
- **Status**: READY TO IMPLEMENT

### H6: Binary File Detection
- **Impact**: ~2 tests
- **Root cause**: Binary files should return empty detections
- **Status**: PENDING

## Investigation Protocol
1. Pick top 3 hypotheses
2. Launch parallel subagent investigations
3. Each investigation:
   - Analyze specific failing test cases
   - Compare Python vs Rust behavior
   - Identify root cause and fix location
   - Recommend implementation approach
4. Verify findings with Python reference
5. Create detailed implementation plan
6. Implement and test
7. Commit if golden tests improve
