# Progress Tracking & Reporting Implementation Plan

> **Status**: 🔴 Placeholder - Not Started
> **Priority**: P3 - Low Priority (UX Feature)
> **Estimated Effort**: 1-2 weeks
> **Dependencies**: None

## Overview

Enhanced progress reporting during scans: progress bars, ETA, throughput metrics, and real-time status updates.

## Scope

### What This Covers

- **Progress Bar**: Visual progress indicator with percentage
- **ETA Calculation**: Estimated time to completion
- **Throughput Metrics**: Files/second, MB/second
- **Phase Indicators**: Show current phase (scanning, post-processing, etc.)
- **Error Reporting**: Real-time error/warning display
- **Quiet Mode**: Suppress progress output for CI/CD

### What This Doesn't Cover

- Logging infrastructure (separate concern)
- Detailed debug output (use logging)

## Python Reference Implementation

**Location**: `reference/scancode-toolkit/src/scancode/cli.py`

**Key Features**:

- **Click ProgressBar**: Uses Click's progress bar
- **Phase Tracking**: Shows current processing phase
- **File Count**: Displays files processed / total files
- **Error Reporting**: Shows errors as they occur

## Current State in Rust

### Implemented

- ✅ Basic progress bar (indicatif crate)
- ✅ File count tracking

### Missing

- ❌ ETA calculation
- ❌ Throughput metrics
- ❌ Phase indicators
- ❌ Real-time error display
- ❌ Quiet mode

## Implementation Phases (TBD)

1. **Phase 1**: Enhanced progress bar with ETA
2. **Phase 2**: Throughput metrics
3. **Phase 3**: Phase indicators
4. **Phase 4**: Real-time error display
5. **Phase 5**: Quiet mode and verbosity levels

## Success Criteria

- [ ] Progress bar shows accurate percentage
- [ ] ETA is reasonably accurate
- [ ] Throughput metrics displayed
- [ ] Phase indicators show current operation
- [ ] Errors displayed in real-time
- [ ] Quiet mode suppresses all output

## Related Documents

- **Evergreen**: `ARCHITECTURE.md` (scanner pipeline)

## Notes

- Already using `indicatif` crate for progress bars
- Consider adding `--quiet` and `--verbose` flags
- ETA calculation should account for variable file sizes
- Phase indicators help users understand what's happening
