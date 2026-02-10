# scancode-rust

A high-performance code scanning tool written in Rust that detects licenses, copyrights, and other relevant metadata in source code.

## Overview

`scancode-rust` is designed to be a faster alternative to the Python-based [ScanCode Toolkit](https://github.com/nexB/scancode-toolkit), aiming to produce compatible output formats while delivering significantly improved performance. This tool currently scans codebases to identify:

- License information
- File metadata
- System information

More ScanCode features coming soon!

## Features

- Efficient file scanning with multi-threading
- Compatible output format with ScanCode Toolkit
- Progress indication for large scans
- Configurable scan depth
- File/directory exclusion patterns

## Installation

### From Crates.io (Recommended)

```sh
cargo install scancode-rust
```

### Download Precompiled Binary

Download the appropriate binary for your platform from the [GitHub Releases](https://github.com/mstykow/scancode-rust/releases) page:

- **Linux (x64)**: `scancode-rust-x86_64-unknown-linux-gnu.tar.gz`
- **Linux (ARM64)**: `scancode-rust-aarch64-unknown-linux-gnu.tar.gz`
- **macOS (Apple Silicon & Intel)**: `scancode-rust-aarch64-apple-darwin.tar.gz`
  - Intel Macs: Use Rosetta 2 for native-like performance
- **Windows**: `scancode-rust-x86_64-pc-windows-msvc.zip`

Extract and place the binary in your system's PATH:

```sh
# Example for Linux/macOS
tar xzf scancode-rust-*.tar.gz
sudo mv scancode-rust /usr/local/bin/
```

### Build from Source

```sh
git clone https://github.com/yourusername/scancode-rust.git
cd scancode-rust
./setup.sh  # Initialize the submodule and configure sparse checkout
cargo build --release
```

The compiled binary will be available at `target/release/scancode-rust`.

## Usage

```sh
scancode-rust [OPTIONS] <DIR_PATH> --output-file <OUTPUT_FILE>
```

### Options

```sh
Options:
  -o, --output-file <OUTPUT_FILE>    Output JSON file path
  -d, --max-depth <MAX_DEPTH>        Maximum directory depth to scan [default: 50]
  -e, --exclude <EXCLUDE>...         Glob patterns to exclude from scanning
      --no-assemble                  Disable package assembly (merging related manifest/lockfiles)
  -h, --help                         Print help
  -V, --version                      Print version
```

### Example

```sh
scancode-rust ~/projects/my-codebase -o scan-results.json --exclude "*.git*" "target/*" "node_modules/*"
```

## Performance

`scancode-rust` is designed to be significantly faster than the Python-based ScanCode Toolkit, especially for large codebases, thanks to native Rust performance and parallel processing. See [Architecture: Performance Characteristics](docs/ARCHITECTURE.md#performance-characteristics) for details.

## Output Format

The tool produces JSON output compatible with ScanCode Toolkit, including:

- Scan headers with timestamp information
- File-level data with license and metadata information
- System environment details

## Documentation

- **[Architecture](docs/ARCHITECTURE.md)** - System design, processing pipeline, and design decisions
- **[Supported Formats](docs/SUPPORTED_FORMATS.md)** - Complete list of supported package ecosystems and file formats
- **[How to Add a Parser](docs/HOW_TO_ADD_A_PARSER.md)** - Step-by-step guide for adding new parsers
- **[Testing Strategy](docs/TESTING_STRATEGY.md)** - Testing approach and guidelines
- **[ADRs](docs/adr/)** - Architectural decision records
- **[Beyond-Parity Improvements](docs/improvements/)** - Features where Rust exceeds the Python original

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### Setting Up for Local Development

To contribute to `scancode-rust`, follow these steps to set up the repository for local development:

1. **Install Rust**  
   Ensure you have Rust installed on your system. You can install it using [rustup](https://rustup.rs/):

   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Clone the Repository**  
   Clone the `scancode-rust` repository to your local machine:

   ```sh
   git clone https://github.com/mstykow/scancode-rust.git
   cd scancode-rust
   ```

3. **Initialize the License Submodule**  
   Use the following script to initialize the submodule and configure sparse checkout:

   ```sh
   ./setup.sh
   ```

4. **Install Dependencies**  
   Install the required Rust dependencies using `cargo`:

   ```sh
   cargo build
   ```

5. **Run Tests**  
   Run the test suite to ensure everything is working correctly:

   ```sh
   cargo test
   ```

6. **Set Up Pre-commit Hooks**  
   This repository uses [pre-commit](https://pre-commit.com/) to run checks before each commit:

   ```sh
   # Using pip
   pip install pre-commit

   # Or using brew on macOS
   brew install pre-commit

   # Install the hooks
   pre-commit install
   ```

7. **Start Developing**  
   You can now make changes and test them locally. Use `cargo run` to execute the tool:

   ```sh
   cargo run -- [OPTIONS] <DIR_PATH>
   ```

## Publishing a Release (Maintainers Only)

Releases are automated using [`cargo-release`](https://github.com/crate-ci/cargo-release) and GitHub Actions.

### Prerequisites

**One-time setup:**

1. Install `cargo-release` CLI tool:

   ```sh
   cargo install cargo-release
   ```

2. Authenticate with crates.io (one-time only):

   ```sh
   cargo login
   ```

   Enter your [crates.io API token](https://crates.io/me) when prompted. This is stored in `~/.cargo/credentials.toml` and persists across sessions.

### Release Process

Use the `release.sh` script:

```sh
# Dry-run first (recommended)
./release.sh patch

# Then execute the actual release
./release.sh patch --execute
```

Available release types:

- `patch`: Increments the patch version (0.0.4 → 0.0.5)
- `minor`: Increments the minor version (0.0.4 → 0.1.0)
- `major`: Increments the major version (0.0.4 → 1.0.0)

**What happens automatically:**

1. **Updates SPDX license data** to the latest version from upstream
2. Commits the license data update (if changes detected)
3. `cargo-release` updates the version in `Cargo.toml` and `Cargo.lock`
4. Creates a git commit: `chore: release vX.Y.Z`
5. Creates a GPG-signed git tag: `vX.Y.Z`
6. Publishes to crates.io
7. Pushes commits and tag to GitHub
8. GitHub Actions workflow is triggered by the tag
9. Builds binaries for all platforms:
   - Linux: x64 and ARM64
   - macOS: ARM64 (Apple Silicon, works on Intel via Rosetta 2)
   - Windows: x64
10. Creates archives (.tar.gz/.zip) and SHA256 checksums
11. Creates a GitHub Release with all artifacts and auto-generated release notes

> **Note**: The release script ensures every release ships with the latest SPDX license definitions. It also handles a sparse checkout workaround for `cargo-release`.

Monitor the [GitHub Actions workflow](https://github.com/mstykow/scancode-rust/actions) to verify completion.

## License

This project is licensed under the [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0).
