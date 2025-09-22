<div align="center">
  <img src="assets/icons/ghostbind.png" alt="Ghostbind Logo" width="200">

  # Ghostbind 👻🔗

  > A tiny, predictable build/FFI bridge that lets **Zig** (via [`zbuild`](https://github.com/ghostkellz/zbuild)) consume **Rust** crates, auto-generate headers, and link artifacts across targets — with reproducible, one-command builds.

  [![License: MIT/Apache-2.0](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](LICENSE)
  [![Rust 2024](https://img.shields.io/badge/Rust-2024%20Edition-orange.svg)](https://blog.rust-lang.org/2024/01/01/Rust-2024.html)
</div>

## What is Ghostbind?

Ghostbind bridges the gap between Zig and Rust, making it seamless to use Rust crates in Zig projects. It handles:

- 🎯 **Target mapping** - Automatically translates Zig targets to Rust targets
- 📦 **Cargo integration** - Builds Rust crates with proper profiles and features
- 🔍 **Artifact discovery** - Finds and caches compiled libraries (.a, .so, .dylib, .dll)
- 📄 **Header generation** - Auto-generates C headers using cbindgen
- 📋 **Build manifests** - Outputs JSON metadata for build system integration
- 🔄 **Cross-compilation** - Full support for cross-platform builds

## Installation

### Prerequisites

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install cbindgen for C header generation
cargo install cbindgen

# Clone and build ghostbind
git clone https://github.com/ghostkellz/ghostbind.git
cd ghostbind
cargo build --release

# Add to PATH (optional)
cargo install --path .
```

## Quick Start

### 1. Create a Rust library with C FFI exports

```rust
// src/lib.rs
#[unsafe(no_mangle)]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

```toml
# Cargo.toml
[package]
name = "my_math"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["staticlib", "cdylib"]
```

### 2. Build with Ghostbind

```bash
ghostbind build --manifest-path Cargo.toml
```

This generates:
- Compiled library in `.ghostbind/cache/`
- C header in `.ghostbind/cache/headers/`
- Build manifest JSON for tooling integration

### 3. Use in your Zig project

The generated manifest provides all information needed to link the Rust library:

```json
{
  "crate_name": "my_math",
  "kind": "staticlib",
  "artifact": "/path/to/libmy_math.a",
  "headers": ["/path/to/my_math.h"],
  "rustc_target": "x86_64-unknown-linux-gnu",
  "link_libs": ["pthread", "dl", "m", "c"]
}
```

## CLI Usage

```bash
# Build a Rust crate for FFI
ghostbind build [OPTIONS]

# Generate headers only (assumes crate is already built)
ghostbind headers [OPTIONS]

# Check system requirements and configuration
ghostbind doctor
```

### Build Options

```
--manifest-path <PATH>       Path to Cargo.toml [default: Cargo.toml]
--zig-target <TARGET>        Zig target triple (will be mapped to Rust)
--rust-target <TARGET>       Override Rust target (bypasses mapping)
--profile <PROFILE>          Build profile [default: release]
--features <FEATURES>        Comma-separated features to enable
--no-default-features        Disable default features
--cbindgen-config <PATH>     Path to cbindgen.toml config
--generate-cbindgen-config   Generate default cbindgen config
```

## zbuild Integration

Ghostbind is designed to work seamlessly with [zbuild](https://github.com/ghostkellz/zbuild). In your `build.zig`:

```zig
const ghostbind = @import("ghostbind");

pub fn build(b: *std.Build) void {
    const exe = b.addExecutable(.{
        .name = "my-app",
        .root_source_file = .{ .path = "src/main.zig" },
        .target = target,
    });

    // Add Rust crate via ghostbind
    ghostbind.addRustCrate(b, exe, .{
        .path = "rust_lib",
        .profile = .release,
        .features = &.{"async"},
    });

    b.installArtifact(exe);
}
```

## Documentation

- [**Quickstart Guide**](docs/QUICKSTART.md) - Get up and running in 5 minutes
- [**FFI Safety**](docs/FFI_SAFETY.md) - Best practices for Rust ↔ Zig FFI
- [**Cross Compilation**](docs/CROSS_COMPILATION.md) - Building for different targets
- [**API Reference**](docs/API.md) - Detailed API documentation
- [**zbuild Integration**](docs/ZBUILD_INTEGRATION.md) - Using with zbuild

## Examples

Check out the [`examples/`](examples/) directory for complete working examples:

- `test_crate/` - Basic Rust library with FFI exports
- `zig-rust-ffi/` - Complete Zig + Rust integration (coming soon)

## Target Support

Ghostbind supports automatic target mapping between Zig and Rust:

| Zig Target | Rust Target |
|------------|-------------|
| `x86_64-linux-gnu` | `x86_64-unknown-linux-gnu` |
| `aarch64-linux-gnu` | `aarch64-unknown-linux-gnu` |
| `x86_64-macos` | `x86_64-apple-darwin` |
| `aarch64-macos` | `aarch64-apple-darwin` |
| `x86_64-windows-msvc` | `x86_64-pc-windows-msvc` |

See [full target mapping list](docs/TARGETS.md).

## Architecture

```
Zig Project
     ↓
  zbuild
     ↓
 Ghostbind ← → Cargo/Rust
     ↓
 Manifest JSON
     ↓
Linked Binary
```

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Dual licensed under:
- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

Choose whichever license works best for your project.

## Related Projects

- [zbuild](https://github.com/ghostkellz/zbuild) - Zig build orchestrator
- [cbindgen](https://github.com/mozilla/cbindgen) - Generate C bindings from Rust code

## Roadmap

- [x] MVP (v0.1) - Basic Rust → Zig FFI bridge
- [ ] v0.2 - Advanced cross-compilation and caching
- [ ] v0.3 - Enhanced tooling and reproducible builds
- [ ] v1.0 - Production ready with stable API

See [TODO.md](TODO.md) for detailed roadmap.

## Authors

Created and maintained by [@ghostkellz](https://github.com/ghostkellz)

---

*Ghostbind: Because FFI shouldn't be frightening* 👻
