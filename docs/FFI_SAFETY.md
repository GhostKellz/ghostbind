# FFI Safety Guidelines

Essential safety rules and best practices for Rust ↔ Zig FFI.

## Core Principles

### 1. Memory Safety

**Rule #1: Never forget who owns the memory**

- **Rust allocates → Rust frees**: Always provide a corresponding `free` function
- **Zig allocates → Zig frees**: Never free Zig-allocated memory in Rust
- **Document ownership**: Be explicit about memory ownership in comments

### 2. ABI Compatibility

**Always use `#[repr(C)]` for FFI structs:**

```rust
// ✅ CORRECT - C-compatible layout
#[repr(C)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

// ❌ WRONG - Rust layout not guaranteed
pub struct Point {
    pub x: f64,
    pub y: f64,
}
```

**Use `extern "C"` for functions:**

```rust
// ✅ CORRECT
#[unsafe(no_mangle)]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a + b
}

// ❌ WRONG - Rust ABI
#[unsafe(no_mangle)]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

## Safe Patterns

### String Handling

**Pattern: Rust String → C String**

```rust
use std::ffi::{c_char, CString};

#[unsafe(no_mangle)]
pub extern "C" fn get_message() -> *mut c_char {
    let message = "Hello from Rust";
    CString::new(message)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

#[unsafe(no_mangle)]
pub extern "C" fn free_message(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}
```

**Pattern: C String → Rust String**

```rust
use std::ffi::{c_char, CStr};

#[unsafe(no_mangle)]
pub extern "C" fn process_string(input: *const c_char) -> i32 {
    if input.is_null() {
        return -1;
    }

    let c_str = unsafe { CStr::from_ptr(input) };

    match c_str.to_str() {
        Ok(s) => s.len() as i32,
        Err(_) => -1,
    }
}
```

### Error Handling

**Pattern: Result Type**

```rust
#[repr(C)]
pub struct FfiResult {
    pub success: bool,
    pub value: f64,
    pub error_code: i32,
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_sqrt(x: f64) -> FfiResult {
    if x < 0.0 {
        FfiResult {
            success: false,
            value: 0.0,
            error_code: 1, // INVALID_INPUT
        }
    } else {
        FfiResult {
            success: true,
            value: x.sqrt(),
            error_code: 0,
        }
    }
}
```

**Pattern: Error Codes**

```rust
#[repr(C)]
pub enum ErrorCode {
    Success = 0,
    InvalidInput = 1,
    OutOfMemory = 2,
    NotFound = 3,
}

#[unsafe(no_mangle)]
pub extern "C" fn operation() -> ErrorCode {
    // ... perform operation ...
    ErrorCode::Success
}
```

### Collections

**Pattern: Arrays/Slices**

```rust
#[unsafe(no_mangle)]
pub extern "C" fn process_array(
    data: *const f64,
    len: usize,
    out: *mut f64,
    out_len: usize,
) -> bool {
    if data.is_null() || out.is_null() || len == 0 || out_len < len {
        return false;
    }

    let input = unsafe { std::slice::from_raw_parts(data, len) };
    let output = unsafe { std::slice::from_raw_parts_mut(out, out_len) };

    for (i, &val) in input.iter().enumerate() {
        if i >= out_len {
            break;
        }
        output[i] = val * 2.0;
    }

    true
}
```

**Pattern: Dynamic Vectors**

```rust
#[repr(C)]
pub struct FfiVec {
    pub data: *mut f64,
    pub len: usize,
    pub capacity: usize,
}

#[unsafe(no_mangle)]
pub extern "C" fn create_vec(size: usize) -> FfiVec {
    let mut vec = Vec::with_capacity(size);
    vec.resize(size, 0.0);

    let data = vec.as_mut_ptr();
    let len = vec.len();
    let capacity = vec.capacity();

    std::mem::forget(vec); // Prevent Rust from freeing

    FfiVec { data, len, capacity }
}

#[unsafe(no_mangle)]
pub extern "C" fn free_vec(vec: FfiVec) {
    if !vec.data.is_null() {
        unsafe {
            let _ = Vec::from_raw_parts(vec.data, vec.len, vec.capacity);
        }
    }
}
```

### Callbacks

**Pattern: Function Pointers**

```rust
type Callback = extern "C" fn(i32) -> i32;

#[unsafe(no_mangle)]
pub extern "C" fn process_with_callback(
    data: *const i32,
    len: usize,
    callback: Callback,
) -> i32 {
    if data.is_null() || len == 0 {
        return 0;
    }

    let slice = unsafe { std::slice::from_raw_parts(data, len) };

    slice.iter().map(|&x| callback(x)).sum()
}
```

## Unsafe Patterns to Avoid

### ❌ Don't Use Rust-Specific Types

```rust
// ❌ WRONG - String is not FFI-safe
#[unsafe(no_mangle)]
pub extern "C" fn bad_function() -> String {
    "Don't do this".to_string()
}

// ✅ CORRECT - Use C-compatible types
#[unsafe(no_mangle)]
pub extern "C" fn good_function() -> *mut c_char {
    CString::new("Do this instead")
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
```

### ❌ Don't Panic Across FFI Boundaries

```rust
// ❌ WRONG - Panic will cause undefined behavior
#[unsafe(no_mangle)]
pub extern "C" fn may_panic(x: i32) -> i32 {
    assert!(x > 0); // This can panic!
    x * 2
}

// ✅ CORRECT - Handle errors gracefully
#[unsafe(no_mangle)]
pub extern "C" fn no_panic(x: i32) -> i32 {
    if x <= 0 {
        return -1; // Error code
    }
    x * 2
}
```

### ❌ Don't Use Non-Repr(C) Enums

```rust
// ❌ WRONG - Rust enum layout
pub enum Status {
    Ok,
    Error,
}

// ✅ CORRECT - C-compatible enum
#[repr(C)]
pub enum Status {
    Ok = 0,
    Error = 1,
}
```

## Thread Safety

### Pattern: Mutex-Protected State

```rust
use std::sync::Mutex;

static STATE: Mutex<Option<Vec<i32>>> = Mutex::new(None);

#[unsafe(no_mangle)]
pub extern "C" fn init_state(capacity: usize) -> bool {
    let mut state = STATE.lock().unwrap();
    *state = Some(Vec::with_capacity(capacity));
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn add_value(value: i32) -> bool {
    let mut state = STATE.lock().unwrap();
    if let Some(ref mut vec) = *state {
        vec.push(value);
        true
    } else {
        false
    }
}
```

## Validation Checklist

Before exposing a Rust function via FFI:

- [ ] All structs use `#[repr(C)]`
- [ ] All functions use `extern "C"` and `#[unsafe(no_mangle)]`
- [ ] No Rust-specific types in signatures (String, Vec, Option, Result)
- [ ] All pointers are checked for null
- [ ] Memory ownership is clearly documented
- [ ] Free functions provided for Rust-allocated memory
- [ ] No panics can occur
- [ ] Thread safety considered
- [ ] Error handling uses C-compatible patterns

## Testing FFI Code

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_function() {
        // Test with valid input
        let result = unsafe { add(2, 3) };
        assert_eq!(result, 5);

        // Test edge cases
        let result = unsafe { add(i32::MAX, 1) };
        // Handle overflow appropriately
    }

    #[test]
    fn test_null_handling() {
        let result = unsafe { process_string(std::ptr::null()) };
        assert_eq!(result, -1);
    }
}
```

## Debugging Tips

1. **Use `RUST_BACKTRACE=1`** to debug panics
2. **Add logging** at FFI boundaries
3. **Validate all inputs** before processing
4. **Use AddressSanitizer** to detect memory issues
5. **Test with Valgrind** for memory leaks

## Resources

- [Rust FFI Omnibus](http://jakegoulding.com/rust-ffi-omnibus/)
- [Rust Nomicon - FFI](https://doc.rust-lang.org/nomicon/ffi.html)
- [cbindgen User Guide](https://github.com/mozilla/cbindgen/blob/master/docs.md)