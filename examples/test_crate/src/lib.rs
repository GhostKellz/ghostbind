/// Add two numbers together
#[unsafe(no_mangle)]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// Multiply two numbers
#[unsafe(no_mangle)]
pub extern "C" fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

/// A simple struct to test FFI
#[repr(C)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// Calculate distance between two points
#[unsafe(no_mangle)]
pub extern "C" fn distance(p1: Point, p2: Point) -> f64 {
    let dx = p1.x - p2.x;
    let dy = p1.y - p2.y;
    (dx * dx + dy * dy).sqrt()
}

/// Free a string allocated by Rust
#[unsafe(no_mangle)]
pub extern "C" fn free_string(s: *mut std::os::raw::c_char) {
    if !s.is_null() {
        unsafe {
            let _ = std::ffi::CString::from_raw(s);
        }
    }
}