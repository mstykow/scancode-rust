# Package Parser Enhancement Plan

> **Status**: 🟡 Active — ecosystem-by-ecosystem enhancement backlog and execution tracker
> **Updated**: March 7, 2026
> **Dependencies**: [PARSER_PLAN.md](PARSER_PLAN.md), [ASSEMBLY_PLAN.md](ASSEMBLY_PLAN.md), [HOW_TO_ADD_A_PARSER.md](../../HOW_TO_ADD_A_PARSER.md), [TESTING_STRATEGY.md](../../TESTING_STRATEGY.md)

## Purpose

This document is the durable workboard for open package-parser enhancement work. It exists so future sessions can resume the effort without rebuilding the issue inventory, sequencing, or testing expectations from scratch.

## How to Use This Plan

At the start of each session:

1. Read this file first.
2. Pick the next ecosystem marked `Planned` unless priorities changed.
3. Before coding, confirm the issue set is still open with GitHub.
4. After finishing an ecosystem PR, update this file:
   - mark the ecosystem `Done` or `In progress`
   - add any follow-up issues created or intentionally deferred
5. If sequencing changes, update the order here instead of relying on chat history.

## Ground Rules

- One PR per ecosystem family.
- Multiple commits inside a PR are fine if they are atomic and reviewable.
- Do not mix unrelated ecosystems in one PR.
- Do not hide generic refactors inside ecosystem PRs.
- Use Python ScanCode as a behavior/spec reference, not as an implementation blueprint.
- Use the relevant parts of [`docs/HOW_TO_ADD_A_PARSER.md`](../../HOW_TO_ADD_A_PARSER.md) as the checklist for registration, golden tests, assembly, validation, and documentation, even when enhancing an existing parser.
- Add parser golden tests when parser-level parity/regression coverage is meaningful.
- Add assembly golden tests when assembly, workspace, or file-reference behavior changes.
- When parser work fixes Python bugs or adds beyond-parity behavior, document it in `docs/improvements/`.
- PRs that fully resolve issues should close them via GitHub closing keywords in the PR body (for example: `Closes #122`).
- Snapshot/expected-output changes require explicit rationale in the PR body.

## Required Touchpoints Per Ecosystem PR

- Parser modules in `src/parsers/`
- `src/parsers/mod.rs`
- Ecosystem-specific unit tests in `src/parsers/*_test.rs`
- Ecosystem-specific golden tests in `src/parsers/*_golden_test.rs`
- Real fixtures under `testdata/`
- `src/models/datasource_id.rs` when datasource coverage changes
- `src/assembly/assemblers.rs` when package assembly membership changes
- `src/assembly/assembly_golden_test.rs` and `testdata/assembly-golden/` when assembly behavior changes
- `docs/SUPPORTED_FORMATS.md` when parser metadata changes
- `docs/improvements/*.md` when Rust behavior intentionally exceeds or corrects Python behavior

## Session Refresh Commands

Refresh the enhancement backlog:

```bash
gh issue list --state open --label package-parsing --label enhancement --limit 200
```

Refresh one ecosystem only:

```bash
gh issue list --state open --label package-parsing --label enhancement --search '"Improve Maven:" in:title'
```

## Validation Baseline For Every Ecosystem PR

```bash
cargo fmt --all -- --check
```

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

```bash
cargo test assembly::assemblers::tests::test_every_datasource_id_is_accounted_for --lib
```

```bash
cargo run --quiet --bin generate-supported-formats && git diff --exit-code docs/SUPPORTED_FORMATS.md
```

## Ecosystem Workboard

| Order | Ecosystem             | Status  | Issue Set                                                                                                  | Primary Validation                                                                                                                                                                                                                                                                                                                                                             |
| ----- | --------------------- | ------- | ---------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1     | Maven                 | Done    | #122, #124, #126, #128, #131, #135, #207, #208, #211, #214                                                 | `cargo test maven`; `cargo test --features golden-tests maven_golden`; `cargo test --features golden-tests test_assembly_maven_basic`                                                                                                                                                                                                                                          |
| 2     | npm + Yarn            | Planned | #123, #125, #127, #129, #133, #197, #198, #205, #206                                                       | `cargo test npm`; `cargo test yarn`; `cargo test --features golden-tests npm_golden`; `cargo test --features golden-tests test_assembly_npm_basic`; `cargo test --features golden-tests test_assembly_npm_workspace`; `cargo test --features golden-tests test_assembly_pnpm_workspace`                                                                                        |
| 3     | NuGet                 | Planned | #157, #159, #162, #163, #165, #215, #216                                                                   | `cargo test nuget`; `cargo test --features golden-tests nuget_golden`                                                                                                                                                                                                                                                                                                          |
| 4     | RPM                   | Planned | #164, #166, #167, #168, #169, #170, #171                                                                   | `cargo test rpm`; `cargo test --features golden-tests rpm_golden`                                                                                                                                                                                                                                                                                                              |
| 5     | Cargo                 | Planned | #184, #189, #217                                                                                           | `cargo test cargo`; `cargo test --features golden-tests cargo_golden`; `cargo test --features golden-tests test_assembly_cargo_basic`; `cargo test --features golden-tests test_assembly_cargo_workspace`                                                                                                                                                                      |
| 6     | Go                    | Planned | #152, #153, #155, #218                                                                                     | `cargo test go`; `cargo test --features golden-tests go_golden`; `cargo test --features golden-tests test_assembly_go_basic`                                                                                                                                                                                                                                                   |
| 7     | Gradle                | Planned | #130, #132, #134, #137                                                                                     | `cargo test gradle`; `cargo test --features golden-tests gradle_golden`                                                                                                                                                                                                                                                                                                        |
| 8     | Ruby                  | Planned | #151, #154, #156, #158, #160, #161                                                                         | `cargo test ruby`; `cargo test --features golden-tests ruby_golden`                                                                                                                                                                                                                                                                                                            |
| 9     | Composer              | Planned | #187, #188, #190                                                                                           | `cargo test composer`; `cargo test --features golden-tests composer_golden`; `cargo test --features golden-tests test_assembly_composer_basic`                                                                                                                                                                                                                                 |
| 10    | Conda                 | Planned | #195, #196                                                                                                 | `cargo test conda`; `cargo test --features golden-tests conda_golden`                                                                                                                                                                                                                                                                                                          |
| 11    | CocoaPods             | Planned | #191, #192                                                                                                 | `cargo test pod`; `cargo test --features golden-tests cocoapods_golden`                                                                                                                                                                                                                                                                                                        |
| 12    | Alpine                | Planned | #172, #173, #174, #175                                                                                     | `cargo test alpine`; `cargo test --features golden-tests alpine_golden`; `cargo test --features golden-tests test_assembly_alpine_file_refs`                                                                                                                                                                                                                                   |
| 13    | ABOUT                 | Planned | #201, #202, #203, #204                                                                                     | `cargo test about`                                                                                                                                                                                                                                                                                                                                                             |
| 14    | Swift                 | Planned | #193                                                                                                       | `cargo test swift`; `cargo test --features golden-tests swift_golden`                                                                                                                                                                                                                                                                                                          |
| 15    | Conan                 | Planned | #194                                                                                                       | `cargo test conan`                                                                                                                                                                                                                                                                                                                                                             |
| 16    | Docker                | Planned | #199, #200                                                                                                 | validation to be finalized when parser files are added because this family is enhancement work around currently unsupported Docker-specific coverage                                                                                                                                                                                                                           |
| 17    | Python                | Planned | #136, #138, #139, #140, #141, #142, #143, #144, #145, #146, #147, #148, #149, #150, #209, #210, #212, #213 | `cargo test python`; `cargo test requirements_txt`; `cargo test pipfile_lock`; `cargo test poetry_lock`; `cargo test pip_inspect_deplock`; `cargo test --features golden-tests python_golden`; `cargo test --features golden-tests requirements_txt_golden`; `cargo test --features golden-tests pipfile_lock_golden`; `cargo test --features golden-tests poetry_lock_golden` |
| 18    | Debian                | Planned | #176, #177, #178, #179, #180, #181, #182, #183, #185, #186, #219                                           | `cargo test debian`; `cargo test --features golden-tests debian_golden`                                                                                                                                                                                                                                                                                                        |
| 19    | General cross-cutting | Planned | #220, #221                                                                                                 | only after enough ecosystem work reveals a stable shared fix                                                                                                                                                                                                                                                                                                                   |

## Detailed Scoping Rules

### Maven Pilot PR

Start here unless priorities change.

Issues:

- #122 POM license normalization
- #124 dependencyManagement field support
- #126 PURL qualifiers processing
- #128 duplicated name in description
- #131 relocations metadata support
- #135 JAR META-INF POM handling validation
- #207 treat META-INF as top-level directory for Java JARs
- #208 fix extracted license statement display in POM
- #211 fix typo in party role field
- #214 add support for POM 4.1.0 model

Likely touchpoints:

- `src/parsers/maven.rs`
- `src/parsers/maven_test.rs`
- `src/parsers/maven_golden_test.rs`
- `src/assembly/assembly_golden_test.rs`
- `testdata/maven/`
- `testdata/maven-golden/`
- `testdata/assembly-golden/maven-basic/`

Current status (March 7, 2026):

- The Maven ecosystem batch now includes parser, unit-test, parser-golden, assembly-golden, and improvement-doc updates covering the listed issue set.
- Verification completed with `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test maven --lib`, and `cargo test --features golden-tests maven_golden --lib`.
- No intentional Maven deferrals are currently recorded in this plan; issue-closing keywords belong in the PR body.

### Python PR Scope Rule

Include only:

- `setup.py`, `setup.cfg`, `pyproject.toml`, wheel/PKG-INFO, PyPI JSON, and file-assignment issues listed in this plan
- parser/golden/test coverage needed to close those specific issues

Exclude from the Python PR:

- unsupported new-parser work outside the current issue set
- broad dependency-engine or license-engine refactors
- unrelated scanner or archive refactors unless strictly required for one listed issue

If Python becomes too large for one reviewable PR, split by sub-family in this order and update this document:

1. setup.py / setup.cfg correctness
2. wheel / PKG-INFO / PyPI metadata
3. file-to-package assignment and package file collection

### Debian PR Scope Rule

Include only:

- `debian/` directory detection
- control-file dependency parsing
- copyright parsing and detection behavior
- `.deb` package and container-image Debian metadata covered by the listed issues

Exclude from the Debian PR:

- cross-ecosystem license engine refactors
- non-Debian distro work
- generic file-reference or scanner refactors unless they are strictly necessary to close a listed Debian issue

## PR Template Checklist

Each ecosystem PR should state:

- issues covered
- issue-closing keywords for fully resolved issues
- explicit exclusions
- intentional differences from Python
- follow-up issues created or intentionally deferred

## Completion Tracking

When an ecosystem PR lands, update the row above to one of:

- `In progress`
- `Done`
- `Deferred`

GitHub remains the source of truth for merged PR numbers and issue-closure links; this document tracks status and scope only.
