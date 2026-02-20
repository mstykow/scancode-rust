# NuGet Party Type Inference

## Summary

The Rust implementation infers party types (`person` or `organization`) for NuGet packages, which the Python reference does not do.

## Behavior Difference

**Python:** `parties[].type` is always `null`

**Rust:** `parties[].type` is inferred using heuristics:

- Organization indicators: "Inc.", "LLC", "Corp", "Foundation", "Project", etc.
- Known organizations: "Microsoft", "Google", "Amazon", etc.
- Multiple comma-separated names: treated as individuals (`person`)
- Default: `person`

## Rationale

Providing party type information improves SBOM quality and enables better downstream analysis. The heuristics are conservative to minimize false positives.

## Files Modified

- `src/parsers/nuget.rs`: Added `infer_party_type()` function
- `testdata/nuget-golden/*.expected`: Updated to include `type` field
