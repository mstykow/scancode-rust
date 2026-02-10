# Email and URL Detection Implementation Plan

> **Status**: 🔴 Placeholder - Not Started
> **Priority**: P2 - Medium Priority
> **Estimated Effort**: 1-2 weeks
> **Dependencies**: None

## Overview

Extract email addresses and URLs from source code files for contact information and reference tracking.

## Scope

### What This Covers

- Email address extraction
- URL/URI extraction (http, https, ftp, etc.)
- Validation and normalization
- Deduplication

### What This Doesn't Cover

- Email/URL validation against external services
- Link checking (dead link detection)
- Email obfuscation detection

## Python Reference Implementation

**Location**: `reference/scancode-toolkit/src/cluecode/`

**Key Components**:

- Pattern matching for email addresses
- URL regex patterns
- Validation logic

## Current State in Rust

### Implemented

- ✅ Email/URL field structures in file data
- ✅ Output format placeholders

### Missing

- ❌ Email extraction patterns
- ❌ URL extraction patterns
- ❌ Validation logic
- ❌ Scanner integration

## Implementation Phases (TBD)

1. **Phase 1**: Email regex patterns
2. **Phase 2**: URL regex patterns
3. **Phase 3**: Validation and normalization
4. **Phase 4**: Scanner integration

## Success Criteria

- [ ] Extracts valid email addresses
- [ ] Extracts URLs with various schemes
- [ ] Handles obfuscated formats (dot, at, etc.)
- [ ] Golden tests pass

## Related Documents

- **Implementation**: `COPYRIGHT_DETECTION_PLAN.md` (similar pattern matching)

## Notes

- Relatively straightforward regex-based extraction
- Consider using existing Rust email/URL parsing crates
