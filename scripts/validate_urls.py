#!/usr/bin/env python3
"""
Systematically validate all URLs in documentation and docstrings.
Excludes: archived docs, testdata, test files, fixtures.
"""

import re
import subprocess
import sys
from pathlib import Path
from collections import defaultdict
from urllib.parse import urlparse
import concurrent.futures

# URL extraction patterns
MARKDOWN_LINK_PATTERN = re.compile(r"\[([^\]]+)\]\((https?://[^\s]+?)\)")
BARE_URL_PATTERN = re.compile(r'https?://[^\s"\'<>)\]]+')
DOCSTRING_PATTERN = re.compile(r"^\s*(///|//!)", re.MULTILINE)


def find_markdown_files(root: Path):
    """Find all markdown files excluding submodules, archived, testdata, tests."""
    excludes = {
        ".git",
        "target",
        "archived",
        "testdata",
        "tests",
        ".sisyphus",
        "reference",
        "resources",
    }

    for md_file in root.rglob("*.md"):
        # Check if any parent directory is in excludes
        if any(part in excludes for part in md_file.parts):
            continue
        yield md_file


def find_rust_files(root: Path):
    """Find all Rust files excluding tests."""
    src_dir = root / "src"
    if not src_dir.exists():
        return

    for rs_file in src_dir.rglob("*.rs"):
        # Skip test files
        if rs_file.name.endswith("_test.rs") or "tests" in rs_file.parts:
            continue
        yield rs_file


def extract_markdown_link_url(text: str, start_pos: int) -> str:
    """Extract URL from markdown link syntax, handling nested parentheses."""
    paren_depth = 0
    url_start = start_pos
    i = start_pos

    while i < len(text):
        if text[i] == "(":
            paren_depth += 1
        elif text[i] == ")":
            if paren_depth == 0:
                return text[url_start:i]
            paren_depth -= 1
        elif text[i] in " \t\n":
            break
        i += 1

    return text[url_start:i]


def extract_urls_from_markdown(file_path: Path):
    """Extract all URLs from markdown file."""
    urls = []
    try:
        content = file_path.read_text(encoding="utf-8")
        for line_no, line in enumerate(content.splitlines(), 1):
            i = 0
            while i < len(line):
                md_match = re.search(r"\]\((https?://)", line[i:])
                if md_match:
                    url_start = i + md_match.start() + 2
                    url = extract_markdown_link_url(line, url_start)
                    urls.append((str(file_path), line_no, url))
                    i = url_start + len(url)
                else:
                    break

            for match in BARE_URL_PATTERN.finditer(line):
                url = match.group(0).rstrip(".,;:")
                if not any(url in existing_url for _, _, existing_url in urls):
                    urls.append((str(file_path), line_no, url))
    except Exception as e:
        print(f"Error reading {file_path}: {e}", file=sys.stderr)
    return urls


def extract_urls_from_rust(file_path: Path):
    """Extract URLs from Rust docstrings (//! and ///)."""
    urls = []
    try:
        content = file_path.read_text(encoding="utf-8")
        for line_no, line in enumerate(content.splitlines(), 1):
            if DOCSTRING_PATTERN.match(line):
                i = 0
                while i < len(line):
                    md_match = re.search(r"\]\((https?://)", line[i:])
                    if md_match:
                        url_start = i + md_match.start() + 2
                        url = extract_markdown_link_url(line, url_start)
                        urls.append((str(file_path), line_no, url))
                        i = url_start + len(url)
                    else:
                        break

                for match in BARE_URL_PATTERN.finditer(line):
                    url = match.group(0).rstrip(".,;:'\"`").lstrip("<")
                    if not any(url in existing_url for _, _, existing_url in urls):
                        urls.append((str(file_path), line_no, url))
    except Exception as e:
        print(f"Error reading {file_path}: {e}", file=sys.stderr)
    return urls


def validate_url(url: str, timeout: int = 10) -> tuple:
    """Validate URL using curl with timeout."""
    # Skip template URLs and placeholders
    if any(placeholder in url for placeholder in ["{", "<", "...", "example.com"]):
        return (url, "SKIP", "Template/placeholder URL")

    # Skip fragment-only URLs (anchors)
    parsed = urlparse(url)
    if not parsed.netloc:
        return (url, "SKIP", "Relative or fragment URL")

    # Allowlist: Known false positives (sites blocking CI user agents)
    allowlist_patterns = [
        "crates.io",  # Blocks non-browser user agents
    ]
    if any(pattern in url for pattern in allowlist_patterns):
        return (url, "SKIP", "Allowlisted (blocks CI user agents)")

    try:
        result = subprocess.run(
            [
                "curl",
                "-sS",
                "--connect-timeout",
                "5",
                "--max-time",
                str(timeout),
                "-o",
                "/dev/null",
                "-w",
                "%{http_code}",
                url,
            ],
            capture_output=True,
            text=True,
            timeout=timeout + 2,
        )
        status_code = result.stdout.strip()

        if result.returncode != 0:
            return (url, "FAIL", f"Curl error: {result.stderr[:100]}")

        if status_code.startswith("2"):  # 2xx success
            return (url, "PASS", f"HTTP {status_code}")
        elif status_code.startswith("3"):  # 3xx redirect (usually OK)
            return (url, "PASS", f"HTTP {status_code} (redirect)")
        elif status_code == "000":
            return (url, "FAIL", "Connection failed")
        else:
            return (url, "FAIL", f"HTTP {status_code}")
    except subprocess.TimeoutExpired:
        return (url, "FAIL", f"Timeout after {timeout}s")
    except Exception as e:
        return (url, "FAIL", f"Exception: {str(e)[:100]}")


def main():
    project_root = Path(__file__).parent.parent

    print("=== Extracting URLs from Documentation ===\n")

    # Collect all URLs with their locations
    all_urls = []

    # Extract from markdown
    print("Scanning markdown files...")
    for md_file in find_markdown_files(project_root):
        all_urls.extend(extract_urls_from_markdown(md_file))

    # Extract from Rust docstrings
    print("Scanning Rust docstrings...")
    for rs_file in find_rust_files(project_root):
        all_urls.extend(extract_urls_from_rust(rs_file))

    print(f"\nFound {len(all_urls)} URL occurrences\n")

    # Group by URL for validation (avoid validating same URL multiple times)
    url_locations = defaultdict(list)
    for file_path, line_no, url in all_urls:
        url_locations[url].append((file_path, line_no))

    unique_urls = list(url_locations.keys())
    print(f"Unique URLs to validate: {len(unique_urls)}\n")

    print("=== Validating URLs ===\n")

    # Validate URLs in parallel
    results = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=10) as executor:
        future_to_url = {executor.submit(validate_url, url): url for url in unique_urls}
        for future in concurrent.futures.as_completed(future_to_url):
            results.append(future.result())
            # Show progress
            if len(results) % 10 == 0:
                print(
                    f"Validated {len(results)}/{len(unique_urls)}...", file=sys.stderr
                )

    # Sort results: FAIL first, then PASS, then SKIP
    status_order = {"FAIL": 0, "PASS": 1, "SKIP": 2}
    results.sort(key=lambda x: (status_order[x[1]], x[0]))

    # Print results
    fail_count = 0
    pass_count = 0
    skip_count = 0

    print("\n=== Validation Results ===\n")

    for url, status, message in results:
        if status == "FAIL":
            fail_count += 1
            print(f"❌ FAIL: {url}")
            print(f"   Reason: {message}")
            print(f"   Found in:")
            for file_path, line_no in url_locations[url][:3]:  # Show first 3 locations
                print(f"     - {file_path}:{line_no}")
            if len(url_locations[url]) > 3:
                print(f"     ... and {len(url_locations[url]) - 3} more")
            print()
        elif status == "PASS":
            pass_count += 1
        elif status == "SKIP":
            skip_count += 1

    # Print passing URLs summary
    if pass_count > 0:
        print(f"\n✅ {pass_count} URLs validated successfully\n")

    # Print skipped URLs summary
    if skip_count > 0:
        print(f"\n⏭️  {skip_count} URLs skipped (templates/placeholders)\n")

    # Summary
    print("\n=== Summary ===")
    print(f"Total URLs found: {len(all_urls)}")
    print(f"Unique URLs: {len(unique_urls)}")
    print(f"✅ Passed: {pass_count}")
    print(f"❌ Failed: {fail_count}")
    print(f"⏭️  Skipped: {skip_count}")

    # Exit with error if any failures
    if fail_count > 0:
        print(f"\n⚠️  {fail_count} URL(s) failed validation!")
        sys.exit(1)
    else:
        print("\n✅ All URLs validated successfully!")
        sys.exit(0)


if __name__ == "__main__":
    main()
