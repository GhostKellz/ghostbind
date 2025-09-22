use anyhow::{anyhow, Context, Result};
use cargo_metadata::{Metadata, MetadataCommand};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct CargoBuilder {
    manifest_path: PathBuf,
    target: Option<String>,
    profile: BuildProfile,
    features: Vec<String>,
    no_default_features: bool,
}

#[derive(Debug, Clone)]
pub enum BuildProfile {
    Debug,
    Release,
}

impl BuildProfile {
    pub fn as_str(&self) -> &str {
        match self {
            BuildProfile::Debug => "debug",
            BuildProfile::Release => "release",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CrateInfo {
    pub name: String,
    pub targets: Vec<CrateTarget>,
    pub manifest_dir: PathBuf,
    pub target_directory: PathBuf,
}

#[derive(Debug, Clone)]
pub struct CrateTarget {
    pub name: String,
    pub kind: TargetKind,
}

#[derive(Debug, Clone)]
pub enum TargetKind {
    StaticLib,
    CdyLib,
    Bin,
    Other(String),
}

impl TargetKind {
    pub fn from_cargo_kinds(kinds: &[String]) -> Self {
        for kind in kinds {
            match kind.as_str() {
                "staticlib" => return TargetKind::StaticLib,
                "cdylib" => return TargetKind::CdyLib,
                "bin" => return TargetKind::Bin,
                _ => continue,
            }
        }
        TargetKind::Other(kinds.join(","))
    }

    pub fn is_library(&self) -> bool {
        matches!(self, TargetKind::StaticLib | TargetKind::CdyLib)
    }
}

impl CargoBuilder {
    pub fn new(manifest_path: impl AsRef<Path>) -> Self {
        Self {
            manifest_path: manifest_path.as_ref().to_path_buf(),
            target: None,
            profile: BuildProfile::Release,
            features: Vec::new(),
            no_default_features: false,
        }
    }

    pub fn target(mut self, target: String) -> Self {
        self.target = Some(target);
        self
    }

    pub fn profile(mut self, profile: BuildProfile) -> Self {
        self.profile = profile;
        self
    }

    pub fn features(mut self, features: Vec<String>) -> Self {
        self.features = features;
        self
    }

    pub fn no_default_features(mut self, no_default: bool) -> Self {
        self.no_default_features = no_default;
        self
    }

    pub fn get_metadata(&self) -> Result<CrateInfo> {
        let mut cmd = MetadataCommand::new();
        cmd.manifest_path(&self.manifest_path);

        let metadata = cmd.exec()
            .context("Failed to execute cargo metadata")?;

        self.extract_crate_info(&metadata)
    }

    fn extract_crate_info(&self, metadata: &Metadata) -> Result<CrateInfo> {
        // Find the root package (the one with the manifest we're looking at)
        let manifest_path_canonical = self.manifest_path.canonicalize()
            .with_context(|| format!("Failed to canonicalize manifest path: {}", self.manifest_path.display()))?;

        let package = metadata.packages.iter()
            .find(|pkg| {
                // Compare canonical paths to handle relative vs absolute paths
                if let Ok(pkg_manifest_canonical) = pkg.manifest_path.as_std_path().canonicalize() {
                    pkg_manifest_canonical == manifest_path_canonical
                } else {
                    false
                }
            })
            .ok_or_else(|| anyhow!("Could not find package for manifest path: {}", self.manifest_path.display()))?;

        let targets = package.targets.iter()
            .filter(|target| target.kind.iter().any(|k| k == "staticlib" || k == "cdylib"))
            .map(|target| CrateTarget {
                name: target.name.clone(),
                kind: TargetKind::from_cargo_kinds(&target.kind),
            })
            .collect();

        let manifest_dir = self.manifest_path.parent()
            .ok_or_else(|| anyhow!("Invalid manifest path"))?;

        Ok(CrateInfo {
            name: package.name.clone(),
            targets,
            manifest_dir: manifest_dir.to_path_buf(),
            target_directory: metadata.target_directory.clone().into_std_path_buf(),
        })
    }

    pub fn build(&self) -> Result<()> {
        let mut cmd = Command::new("cargo");
        cmd.arg("build");
        cmd.arg("--manifest-path").arg(&self.manifest_path);

        match self.profile {
            BuildProfile::Release => {
                cmd.arg("--release");
            }
            BuildProfile::Debug => {
                // Debug is default, no flag needed
            }
        }

        if let Some(ref target) = self.target {
            cmd.arg("--target").arg(target);
        }

        if self.no_default_features {
            cmd.arg("--no-default-features");
        }

        if !self.features.is_empty() {
            cmd.arg("--features").arg(self.features.join(","));
        }

        // Only build library targets for FFI
        cmd.arg("--lib");

        let output = cmd.output()
            .context("Failed to execute cargo build")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Cargo build failed: {}", stderr));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_kind_detection() {
        assert!(matches!(
            TargetKind::from_cargo_kinds(&["staticlib".to_string()]),
            TargetKind::StaticLib
        ));

        assert!(matches!(
            TargetKind::from_cargo_kinds(&["cdylib".to_string()]),
            TargetKind::CdyLib
        ));

        assert!(TargetKind::StaticLib.is_library());
        assert!(TargetKind::CdyLib.is_library());
        assert!(!TargetKind::Bin.is_library());
    }
}