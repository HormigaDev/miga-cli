use anyhow::{anyhow, Result};
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::json;
use std::time::Duration;

use crate::registry::manifest::ProjectManifest;
use crate::utils::{fs, net, npm, output, project, tsconfig};

pub fn run(packages: Vec<String>) -> Result<()> {
    project::require_initialized()?;

    if packages.is_empty() {
        return Err(anyhow!("No package specified."));
    }

    if !net::is_online() {
        return Err(anyhow!("NPM requires internet access."));
    }

    output::section("Adding external type packages");

    let mut manifest = project::load_manifest()?;
    let mut modified = false;

    for package in &packages {
        let name_only = extract_package_name(package);

        if let Some(version) = manifest.externals.get(&name_only) {
            output::info(&format!(
                "Package '{}' v{} already in miga.json",
                name_only, version
            ));
            continue;
        }

        download_and_register(package, &mut manifest)?;

        if let Some(version) = manifest.externals.get(&name_only) {
            sync_behavior_manifest(&name_only, version)?;
        }

        modified = true;
    }

    if modified {
        project::save_manifest(&manifest)?;
        let lock = project::load_lock()?;
        tsconfig::update(&manifest, &lock)?;
        output::success("Done. Project synchronized.");
    }

    Ok(())
}

/// Extracts the bare package name from a spec like `@minecraft/server@2.5.0`.
fn extract_package_name(spec: &str) -> String {
    if spec.starts_with('@') {
        let without_at = &spec[1..];
        let base = without_at.split('@').next().unwrap_or(without_at);
        format!("@{}", base)
    } else {
        spec.split('@').next().unwrap_or(spec).to_string()
    }
}

/// Injects the `@minecraft/*` dependency into `behavior/manifest.json`.
fn sync_behavior_manifest(name: &str, version: &str) -> Result<()> {
    if !name.starts_with("@minecraft/") {
        return Ok(());
    }

    let manifest_path = "behavior/manifest.json";
    if !fs::exists(manifest_path) {
        output::warn("behavior/manifest.json not found. Skipping auto-injection.");
        return Ok(());
    }

    let clean_version = normalize_mc_version(version);

    let content = fs::read_to_string(manifest_path)?;
    let mut manifest: serde_json::Value = serde_json::from_str(&content)?;

    if !manifest.get("dependencies").map_or(false, |d| d.is_array()) {
        manifest["dependencies"] = json!([]);
    }

    let deps = manifest["dependencies"].as_array_mut().unwrap();
    deps.retain(|d| d.get("module_name").and_then(|v| v.as_str()) != Some(name));
    deps.push(json!({ "module_name": name, "version": clean_version }));

    fs::write_force(manifest_path, serde_json::to_string_pretty(&manifest)?)?;
    output::step(&format!("synced manifest -> {} v{}", name, clean_version));

    Ok(())
}

/// Normalizes an npm version to a Bedrock-compatible format.
/// `"2.6.0-beta.1.23..."` -> `"2.6.0-beta"`, `"1.15.0.4"` -> `"1.15.0"`
fn normalize_mc_version(version: &str) -> String {
    if version.contains("-beta") {
        let base = version.split('-').next().unwrap_or(version);
        let xyz = base.split('.').take(3).collect::<Vec<_>>().join(".");
        format!("{}-beta", xyz)
    } else {
        version.split('.').take(3).collect::<Vec<_>>().join(".")
    }
}

/// Downloads type definitions from npm and registers them in the project manifest.
fn download_and_register(spec: &str, manifest: &mut ProjectManifest) -> Result<()> {
    output::info(&format!("resolving '{}' from npm...", spec));

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("  {spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(format!("downloading {}...", spec));
    pb.enable_steady_tick(Duration::from_millis(80));

    let fetched = npm::fetch_types(spec)?;
    pb.finish_and_clear();

    manifest
        .externals
        .insert(fetched.name.clone(), fetched.version.clone());
    Ok(())
}
