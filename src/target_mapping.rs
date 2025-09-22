use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TargetMapping {
    zig_to_rust: HashMap<String, String>,
}

impl TargetMapping {
    pub fn new() -> Self {
        let mut mapping = HashMap::new();

        // Linux targets
        mapping.insert("x86_64-linux-gnu".to_string(), "x86_64-unknown-linux-gnu".to_string());
        mapping.insert("x86_64-linux-musl".to_string(), "x86_64-unknown-linux-musl".to_string());
        mapping.insert("aarch64-linux-gnu".to_string(), "aarch64-unknown-linux-gnu".to_string());
        mapping.insert("aarch64-linux-musl".to_string(), "aarch64-unknown-linux-musl".to_string());
        mapping.insert("i386-linux-gnu".to_string(), "i686-unknown-linux-gnu".to_string());

        // macOS targets
        mapping.insert("x86_64-macos".to_string(), "x86_64-apple-darwin".to_string());
        mapping.insert("aarch64-macos".to_string(), "aarch64-apple-darwin".to_string());

        // Windows targets
        mapping.insert("x86_64-windows-gnu".to_string(), "x86_64-pc-windows-gnu".to_string());
        mapping.insert("x86_64-windows-msvc".to_string(), "x86_64-pc-windows-msvc".to_string());
        mapping.insert("i386-windows-gnu".to_string(), "i686-pc-windows-gnu".to_string());
        mapping.insert("i386-windows-msvc".to_string(), "i686-pc-windows-msvc".to_string());
        mapping.insert("aarch64-windows".to_string(), "aarch64-pc-windows-msvc".to_string());

        // FreeBSD targets
        mapping.insert("x86_64-freebsd".to_string(), "x86_64-unknown-freebsd".to_string());

        Self {
            zig_to_rust: mapping,
        }
    }

    pub fn map_target(&self, zig_target: &str) -> Option<&str> {
        self.zig_to_rust.get(zig_target).map(|s| s.as_str())
    }

    pub fn map_target_or_default(&self, zig_target: &str) -> String {
        self.map_target(zig_target)
            .unwrap_or(zig_target)
            .to_string()
    }

    pub fn supported_targets(&self) -> Vec<&str> {
        self.zig_to_rust.keys().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_mappings() {
        let mapping = TargetMapping::new();

        assert_eq!(
            mapping.map_target("x86_64-linux-gnu"),
            Some("x86_64-unknown-linux-gnu")
        );

        assert_eq!(
            mapping.map_target("aarch64-macos"),
            Some("aarch64-apple-darwin")
        );

        assert_eq!(
            mapping.map_target("x86_64-windows-msvc"),
            Some("x86_64-pc-windows-msvc")
        );
    }

    #[test]
    fn test_unknown_target() {
        let mapping = TargetMapping::new();
        assert_eq!(mapping.map_target("unknown-target"), None);
        assert_eq!(mapping.map_target_or_default("unknown-target"), "unknown-target");
    }
}