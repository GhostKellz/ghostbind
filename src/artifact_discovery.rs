use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::cargo_integration::{BuildProfile, CrateInfo, TargetKind};

pub struct ArtifactDiscovery {
    target_dir: PathBuf,
    target_triple: Option<String>,
    profile: BuildProfile,
    cache_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct DiscoveredArtifact {
    pub name: String,
    pub kind: ArtifactKind,
    pub original_path: PathBuf,
    pub cached_path: PathBuf,
}

#[derive(Debug, Clone)]
pub enum ArtifactKind {
    StaticLib,
    DynamicLib,
}

impl ArtifactKind {
    pub fn from_target_kind(target_kind: &TargetKind) -> Option<Self> {
        match target_kind {
            TargetKind::StaticLib => Some(ArtifactKind::StaticLib),
            TargetKind::CdyLib => Some(ArtifactKind::DynamicLib),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            ArtifactKind::StaticLib => "staticlib",
            ArtifactKind::DynamicLib => "cdylib",
        }
    }
}

impl ArtifactDiscovery {
    pub fn new(
        target_dir: impl AsRef<Path>,
        target_triple: Option<String>,
        profile: BuildProfile,
    ) -> Self {
        let cache_dir = PathBuf::from(".ghostbind/cache");

        Self {
            target_dir: target_dir.as_ref().to_path_buf(),
            target_triple,
            profile,
            cache_dir,
        }
    }

    pub fn discover_artifacts(&self, crate_info: &CrateInfo) -> Result<Vec<DiscoveredArtifact>> {
        let mut artifacts = Vec::new();

        for target in &crate_info.targets {
            if !target.kind.is_library() {
                continue;
            }

            if let Some(artifact_kind) = ArtifactKind::from_target_kind(&target.kind) {
                let artifact = self.find_artifact(&target.name, &artifact_kind)?;
                artifacts.push(artifact);
            }
        }

        Ok(artifacts)
    }

    fn find_artifact(&self, crate_name: &str, kind: &ArtifactKind) -> Result<DiscoveredArtifact> {
        let build_dir = self.get_build_directory();
        let lib_patterns = self.get_library_patterns(crate_name, kind);

        for pattern in &lib_patterns {
            let artifact_path = build_dir.join(pattern);
            if artifact_path.exists() {
                let cached_path = self.get_cache_path(crate_name, kind);
                return Ok(DiscoveredArtifact {
                    name: crate_name.to_string(),
                    kind: kind.clone(),
                    original_path: artifact_path,
                    cached_path,
                });
            }
        }

        Err(anyhow!(
            "Could not find {} artifact for crate '{}' in {}",
            kind.as_str(),
            crate_name,
            build_dir.display()
        ))
    }

    fn get_build_directory(&self) -> PathBuf {
        let mut build_dir = self.target_dir.clone();

        if let Some(ref target) = self.target_triple {
            build_dir = build_dir.join(target);
        }

        build_dir.join(self.profile.as_str())
    }

    fn get_library_patterns(&self, crate_name: &str, kind: &ArtifactKind) -> Vec<String> {
        let normalized_name = crate_name.replace('-', "_");

        match kind {
            ArtifactKind::StaticLib => {
                if cfg!(target_os = "windows") {
                    vec![format!("{}.lib", normalized_name)]
                } else {
                    vec![format!("lib{}.a", normalized_name)]
                }
            }
            ArtifactKind::DynamicLib => {
                if cfg!(target_os = "windows") {
                    vec![
                        format!("{}.dll", normalized_name),
                        format!("{}.dll.lib", normalized_name),
                    ]
                } else if cfg!(target_os = "macos") {
                    vec![format!("lib{}.dylib", normalized_name)]
                } else {
                    vec![format!("lib{}.so", normalized_name)]
                }
            }
        }
    }

    fn get_cache_path(&self, crate_name: &str, kind: &ArtifactKind) -> PathBuf {
        let target_str = self.target_triple.as_deref().unwrap_or("native");

        self.cache_dir
            .join(target_str)
            .join(self.profile.as_str())
            .join(format!("{}.{}", crate_name, self.get_artifact_extension(kind)))
    }

    fn get_artifact_extension(&self, kind: &ArtifactKind) -> &str {
        match kind {
            ArtifactKind::StaticLib => {
                if cfg!(target_os = "windows") { "lib" } else { "a" }
            }
            ArtifactKind::DynamicLib => {
                if cfg!(target_os = "windows") {
                    "dll"
                } else if cfg!(target_os = "macos") {
                    "dylib"
                } else {
                    "so"
                }
            }
        }
    }

    pub fn cache_artifacts(&self, artifacts: &[DiscoveredArtifact]) -> Result<()> {
        for artifact in artifacts {
            self.cache_artifact(artifact)?;
        }
        Ok(())
    }

    fn cache_artifact(&self, artifact: &DiscoveredArtifact) -> Result<()> {
        // Create cache directory
        if let Some(cache_parent) = artifact.cached_path.parent() {
            fs::create_dir_all(cache_parent)
                .with_context(|| format!("Failed to create cache directory: {}", cache_parent.display()))?;
        }

        // Copy artifact to cache
        fs::copy(&artifact.original_path, &artifact.cached_path)
            .with_context(|| {
                format!(
                    "Failed to copy artifact from {} to {}",
                    artifact.original_path.display(),
                    artifact.cached_path.display()
                )
            })?;

        println!(
            "Cached {} artifact: {} -> {}",
            artifact.kind.as_str(),
            artifact.original_path.display(),
            artifact.cached_path.display()
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_patterns() {
        let discovery = ArtifactDiscovery::new("/tmp", None, BuildProfile::Release);

        let static_patterns = discovery.get_library_patterns("my_crate", &ArtifactKind::StaticLib);
        let dynamic_patterns = discovery.get_library_patterns("my_crate", &ArtifactKind::DynamicLib);

        if cfg!(target_os = "windows") {
            assert!(static_patterns.contains(&"my_crate.lib".to_string()));
            assert!(dynamic_patterns.contains(&"my_crate.dll".to_string()));
        } else {
            assert!(static_patterns.contains(&"libmy_crate.a".to_string()));
            if cfg!(target_os = "macos") {
                assert!(dynamic_patterns.contains(&"libmy_crate.dylib".to_string()));
            } else {
                assert!(dynamic_patterns.contains(&"libmy_crate.so".to_string()));
            }
        }
    }

    #[test]
    fn test_cache_path_generation() {
        let discovery = ArtifactDiscovery::new("/tmp", Some("x86_64-unknown-linux-gnu".to_string()), BuildProfile::Release);

        let cache_path = discovery.get_cache_path("my_crate", &ArtifactKind::StaticLib);
        let expected_extension = if cfg!(target_os = "windows") { "lib" } else { "a" };

        assert!(cache_path.to_string_lossy().contains("x86_64-unknown-linux-gnu"));
        assert!(cache_path.to_string_lossy().contains("release"));
        assert!(cache_path.to_string_lossy().ends_with(&format!("my_crate.{}", expected_extension)));
    }
}