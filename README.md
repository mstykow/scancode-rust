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

### Download Precompiled Binary

You can download the appropriate binary for your platform from the [GitHub Releases](https://github.com/mstykow/scancode-rust/releases) page. Simply extract the binary and place it in your system's PATH.

### Use the Installer Script

Alternatively, you can use the `scancode-rust-installer.sh` script to automatically download and install the correct binary for your architecture and platform:

```sh
curl -sSfL https://github.com/mstykow/scancode-rust/releases/latest/download/scancode-rust-installer.sh | sh
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
  -h, --help                         Print help
  -V, --version                      Print version
```

### Example

```sh
scancode-rust ~/projects/my-codebase -o scan-results.json --exclude "*.git*" "target/*" "node_modules/*"
```

## Performance

`scancode-rust` is designed to be significantly faster than the Python-based ScanCode Toolkit, especially for large codebases. Performance improvements come from:

- Native Rust implementation
- Efficient parallel processing
- Optimized file handling

## Output Format

The tool produces JSON output compatible with ScanCode Toolkit, including:

- Scan headers with timestamp information
- File-level data with license and metadata information
- System environment details

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

## Publishing a Release

This project uses [cargo-dist](https://github.com/axodotdev/cargo-dist) to automate the release process for both GitHub releases and crates.io.

### Prerequisites

1. Install cargo-dist:

   ```sh
   cargo install cargo-dist
   ```

2. Ensure you have the necessary permissions on the GitHub repository and for crates.io.

3. Authenticate with GitHub CLI (`gh`) and ensure you're logged in to crates.io:

   ```sh
   gh auth login
   cargo login
   ```

### Release Process

1. Update version in `Cargo.toml`:

   ```sh
   # Edit Cargo.toml to bump the version number
   vim Cargo.toml
   ```

2. Create a new git tag matching the version:

   ```sh
   git add Cargo.toml
   git commit -m "Bump version to x.y.z"
   git tag -a vx.y.z -m "Release version x.y.z"
   ```

3. Push the tag to trigger the release workflow:

   ```sh
   git push origin main --tags
   ```

4. The GitHub Actions workflow will:

   - Build binaries for all supported platforms
   - Create a GitHub release with the binaries

5. Monitor the GitHub Actions workflow to ensure the GitHub release completes successfully.

6. Publish to crates.io manually:

   ```sh
   cargo publish
   ```

## Updating the License Data

If you want to update the embedded license data, simply run the `setup.sh` script:

```sh
./setup.sh
```

This will reconfigure the sparse checkout and fetch the latest changes. After updating the license data, rebuild the binary:

```sh
cargo build --release
```

This will embed the latest changes from the `license-list-data` repository into the binary.

## License

This project is licensed under the [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0).
