# Go Parser Golden Test Suite

## Purpose

Golden tests compare parser output against expected results from the original ScanCode Toolkit to ensure compatibility.

## Coverage Summary

This fixture set covers `go.mod` direct and indirect requirements, `exclude` and `replace` directives, `go.sum` normalization behavior, checked-in `go mod graph` outputs, and representative `go.work` workspace forms.

## Test Data

Test files sourced from Python ScanCode reference:

- `reference/scancode-toolkit/tests/packagedcode/data/golang/`

Workspace member `go.mod` files under the `gowork-*` fixtures are local companion files used to resolve module paths for `use` entries.
