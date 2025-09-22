use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::artifact_discovery::DiscoveredArtifact;
use crate::header_generation::GeneratedHeader;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildManifest {
    pub crate_name: String,
    pub kind: String,
    pub artifact: PathBuf,
    pub headers: Vec<PathBuf>,
    pub rustc_target: String,
    pub link_libs: Vec<String>,
    pub link_search: Vec<PathBuf>,
}

pub struct ManifestGenerator {
    cache_dir: PathBuf,
}

impl ManifestGenerator {
    pub fn new() -> Self {
        Self {
            cache_dir: PathBuf::from(".ghostbind/cache"),
        }
    }

    pub fn generate_manifest(
        &self,
        crate_name: &str,
        artifact: &DiscoveredArtifact,
        headers: &[GeneratedHeader],
        rustc_target: &str,
    ) -> Result<BuildManifest> {
        let manifest = BuildManifest {
            crate_name: crate_name.to_string(),
            kind: artifact.kind.as_str().to_string(),
            artifact: artifact.cached_path.clone(),
            headers: headers.iter().map(|h| h.header_path.clone()).collect(),
            rustc_target: rustc_target.to_string(),
            link_libs: self.get_system_link_libs(rustc_target),
            link_search: Vec::new(), // Will be populated later if needed
        };

        Ok(manifest)
    }

    pub fn write_manifest(
        &self,
        manifest: &BuildManifest,
        target_triple: Option<&str>,
    ) -> Result<PathBuf> {
        let manifest_path = self.get_manifest_path(&manifest.crate_name, target_triple);

        // Create cache directory
        if let Some(manifest_parent) = manifest_path.parent() {
            fs::create_dir_all(manifest_parent)
                .with_context(|| format!("Failed to create manifest directory: {}", manifest_parent.display()))?;
        }

        // Serialize manifest to JSON
        let manifest_json = serde_json::to_string_pretty(manifest)
            .context("Failed to serialize manifest to JSON")?;

        // Write to file
        fs::write(&manifest_path, manifest_json)
            .with_context(|| format!("Failed to write manifest to {}", manifest_path.display()))?;

        println!("Generated manifest: {}", manifest_path.display());

        Ok(manifest_path)
    }

    fn get_manifest_path(&self, crate_name: &str, target_triple: Option<&str>) -> PathBuf {
        let target_str = target_triple.unwrap_or("native");

        self.cache_dir
            .join(target_str)
            .join(format!("{}-manifest.json", crate_name))
    }

    fn get_system_link_libs(&self, rustc_target: &str) -> Vec<String> {
        let mut libs = Vec::new();

        // Common system libraries that Rust often requires
        if rustc_target.contains("linux") {
            libs.extend_from_slice(&[
                "pthread".to_string(),
                "dl".to_string(),
                "m".to_string(), // math library
            ]);

            // Add additional libraries for musl targets
            if rustc_target.contains("musl") {
                // musl typically statically links these, but may need some
                libs.push("c".to_string());
            } else {
                // glibc targets
                libs.push("c".to_string());
            }
        } else if rustc_target.contains("darwin") || rustc_target.contains("macos") {
            libs.extend_from_slice(&[
                "System".to_string(),
                "pthread".to_string(),
                "c".to_string(),
            ]);
        } else if rustc_target.contains("windows") {
            libs.extend_from_slice(&[
                "kernel32".to_string(),
                "user32".to_string(),
                "shell32".to_string(),
                "msvcrt".to_string(),
            ]);

            if rustc_target.contains("msvc") {
                libs.extend_from_slice(&[
                    "vcruntime".to_string(),
                    "ucrt".to_string(),
                ]);
            }
        } else if rustc_target.contains("freebsd") {
            libs.extend_from_slice(&[
                "pthread".to_string(),
                "c".to_string(),
                "m".to_string(),
            ]);
        }

        libs
    }

    pub fn read_manifest(&self, manifest_path: &Path) -> Result<BuildManifest> {
        let manifest_content = fs::read_to_string(manifest_path)
            .with_context(|| format!("Failed to read manifest from {}", manifest_path.display()))?;

        let manifest: BuildManifest = serde_json::from_str(&manifest_content)
            .with_context(|| format!("Failed to parse manifest JSON from {}", manifest_path.display()))?;

        Ok(manifest)
    }

    pub fn validate_manifest(&self, manifest: &BuildManifest) -> Result<()> {
        // Check that the artifact exists
        if !manifest.artifact.exists() {
            return Err(anyhow::anyhow!(
                "Artifact file does not exist: {}",
                manifest.artifact.display()
            ));
        }

        // Check that header files exist
        for header in &manifest.headers {
            if !header.exists() {
                return Err(anyhow::anyhow!(
                    "Header file does not exist: {}",
                    header.display()
                ));
            }
        }

        Ok(())
    }
}

impl Default for ManifestGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact_discovery::{ArtifactKind, DiscoveredArtifact};
    use crate::header_generation::GeneratedHeader;

    #[test]
    fn test_manifest_generation() {
        let generator = ManifestGenerator::new();

        let artifact = DiscoveredArtifact {
            name: "test_crate".to_string(),
            kind: ArtifactKind::StaticLib,
            original_path: PathBuf::from("/tmp/libtest_crate.a"),
            cached_path: PathBuf::from(".ghostbind/cache/native/release/test_crate.a"),
        };

        let headers = vec![GeneratedHeader {
            crate_name: "test_crate".to_string(),
            header_path: PathBuf::from(".ghostbind/cache/native/headers/test_crate.h"),
        }];

        let manifest = generator.generate_manifest(
            "test_crate",
            &artifact,
            &headers,
            "x86_64-unknown-linux-gnu",
        ).unwrap();

        assert_eq!(manifest.crate_name, "test_crate");
        assert_eq!(manifest.kind, "staticlib");
        assert_eq!(manifest.rustc_target, "x86_64-unknown-linux-gnu");
        assert!(manifest.link_libs.contains(&"pthread".to_string()));
        assert!(manifest.link_libs.contains(&"dl".to_string()));
    }

    #[test]
    fn test_system_link_libs() {
        let generator = ManifestGenerator::new();

        let linux_libs = generator.get_system_link_libs("x86_64-unknown-linux-gnu");
        assert!(linux_libs.contains(&"pthread".to_string()));
        assert!(linux_libs.contains(&"dl".to_string()));

        let windows_libs = generator.get_system_link_libs("x86_64-pc-windows-msvc");
        assert!(windows_libs.contains(&"kernel32".to_string()));
        assert!(windows_libs.contains(&"vcruntime".to_string()));

        let macos_libs = generator.get_system_link_libs("x86_64-apple-darwin");
        assert!(macos_libs.contains(&"System".to_string()));
        assert!(macos_libs.contains(&"pthread".to_string()));
    }
}