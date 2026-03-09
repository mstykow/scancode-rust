# Go Parser Golden Test Suite

## Purpose

Golden tests compare parser output against expected results from the original ScanCode Toolkit to ensure compatibility.

## Test Status

**Currently Passing:** 6/6 tests

- ✅ `test_golden_kingpin_mod`
- ✅ `test_golden_sample_mod`
- ✅ `test_golden_opencensus_service_mod`
- ✅ `test_golden_sample2_sum`
- ✅ `test_golden_sample3_sum`
- ✅ `test_golden_sample_graph`

## Test Coverage

### Active Tests

1. **kingpin-mod** - `go.mod` with direct and indirect requirements
2. **sample-mod** - `go.mod` with `exclude` coverage
3. **opencensus-service** - `go.mod` with `replace` directive coverage
4. **sample2-sum** - `go.sum` dedup coverage
5. **sample3-sum** - `go.sum` `/go.mod` line handling
6. **sample-graph** - `go.mod graph` direct vs transitive dependency coverage

## Test Data

Test files sourced from Python ScanCode reference:

- `reference/scancode-toolkit/tests/packagedcode/data/golang/`
