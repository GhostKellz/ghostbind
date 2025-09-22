# zbuild Integration Guide

How to integrate Ghostbind with [zbuild](https://github.com/ghostkellz/zbuild) for seamless Rust → Zig builds.

## Overview

zbuild is a Zig build orchestrator that works with Ghostbind to automatically:
1. Build Rust crates with proper targets
2. Generate C headers
3. Link artifacts into Zig projects
4. Handle cross-compilation

## Setup

### Prerequisites

1. Install zbuild:
```bash
git clone https://github.com/ghostkellz/zbuild
# Follow zbuild installation instructions
```

2. Install ghostbind:
```bash
cargo install --git https://github.com/ghostkellz/ghostbind
```

## Integration Approaches

### Approach 1: Direct Manifest Reading

Read the Ghostbind manifest directly in your `build.zig`:

```zig
const std = @import("std");
const json = std.json;

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const exe = b.addExecutable(.{
        .name = "my_app",
        .root_source_file = .{ .path = "src/main.zig" },
        .target = target,
        .optimize = optimize,
    });

    // First, run ghostbind to build the Rust crate
    const ghostbind_cmd = b.addSystemCommand(&.{
        "ghostbind", "build",
        "--manifest-path", "rust_lib/Cargo.toml",
        "--zig-target", target.zigTriple(),
    });

    // Read the generated manifest
    const manifest_path = "rust_lib/.ghostbind/cache/manifest.json";
    const manifest_file = std.fs.cwd().openFile(manifest_path, .{}) catch |err| {
        std.log.err("Failed to open manifest: {}", .{err});
        return;
    };
    defer manifest_file.close();

    const manifest_contents = manifest_file.readToEndAlloc(b.allocator, 1024 * 1024) catch |err| {
        std.log.err("Failed to read manifest: {}", .{err});
        return;
    };

    const manifest = json.parseFromSlice(
        GhostbindManifest,
        b.allocator,
        manifest_contents,
        .{},
    ) catch |err| {
        std.log.err("Failed to parse manifest: {}", .{err});
        return;
    };

    // Link the Rust library
    exe.addObjectFile(.{ .path = manifest.value.artifact });

    // Add headers
    const header_dir = std.fs.path.dirname(manifest.value.headers[0]) orelse ".";
    exe.addIncludePath(.{ .path = header_dir });

    // Link system libraries
    for (manifest.value.link_libs) |lib| {
        exe.linkSystemLibrary(lib);
    }

    exe.step.dependOn(&ghostbind_cmd.step);
    b.installArtifact(exe);
}

const GhostbindManifest = struct {
    crate_name: []const u8,
    kind: []const u8,
    artifact: []const u8,
    headers: [][]const u8,
    rustc_target: []const u8,
    link_libs: [][]const u8,
    link_search: [][]const u8,
};
```

### Approach 2: zbuild Helper Module

Create a reusable helper module `ghostbind.zig`:

```zig
const std = @import("std");

pub const RustCrateOptions = struct {
    /// Path to Rust crate directory (containing Cargo.toml)
    path: []const u8,
    /// Rust package name (for workspaces)
    package: ?[]const u8 = null,
    /// Build profile
    profile: enum { debug, release } = .release,
    /// Features to enable
    features: []const []const u8 = &.{},
    /// Disable default features
    no_default_features: bool = false,
    /// Override Rust target (otherwise derived from Zig target)
    rust_target: ?[]const u8 = null,
    /// Link mode
    link_mode: enum { static, dynamic } = .static,
    /// Path to cbindgen config
    cbindgen_config: ?[]const u8 = null,
};

pub fn addRustCrate(
    b: *std.Build,
    artifact: *std.Build.Step.Compile,
    options: RustCrateOptions,
) void {
    const allocator = b.allocator;

    // Build ghostbind command
    var cmd = std.ArrayList([]const u8).init(allocator);
    defer cmd.deinit();

    cmd.append("ghostbind") catch unreachable;
    cmd.append("build") catch unreachable;
    cmd.append("--manifest-path") catch unreachable;

    const manifest_path = b.fmt("{s}/Cargo.toml", .{options.path});
    cmd.append(manifest_path) catch unreachable;

    // Map Zig target to Rust target
    if (options.rust_target) |rust_target| {
        cmd.append("--rust-target") catch unreachable;
        cmd.append(rust_target) catch unreachable;
    } else {
        const zig_target = artifact.target_info.target;
        cmd.append("--zig-target") catch unreachable;
        cmd.append(zig_target.zigTriple()) catch unreachable;
    }

    // Add profile
    cmd.append("--profile") catch unreachable;
    cmd.append(@tagName(options.profile)) catch unreachable;

    // Add features
    if (options.features.len > 0) {
        cmd.append("--features") catch unreachable;
        const features = std.mem.join(allocator, ",", options.features) catch unreachable;
        cmd.append(features) catch unreachable;
    }

    if (options.no_default_features) {
        cmd.append("--no-default-features") catch unreachable;
    }

    if (options.cbindgen_config) |config| {
        cmd.append("--cbindgen-config") catch unreachable;
        cmd.append(config) catch unreachable;
    }

    // Run ghostbind
    const ghostbind_step = b.addSystemCommand(cmd.items);
    artifact.step.dependOn(&ghostbind_step.step);

    // Parse manifest and link
    // (In a real implementation, this would read the manifest file)
    linkRustArtifacts(b, artifact, options);
}

fn linkRustArtifacts(
    b: *std.Build,
    artifact: *std.Build.Step.Compile,
    options: RustCrateOptions,
) void {
    // Construct expected paths based on ghostbind conventions
    const cache_dir = b.fmt("{s}/.ghostbind/cache", .{options.path});

    // Add library
    const lib_name = std.fs.path.basename(options.path);
    const lib_ext = switch (options.link_mode) {
        .static => if (builtin.os.tag == .windows) ".lib" else ".a",
        .dynamic => switch (builtin.os.tag) {
            .windows => ".dll",
            .macos => ".dylib",
            else => ".so",
        },
    };

    const lib_path = b.fmt("{s}/{s}/lib{s}{s}", .{
        cache_dir,
        @tagName(options.profile),
        lib_name,
        lib_ext,
    });

    artifact.addObjectFile(.{ .path = lib_path });

    // Add headers
    const header_path = b.fmt("{s}/headers", .{cache_dir});
    artifact.addIncludePath(.{ .path = header_path });

    // Link common system libraries based on target
    const target = artifact.target_info.target;
    if (target.os.tag == .linux) {
        artifact.linkSystemLibrary("pthread");
        artifact.linkSystemLibrary("dl");
        artifact.linkSystemLibrary("m");
        artifact.linkSystemLibrary("c");
    } else if (target.os.tag == .macos) {
        artifact.linkSystemLibrary("System");
        artifact.linkSystemLibrary("pthread");
        artifact.linkSystemLibrary("c");
    } else if (target.os.tag == .windows) {
        artifact.linkSystemLibrary("kernel32");
        artifact.linkSystemLibrary("user32");
        artifact.linkSystemLibrary("shell32");
    }
}
```

Use it in your `build.zig`:

```zig
const std = @import("std");
const ghostbind = @import("ghostbind.zig");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const exe = b.addExecutable(.{
        .name = "my_app",
        .root_source_file = .{ .path = "src/main.zig" },
        .target = target,
        .optimize = optimize,
    });

    // Add Rust crate - it's this simple!
    ghostbind.addRustCrate(b, exe, .{
        .path = "rust_lib",
        .profile = .release,
        .features = &.{ "async", "parallel" },
    });

    b.installArtifact(exe);
}
```

### Approach 3: Build Step Integration

Create a custom build step for more control:

```zig
const GhostbindStep = struct {
    step: std.Build.Step,
    builder: *std.Build,
    crate_path: []const u8,
    manifest_path: []const u8,
    options: RustCrateOptions,

    pub fn create(b: *std.Build, options: RustCrateOptions) *GhostbindStep {
        const self = b.allocator.create(GhostbindStep) catch unreachable;
        self.* = .{
            .step = std.Build.Step.init(.{
                .id = .custom,
                .name = "ghostbind",
                .owner = b,
                .makeFn = make,
            }),
            .builder = b,
            .crate_path = options.path,
            .manifest_path = b.fmt("{s}/.ghostbind/cache/manifest.json", .{options.path}),
            .options = options,
        };
        return self;
    }

    fn make(step: *std.Build.Step, _: *std.Progress.Node) !void {
        const self = @fieldParentPtr(GhostbindStep, "step", step);

        // Run ghostbind
        var cmd = std.ArrayList([]const u8).init(self.builder.allocator);
        defer cmd.deinit();

        try cmd.append("ghostbind");
        try cmd.append("build");
        // ... add options ...

        const result = try std.ChildProcess.exec(.{
            .allocator = self.builder.allocator,
            .argv = cmd.items,
        });

        if (result.term.Exited != 0) {
            std.log.err("ghostbind failed: {s}", .{result.stderr});
            return error.GhostbindFailed;
        }

        // Manifest is now ready for consumption
    }
};
```

## Working with Workspaces

For Rust workspaces with multiple crates:

```zig
pub fn build(b: *std.Build) void {
    const exe = b.addExecutable(.{
        .name = "app",
        // ...
    });

    // Add multiple crates from a workspace
    ghostbind.addRustCrate(b, exe, .{
        .path = "rust_workspace",
        .package = "crate_a",
    });

    ghostbind.addRustCrate(b, exe, .{
        .path = "rust_workspace",
        .package = "crate_b",
        .features = &.{"experimental"},
    });
}
```

## Cross-Compilation

zbuild + Ghostbind makes cross-compilation seamless:

```zig
pub fn build(b: *std.Build) void {
    // Define multiple targets
    const targets = [_]std.zig.CrossTarget{
        .{ .cpu_arch = .x86_64, .os_tag = .linux },
        .{ .cpu_arch = .aarch64, .os_tag = .linux },
        .{ .cpu_arch = .x86_64, .os_tag = .windows },
        .{ .cpu_arch = .aarch64, .os_tag = .macos },
    };

    for (targets) |target| {
        const exe = b.addExecutable(.{
            .name = b.fmt("app-{s}", .{target.zigTriple()}),
            .root_source_file = .{ .path = "src/main.zig" },
            .target = target,
            .optimize = .ReleaseSafe,
        });

        ghostbind.addRustCrate(b, exe, .{
            .path = "rust_lib",
            .profile = .release,
            // Target is automatically mapped from Zig to Rust
        });

        b.installArtifact(exe);
    }
}
```

## Caching

Ghostbind caches build artifacts. To integrate with Zig's cache:

```zig
const cache_dir = b.cache_root.join(b.allocator, &.{"ghostbind"}) catch unreachable;

ghostbind.addRustCrate(b, exe, .{
    .path = "rust_lib",
    .cache_dir = cache_dir, // Custom cache location
});
```

## Advanced Configuration

### Custom Build Commands

```zig
// Run ghostbind with custom environment
const ghostbind_step = b.addSystemCommand(&.{"ghostbind", "build"});
ghostbind_step.setEnvironmentVariable("RUST_LOG", "debug");
ghostbind_step.setEnvironmentVariable("CARGO_TARGET_DIR", "/tmp/custom");
```

### Conditional Compilation

```zig
const features = std.ArrayList([]const u8).init(b.allocator);
defer features.deinit();

if (b.option(bool, "enable-gpu", "Enable GPU support") orelse false) {
    try features.append("gpu");
}

if (target.os.tag == .linux) {
    try features.append("linux-specific");
}

ghostbind.addRustCrate(b, exe, .{
    .path = "rust_lib",
    .features = features.items,
});
```

### Build Dependencies

```zig
// Ensure Rust crate is built before other steps
const rust_step = ghostbind.createBuildStep(b, .{
    .path = "rust_lib",
});

const test_step = b.addTest(.{
    .root_source_file = .{ .path = "src/tests.zig" },
});
test_step.step.dependOn(&rust_step.step);
```

## Troubleshooting

### "Manifest not found"
Ensure ghostbind build completes before reading the manifest.

### "Undefined symbols"
Check that all required system libraries are linked. Review the manifest's `link_libs` field.

### "Header not found"
Verify the header path in the manifest and add it to include paths.

### Performance Issues
- Use `--profile release` for production builds
- Enable LTO in Cargo.toml for smaller binaries
- Consider using `cdylib` instead of `staticlib` for shared libraries

## Best Practices

1. **Version Lock**: Commit `Cargo.lock` for reproducible builds
2. **Cache Artifacts**: Use Zig's cache system for Rust artifacts
3. **Parallel Builds**: Build multiple Rust crates in parallel when possible
4. **Feature Flags**: Use Zig options to control Rust features
5. **Error Handling**: Always check ghostbind exit codes

## Example Project Structure

```
my_project/
├── build.zig
├── src/
│   └── main.zig
├── rust_lib/
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── cbindgen.toml
│   └── src/
│       └── lib.rs
└── .ghostbind/
    └── cache/
        ├── x86_64-unknown-linux-gnu/
        │   ├── release/
        │   │   └── librust_lib.a
        │   ├── headers/
        │   │   └── rust_lib.h
        │   └── manifest.json
        └── aarch64-apple-darwin/
            └── ...
```

## Next Steps

- Explore [Cross-Compilation Guide](CROSS_COMPILATION.md)
- Read [FFI Safety Guidelines](FFI_SAFETY.md)
- Check out [complete examples](../examples/)