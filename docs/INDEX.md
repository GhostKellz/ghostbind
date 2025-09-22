# Ghostbind Documentation Index

Welcome to the Ghostbind documentation! Here's what's available:

## Getting Started
- [**README**](../README.md) - Project overview and installation
- [**Quickstart Guide**](QUICKSTART.md) - Get running in 5 minutes
- [**TODO/Roadmap**](../TODO.md) - Project roadmap and planned features

## Core Guides
- [**FFI Safety Guidelines**](FFI_SAFETY.md) - Essential safety rules for Rust ↔ Zig FFI
- [**zbuild Integration**](ZBUILD_INTEGRATION.md) - Using Ghostbind with zbuild
- [**Cross Compilation**](CROSS_COMPILATION.md) - Building for different targets *(coming soon)*
- [**API Reference**](API.md) - Detailed API documentation *(coming soon)*

## Examples
- [**test_crate**](../examples/test_crate/) - Basic Rust library with FFI exports
- More examples coming soon!

## Command Reference

### `ghostbind build`
Build a Rust crate and generate all FFI artifacts.

```bash
ghostbind build [OPTIONS]
```

Options:
- `--manifest-path <PATH>` - Path to Cargo.toml
- `--zig-target <TARGET>` - Zig target triple (auto-mapped to Rust)
- `--rust-target <TARGET>` - Override Rust target
- `--profile <debug|release>` - Build profile
- `--features <FEATURES>` - Comma-separated features
- `--no-default-features` - Disable default features
- `--cbindgen-config <PATH>` - Path to cbindgen config
- `--generate-cbindgen-config` - Generate default cbindgen config

### `ghostbind headers`
Generate C headers for an already-built Rust crate.

```bash
ghostbind headers --manifest-path <PATH>
```

### `ghostbind doctor`
Check system requirements and configuration.

```bash
ghostbind doctor
```

## Generated Artifacts

Ghostbind generates the following structure:

```
.ghostbind/
└── cache/
    └── <target>/
        ├── release/
        │   └── <crate_name>.a     # Static library
        ├── headers/
        │   └── <crate_name>.h      # C header
        └── <crate_name>-manifest.json  # Build manifest
```

## Manifest Format

The JSON manifest contains:

```json
{
  "crate_name": "string",        // Name of the Rust crate
  "kind": "staticlib|cdylib",    // Library type
  "artifact": "path/to/lib",     // Path to compiled library
  "headers": ["path/to/header"], // Generated header paths
  "rustc_target": "string",       // Rust target triple
  "link_libs": ["libs"],         // System libraries to link
  "link_search": ["paths"]       // Additional library search paths
}
```

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for contribution guidelines.

## Support

- [GitHub Issues](https://github.com/ghostkellz/ghostbind/issues)
- [GitHub Discussions](https://github.com/ghostkellz/ghostbind/discussions)

## License

Dual licensed under MIT/Apache-2.0. See [LICENSE](../LICENSE) for details.