# Gitmodules Parser: New Feature

## Summary

The `.gitmodules` parser in scancode-rust is a **new feature** not present in the Python reference implementation:

- **✨ New Feature**: Parse `.gitmodules` files to treat git submodules as dependencies

## Python Implementation (MISSING)

**Location**: `reference/scancode-toolkit/src/packagedcode/`

**Status**: No `.gitmodules` parser exists. Git submodules are not detected as dependencies.

**Impact**: Projects using git submodules have incomplete dependency graphs in scan results.

### Our Rust Implementation (NEW)

**Location**: `src/parsers/gitmodules.rs`

**Approach**: Parse INI-style `.gitmodules` files and treat each submodule as a dependency:

```rust
pub struct GitmodulesParser;

impl PackageParser for GitmodulesParser {
    const PACKAGE_TYPE: PackageType = PackageType::Github;

    fn is_match(path: &Path) -> bool {
        path.file_name().is_some_and(|name| name == ".gitmodules")
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        let content = read_file_to_string(path)?;
        let submodules = parse_gitmodules(&content);
        
        let dependencies: Vec<Dependency> = submodules
            .into_iter()
            .map(|sub| Dependency {
                purl: sub.purl,
                extracted_requirement: Some(format!("{} at {}", sub.path, sub.url)),
                scope: Some("runtime".to_string()),
                is_runtime: Some(true),
                is_direct: Some(true),
                is_pinned: Some(false),
                ..Default::default()
            })
            .collect();

        vec![PackageData {
            package_type: Some(PackageType::Github),
            datasource_id: Some(DatasourceId::Gitmodules),
            dependencies,
            ..Default::default()
        }]
    }
}
```

## Supported Format

### Input: `.gitmodules`

```ini
[submodule "dep-lib"]
    path = lib/dep
    url = https://github.com/user/dep-lib.git

[submodule "private-repo"]
    path = private
    url = git@github.com:company/private-repo.git

[submodule "gitlab-dep"]
    path = gitlab-lib
    url = https://gitlab.com/group/project.git
```

### Output: Dependencies

```json
{
  "package_type": "github",
  "datasource_id": "gitmodules",
  "dependencies": [
    {
      "purl": "pkg:github/user/dep-lib",
      "extracted_requirement": "lib/dep at https://github.com/user/dep-lib.git",
      "scope": "runtime",
      "is_runtime": true,
      "is_direct": true,
      "is_pinned": false
    },
    {
      "purl": "pkg:github/company/private-repo",
      "extracted_requirement": "private at git@github.com:company/private-repo.git",
      "scope": "runtime",
      "is_direct": true,
      "is_pinned": false
    },
    {
      "purl": "pkg:gitlab/group/project",
      "extracted_requirement": "gitlab-lib at https://gitlab.com/group/project.git",
      "scope": "runtime",
      "is_direct": true,
      "is_pinned": false
    }
  ]
}
```

## Features

| Feature | Description |
|---------|-------------|
| INI parsing | Handles standard `.gitmodules` INI format |
| GitHub URLs | Generates `pkg:github/...` PURLs for github.com URLs |
| GitLab URLs | Generates `pkg:gitlab/...` PURLs for gitlab.com URLs |
| SSH format | Handles `git@github.com:user/repo.git` URLs |
| HTTPS format | Handles `https://github.com/user/repo.git` URLs |
| Comments | Ignores `#` and `;` comment lines |
| Unknown hosts | Stores URL in extracted_requirement (no PURL) |

## URL Parsing

### GitHub URLs

Both HTTPS and SSH formats are supported:

```text
https://github.com/user/repo.git    → pkg:github/user/repo
git@github.com:user/repo.git        → pkg:github/user/repo
https://github.com/org/repo         → pkg:github/org/repo
```

### GitLab URLs

```text
https://gitlab.com/group/project.git → pkg:gitlab/group/project
git@gitlab.com:group/project.git     → pkg:gitlab/group/project
```

### Other URLs

For URLs that aren't GitHub or GitLab:

```json
{
  "purl": null,
  "extracted_requirement": "path at https://example.com/repo.git"
}
```

## Test Coverage

- `test_is_match()` - Correctly identifies `.gitmodules` files
- `test_parse_single_submodule()` - Single submodule extraction
- `test_parse_multiple_submodules()` - Multiple submodules
- `test_parse_git_ssh_url()` - SSH URL format (`git@github.com:...`)
- `test_parse_gitlab_url()` - GitLab URL handling
- `test_parse_unknown_url()` - Non-GitHub/GitLab URLs
- `test_parse_empty_file()` - Empty file handling
- `test_parse_with_comments()` - Comment handling

## Use Case

This parser enables:

1. **Complete dependency graphs** - Git submodules are now visible in scan results
2. **License compliance** - Can track licenses of submodule dependencies
3. **SBOM generation** - Submodules included in Software Bill of Materials
4. **Security scanning** - Check submodules for vulnerabilities

## Comparison

| Aspect | Python | Rust |
|--------|--------|------|
| `.gitmodules` parsing | ❌ Not supported | ✅ Full support |
| Submodule dependencies | ❌ Not detected | ✅ Detected as dependencies |
| GitHub PURL generation | N/A | ✅ Automatic |
| GitLab PURL generation | N/A | ✅ Automatic |

## Reference

- **Issue**: ScanCode #2853 - Treat git submodule as dependency manifest
- **Location**: `src/parsers/gitmodules.rs`
- **Datasource ID**: `gitmodules`
- **Package Type**: `github`
