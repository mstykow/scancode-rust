# OSGi Manifest Parser: Bundle Metadata Extraction

## Summary

**✨ New Feature**: the Python reference only reaches OSGi manifest handling during assembly-oriented flows, while Rust can recognize OSGi bundles during ordinary file scanning and extract useful bundle metadata directly from the manifest.

## Reference limitation

In the Python reference, OSGi handling is effectively assembly-oriented. That means a manifest can exist in the scanned tree without producing the package data users would expect from a direct file scan.

## Rust behavior

Rust detects OSGi bundles from `META-INF/MANIFEST.MF` files when OSGi-specific headers are present, most importantly `Bundle-SymbolicName`.

When a bundle is recognized, Rust can extract:

- bundle identity from `Bundle-SymbolicName` and `Bundle-Version`
- human-facing description fields from `Bundle-Name` and `Bundle-Description`
- vendor information from `Bundle-Vendor`
- homepage information from `Bundle-DocURL`
- declared license information from `Bundle-License`
- dependency edges from `Import-Package` and `Require-Bundle`

OSGi version ranges are preserved in the extracted dependency requirements instead of being flattened into looser text.

## Why this matters

- **Automatic detection**: OSGi bundles are no longer invisible during regular scans
- **Better bundle metadata**: vendor, license, and homepage data can flow straight from the manifest into package output
- **Richer dependency visibility**: imported packages and required bundles show up as structured dependency edges

## Reference

- [OSGi Core Specification](https://docs.osgi.org/specification/osgi.core/7.0.0/framework.module.html)
