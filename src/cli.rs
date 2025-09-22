use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::artifact_discovery::ArtifactDiscovery;
use crate::cargo_integration::{BuildProfile, CargoBuilder};
use crate::header_generation::HeaderGenerator;
use crate::manifest::ManifestGenerator;
use crate::target_mapping::TargetMapping;

#[derive(Parser)]
#[command(name = "ghostbind")]
#[command(about = "A tiny, predictable build/FFI bridge that lets Zig consume Rust crates")]
#[command(version = "0.1.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Build a Rust crate and generate FFI artifacts
    Build {
        /// Path to Cargo.toml
        #[arg(long, default_value = "Cargo.toml")]
        manifest_path: PathBuf,

        /// Target triple for Zig (will be mapped to Rust target)
        #[arg(long)]
        zig_target: Option<String>,

        /// Override Rust target (bypasses mapping)
        #[arg(long)]
        rust_target: Option<String>,

        /// Build profile
        #[arg(long, default_value = "release")]
        profile: String,

        /// Features to enable
        #[arg(long)]
        features: Vec<String>,

        /// Disable default features
        #[arg(long)]
        no_default_features: bool,

        /// Path to cbindgen config
        #[arg(long)]
        cbindgen_config: Option<PathBuf>,

        /// Generate default cbindgen config if none exists
        #[arg(long)]
        generate_cbindgen_config: bool,
    },

    /// Generate headers only (assumes crate is already built)
    Headers {
        /// Path to Cargo.toml
        #[arg(long, default_value = "Cargo.toml")]
        manifest_path: PathBuf,

        /// Target triple
        #[arg(long)]
        target: Option<String>,

        /// Path to cbindgen config
        #[arg(long)]
        cbindgen_config: Option<PathBuf>,
    },

    /// Check system requirements and configuration
    Doctor,
}

pub fn run_cli() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build {
            manifest_path,
            zig_target,
            rust_target,
            profile,
            features,
            no_default_features,
            cbindgen_config,
            generate_cbindgen_config,
        } => {
            build_command(
                manifest_path,
                zig_target,
                rust_target,
                profile,
                features,
                no_default_features,
                cbindgen_config,
                generate_cbindgen_config,
            )
        }
        Commands::Headers {
            manifest_path,
            target,
            cbindgen_config,
        } => headers_command(manifest_path, target, cbindgen_config),
        Commands::Doctor => doctor_command(),
    }
}

fn build_command(
    manifest_path: PathBuf,
    zig_target: Option<String>,
    rust_target_override: Option<String>,
    profile: String,
    features: Vec<String>,
    no_default_features: bool,
    cbindgen_config: Option<PathBuf>,
    generate_cbindgen_config: bool,
) -> Result<()> {
    // Parse build profile
    let build_profile = match profile.as_str() {
        "debug" => BuildProfile::Debug,
        "release" => BuildProfile::Release,
        _ => return Err(anyhow::anyhow!("Invalid profile: {}. Use 'debug' or 'release'", profile)),
    };

    // Determine the Rust target
    let rust_target = if let Some(override_target) = rust_target_override {
        override_target
    } else if let Some(zig_target) = zig_target {
        let target_mapping = TargetMapping::new();
        target_mapping.map_target_or_default(&zig_target)
    } else {
        // Use host target
        get_host_target()?
    };

    println!("Building crate with target: {}", rust_target);

    // Create Cargo builder
    let mut cargo_builder = CargoBuilder::new(&manifest_path)
        .profile(build_profile.clone())
        .features(features)
        .no_default_features(no_default_features);

    let is_cross_compile = rust_target != get_host_target()?;
    if is_cross_compile {
        cargo_builder = cargo_builder.target(rust_target.clone());
    }

    // Get crate metadata
    let crate_info = cargo_builder.get_metadata()
        .context("Failed to get crate metadata")?;

    println!("Found crate: {} with {} targets", crate_info.name, crate_info.targets.len());

    // Generate default cbindgen config if requested
    if generate_cbindgen_config {
        let header_generator = HeaderGenerator::new(None);
        header_generator.create_default_cbindgen_config(&crate_info.manifest_dir)?;
    }

    // Build the crate
    cargo_builder.build()
        .context("Failed to build crate")?;

    println!("Crate built successfully");

    // Discover artifacts
    let artifact_discovery = ArtifactDiscovery::new(
        &crate_info.target_directory,
        Some(rust_target.clone()),
        build_profile,
    );

    let artifacts = artifact_discovery.discover_artifacts(&crate_info)
        .context("Failed to discover artifacts")?;

    if artifacts.is_empty() {
        return Err(anyhow::anyhow!("No library artifacts found. Make sure your crate produces a staticlib or cdylib"));
    }

    println!("Found {} artifacts", artifacts.len());

    // Cache artifacts
    artifact_discovery.cache_artifacts(&artifacts)
        .context("Failed to cache artifacts")?;

    // Generate headers
    let header_generator = HeaderGenerator::new(cbindgen_config);
    let headers = header_generator.generate_headers(&crate_info, Some(&rust_target))
        .context("Failed to generate headers")?;

    // Generate manifest for the first (primary) artifact
    let primary_artifact = &artifacts[0];
    let manifest_generator = ManifestGenerator::new();
    let manifest = manifest_generator.generate_manifest(
        &crate_info.name,
        primary_artifact,
        &headers,
        &rust_target,
    ).context("Failed to generate manifest")?;

    // Write manifest
    let manifest_path = manifest_generator.write_manifest(
        &manifest,
        Some(&rust_target),
    ).context("Failed to write manifest")?;

    // Output the manifest path for tooling
    println!("\nManifest path: {}", manifest_path.display());

    Ok(())
}

fn headers_command(
    manifest_path: PathBuf,
    target: Option<String>,
    cbindgen_config: Option<PathBuf>,
) -> Result<()> {
    // Get crate metadata
    let cargo_builder = CargoBuilder::new(&manifest_path);
    let crate_info = cargo_builder.get_metadata()
        .context("Failed to get crate metadata")?;

    // Generate headers
    let header_generator = HeaderGenerator::new(cbindgen_config);
    let headers = header_generator.generate_headers(&crate_info, target.as_deref())
        .context("Failed to generate headers")?;

    println!("Generated {} headers:", headers.len());
    for header in &headers {
        println!("  {}", header.header_path.display());
    }

    Ok(())
}

fn doctor_command() -> Result<()> {
    println!("Ghostbind Doctor - Checking system requirements...\n");

    // Check Rust/Cargo
    check_command_available("cargo", "Rust toolchain")?;
    check_command_available("rustc", "Rust compiler")?;

    // Check cbindgen
    match which::which("cbindgen") {
        Ok(path) => println!("✓ cbindgen found at: {}", path.display()),
        Err(_) => {
            println!("✗ cbindgen not found");
            println!("  Install with: cargo install cbindgen");
        }
    }

    // Check common system tools
    if cfg!(unix) {
        check_command_available("cc", "C compiler (optional, for testing generated headers)")?;
    }

    println!("\nTarget mapping support:");
    let target_mapping = TargetMapping::new();
    let supported_targets = target_mapping.supported_targets();
    println!("  Supported Zig targets: {}", supported_targets.len());
    for target in supported_targets.iter().take(5) {
        if let Some(rust_target) = target_mapping.map_target(target) {
            println!("    {} -> {}", target, rust_target);
        }
    }
    if supported_targets.len() > 5 {
        println!("    ... and {} more", supported_targets.len() - 5);
    }

    println!("\n✓ Ghostbind doctor check complete");

    Ok(())
}

fn check_command_available(command: &str, description: &str) -> Result<()> {
    match which::which(command) {
        Ok(path) => {
            println!("✓ {} found at: {}", description, path.display());
            Ok(())
        }
        Err(_) => {
            println!("✗ {} not found ({})", description, command);
            Err(anyhow::anyhow!("{} is required but not found in PATH", description))
        }
    }
}

fn get_host_target() -> Result<String> {
    // This is a simplified version - in a real implementation,
    // you might want to detect the actual host target more accurately
    if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        Ok("x86_64-unknown-linux-gnu".to_string())
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "x86_64") {
        Ok("x86_64-apple-darwin".to_string())
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        Ok("aarch64-apple-darwin".to_string())
    } else if cfg!(target_os = "windows") && cfg!(target_arch = "x86_64") {
        Ok("x86_64-pc-windows-msvc".to_string())
    } else {
        // Fallback - use rustc to get the host target
        let output = std::process::Command::new("rustc")
            .args(["--version", "--verbose"])
            .output()
            .context("Failed to run rustc to detect host target")?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.starts_with("host: ") {
                return Ok(line.strip_prefix("host: ").unwrap().to_string());
            }
        }

        Err(anyhow::anyhow!("Could not detect host target"))
    }
}