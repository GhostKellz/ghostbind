# Ghostbind Quickstart Guide

Get up and running with Ghostbind in 5 minutes!

## Installation

```bash
# 1. Install prerequisites
cargo install cbindgen

# 2. Install ghostbind
cargo install --git https://github.com/ghostkellz/ghostbind
```

## Your First Rust → Zig FFI Bridge

### Step 1: Create a Rust Library

Create a new Rust project or use an existing one:

```bash
cargo new --lib my_rust_lib
cd my_rust_lib
```

Edit `Cargo.toml`:

```toml
[package]
name = "my_rust_lib"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["staticlib", "cdylib"]

[dependencies]
# Add your dependencies here
```

Edit `src/lib.rs`:

```rust
use std::ffi::{c_char, CString};

/// A simple greeting function
#[unsafe(no_mangle)]
pub extern "C" fn greet(name: *const c_char) -> *mut c_char {
    let name = unsafe {
        if name.is_null() {
            return std::ptr::null_mut();
        }
        std::ffi::CStr::from_ptr(name)
    };

    let name_str = name.to_str().unwrap_or("unknown");
    let greeting = format!("Hello, {}! From Rust with ❤️", name_str);

    CString::new(greeting)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

/// Free a string allocated by Rust
#[unsafe(no_mangle)]
pub extern "C" fn free_rust_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

/// Perform some calculation
#[unsafe(no_mangle)]
pub extern "C" fn calculate(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}
```

### Step 2: Build with Ghostbind

```bash
# Build the Rust library and generate FFI artifacts
ghostbind build

# Or with custom options
ghostbind build --profile release --features "async,parallel"
```

This creates:
- `.ghostbind/cache/` - Contains compiled libraries and headers
- `.ghostbind/cache/*/my_rust_lib-manifest.json` - Build manifest

### Step 3: Create a Zig Project

Create `main.zig`:

```zig
const std = @import("std");

// Import the generated header
const rust = @cImport({
    @cInclude("my_rust_lib.h");
});

pub fn main() !void {
    // Call Rust function
    const name = "Zig Developer";
    const c_name = @ptrCast([*c]const u8, name);

    const greeting = rust.greet(c_name);
    defer rust.free_rust_string(greeting);

    if (greeting != null) {
        std.debug.print("{s}\n", .{greeting});
    }

    // Use calculation function
    const result = rust.calculate(3.0, 4.0);
    std.debug.print("Distance: {d}\n", .{result});
}
```

Create `build.zig`:

```zig
const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const exe = b.addExecutable(.{
        .name = "my_app",
        .root_source_file = .{ .path = "main.zig" },
        .target = target,
        .optimize = optimize,
    });

    // Read ghostbind manifest
    const manifest = @embedFile("my_rust_lib/.ghostbind/cache/native/my_rust_lib-manifest.json");
    const parsed = std.json.parse(manifest) catch @panic("Invalid manifest");

    // Link Rust library
    exe.addObjectFile(.{ .path = parsed.artifact });
    exe.addIncludePath(.{ .path = std.fs.path.dirname(parsed.headers[0]) });

    // Link system libraries
    for (parsed.link_libs) |lib| {
        exe.linkSystemLibrary(lib);
    }

    b.installArtifact(exe);
}
```

### Step 4: Build and Run

```bash
zig build run
```

Output:
```
Hello, Zig Developer! From Rust with ❤️
Distance: 5.0
```

## Common Patterns

### Passing Structs

Rust:
```rust
#[repr(C)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[unsafe(no_mangle)]
pub extern "C" fn distance(p1: Point, p2: Point) -> f64 {
    let dx = p1.x - p2.x;
    let dy = p1.y - p2.y;
    (dx * dx + dy * dy).sqrt()
}
```

Zig:
```zig
const point1 = rust.Point{ .x = 0.0, .y = 0.0 };
const point2 = rust.Point{ .x = 3.0, .y = 4.0 };
const dist = rust.distance(point1, point2);
```

### Arrays and Slices

Rust:
```rust
#[unsafe(no_mangle)]
pub extern "C" fn sum_array(arr: *const i32, len: usize) -> i32 {
    if arr.is_null() || len == 0 {
        return 0;
    }

    let slice = unsafe { std::slice::from_raw_parts(arr, len) };
    slice.iter().sum()
}
```

Zig:
```zig
const numbers = [_]i32{ 1, 2, 3, 4, 5 };
const sum = rust.sum_array(&numbers[0], numbers.len);
```

### Error Handling

Rust:
```rust
#[repr(C)]
pub struct Result {
    pub success: bool,
    pub value: f64,
    pub error_msg: *const c_char,
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_divide(a: f64, b: f64) -> Result {
    if b == 0.0 {
        let msg = CString::new("Division by zero").unwrap();
        Result {
            success: false,
            value: 0.0,
            error_msg: msg.into_raw(),
        }
    } else {
        Result {
            success: true,
            value: a / b,
            error_msg: std::ptr::null(),
        }
    }
}
```

## Tips & Best Practices

1. **Always use `#[repr(C)]`** for structs that cross FFI boundaries
2. **Handle null pointers** in Rust functions that accept pointers
3. **Provide cleanup functions** for any memory allocated by Rust
4. **Use `extern "C"` and `#[unsafe(no_mangle)]`** for all exported functions
5. **Keep FFI surface small** - wrap complex Rust APIs in simple C-compatible functions

## Next Steps

- Read [FFI Safety Guidelines](FFI_SAFETY.md) for production code
- Learn about [Cross Compilation](CROSS_COMPILATION.md)
- Integrate with [zbuild](ZBUILD_INTEGRATION.md) for automated builds
- Check out [complete examples](../examples/)

## Troubleshooting

### "cbindgen not found"
```bash
cargo install cbindgen
```

### "No library artifacts found"
Ensure your `Cargo.toml` includes:
```toml
[lib]
crate-type = ["staticlib"]  # or ["staticlib", "cdylib"]
```

### Linking errors in Zig
Check the manifest JSON to ensure all required system libraries are linked.

### Can't find headers
Headers are generated in `.ghostbind/cache/[target]/headers/`. Add this to your include path.