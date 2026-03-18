# npm and Yarn Parser: Local Metadata, Lockfile, and Package-Root Improvements

## Summary

The current npm and Yarn enhancement pass in Provenant improves several behaviors that are missing, incomplete, or buggy in the Python reference:

- `package.json` now preserves npm `overrides` metadata.
- Empty npm `name` and `version` values no longer generate synthetic registry URLs or dummy PURLs.
- Scoped npm fallback URLs now use the correct registry path and tarball filename shape, and invalid `homepage`/blank `bugs` metadata is dropped instead of being kept as noisy values.
- `package-lock.json` root package identity now falls back to the `packages[""]` entry, matching modern hidden-lockfile layouts.
- npm lockfiles now preserve link-style and non-version dependency specs without incorrectly treating them as pinned registry versions.
- `yarn.lock` can infer direct dependency scope from a sibling `package.json`.
- npm package-root resource assignment now skips first-level `node_modules` while allowing nested bundled packages to own their own files, including `.pnp.cjs`-style project files at the package root.
- Workspace assembly now accepts array, string, and object-style npm workspace declarations, preserves unattached lockfile dependencies when a sibling manifest is not packageable, and emits deterministic package/file ordering.

## Python Reference Status

Relevant reference signals from the upstream ScanCode npm/Yarn packagedcode implementation:

- Python still carries TODO notes around newer npm/Yarn lockfile support and `pnp.js` / pnpm lockfile handling.
- Python's npm assembly relies on `walk_npm()` to assign resources while skipping first-level `node_modules`.
- Python updates Yarn/package-lock dependency metadata from nearby manifests during assembly/resolution.

## Rust Improvements

### 1. Preserve npm `overrides`

Rust now stores the raw `overrides` mapping in `PackageData.extra_data`, so override intent is preserved for downstream tooling and future dependency analysis.

### 2. Avoid dummy URLs for empty npm metadata

If `name` or `version` is empty or whitespace, Rust now treats it as missing metadata instead of building malformed registry URLs such as placeholder tarball or API paths.

Rust also now uses the correct scoped npm tarball shape (`/@scope/name/-/name-version.tgz`) for fallback URLs and drops invalid `homepage` arrays / blank `bugs.url` values instead of preserving misleading metadata.

### 3. Modern npm lockfile root fallback

For npm v2/v3 lockfiles, Rust now uses `packages[""]` as a fallback source for root `name` and `version`, which is important for hidden lockfiles and other modern layouts where top-level fields may be absent.

### 4. Preserve non-version npm lockfile dependencies

Rust now keeps `link: true` dependencies from modern lockfiles as dependency records with unversioned npm PURLs and source-path metadata in `extra_data`, instead of silently dropping them.

Rust also preserves non-version lockfile specs such as `file:`, `git+...`, tarball URLs, and `npm:` aliases as unpinned dependency requirements. This avoids emitting invalid versioned PURLs for source-based dependencies and prevents alias ranges like `npm:wrap-ansi@^7.0.0` from being misclassified as exact pinned versions.

### 5. Correct directness and infer Yarn scope from sibling `package.json`

When a `yarn.lock` sits next to a `package.json`, Rust now uses the manifest to classify direct dependencies as:

- `dependencies`
- `devDependencies`
- `optionalDependencies`
- `peerDependencies`

This improves direct/transitive classification and brings lockfile output closer to the manifest semantics users expect.

For npm lockfiles, Rust also now marks nested duplicate packages under `node_modules/.../node_modules/...` as transitive instead of incorrectly counting them as direct dependencies just because their package name also appears at the root.

### 6. Correct nested npm package ownership

Rust now assigns package-root resources for npm packages while skipping first-level `node_modules` for the parent package. Nested packages under `node_modules` therefore keep their own package identity instead of inheriting the parent package UID.

This directly improves bundled-package scans and also ensures root-level Yarn PnP files such as `.pnp.cjs` attach to the correct package.

### 7. Preserve workspace and unattached lockfile semantics

Workspace assembly now accepts npm workspace declarations from array, string, or `{ "packages": [...] }` forms, which prevents valid monorepos from being skipped during assembly.

When a sibling `package.json` exists but is not packageable, Rust now keeps lockfile dependencies as unattached top-level dependencies instead of incorrectly manufacturing a package from lockfile-only identity.

The assembly phase also sorts `datafile_paths`, `datasource_ids`, and `for_packages`, so workspace and sibling-merge output stays deterministic across runs.

## Primary Areas Affected

- npm manifest parsing and metadata normalization
- npm lockfile root identity and dependency-spec extraction
- Yarn lockfile dependency-scope inference
- npm/Yarn assembly behavior for sibling manifests, workspaces, deterministic ordering, and nested package ownership
- Parser and assembly regression coverage for the affected behaviors

## Verification

This enhancement set is covered by:

- npm parser-focused unit tests
- npm lockfile-focused unit tests
- Yarn lockfile-focused unit tests
- npm parser golden coverage
- npm workspace, pnpm workspace, and nested package assembly golden coverage
