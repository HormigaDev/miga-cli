use anyhow::{anyhow, Context, Result};
use dialoguer::Input;
use std::path::Path;

use crate::utils::{fs, output, project, tsconfig};

pub fn run(packages: Vec<String>, all: bool) -> Result<()> {
    project::require_initialized()?;

    let mut manifest = project::load_manifest()?;
    let mut lock = project::load_lock()?;

    let targets: Vec<String> = if all {
        output::warn("WARNING: You are about to remove ALL packages from this project.");
        let confirm: String = Input::new()
            .with_prompt("Type 'yes' to confirm")
            .interact_text()?;

        if confirm.trim().to_lowercase() != "yes" {
            output::info("Operation cancelled.");
            return Ok(());
        }

        manifest
            .externals
            .keys()
            .cloned()
            .chain(manifest.modules.keys().cloned())
            .collect()
    } else {
        if packages.is_empty() {
            return Err(anyhow!(
                "No package specified.\n  Usage: miga remove <package>\n         miga remove --all"
            ));
        }
        packages
    };

    let mut modified = false;
    output::section("Removing packages and cleaning workspace");

    for package in &targets {
        let is_external = manifest.externals.contains_key(package);
        let is_internal =
            manifest.modules.contains_key(package) || lock.modules.contains_key(package);

        if !is_external && !is_internal {
            if !all {
                output::warn(&format!(
                    "Package '{}' is not installed. Skipping.",
                    package
                ));
            }
            continue;
        }

        if package.starts_with("@minecraft/") {
            remove_from_behavior_manifest(package)?;
        }

        remove_module_files(package)?;

        manifest.externals.remove(package);
        manifest.modules.remove(package);
        lock.modules.remove(package);

        output::step(&format!("removed  {}", package));
        modified = true;
    }

    if modified {
        project::save_manifest(&manifest)?;
        project::save_lock(&lock)?;
        tsconfig::update(&manifest, &lock)?;
        output::success("Done. Workspace is clean.");
    } else {
        output::info("No changes made.");
    }

    Ok(())
}

/// Removes a `@minecraft/*` script dependency from `behavior/manifest.json`.
/// UUID-based dependencies (e.g., resource pack links) are left untouched.
fn remove_from_behavior_manifest(name: &str) -> Result<()> {
    let manifest_path = "behavior/manifest.json";
    if !fs::exists(manifest_path) {
        return Ok(());
    }

    let content = fs::read_to_string(manifest_path)?;
    let mut manifest: serde_json::Value = serde_json::from_str(&content)?;

    if let Some(deps) = manifest
        .get_mut("dependencies")
        .and_then(|d| d.as_array_mut())
    {
        let before = deps.len();
        deps.retain(|d| d.get("module_name").and_then(|m| m.as_str()) != Some(name));

        if deps.len() != before {
            fs::write_force(manifest_path, serde_json::to_string_pretty(&manifest)?)?;
            output::step(&format!(
                "cleaned  behavior/manifest.json (removed {})",
                name
            ));
        }
    }

    Ok(())
}

/// Removes all versioned files for a module from `.miga_modules/`.
fn remove_module_files(name: &str) -> Result<()> {
    let module_dir = Path::new(".miga_modules").join(name);
    if module_dir.exists() {
        std::fs::remove_dir_all(&module_dir)
            .with_context(|| format!("Failed to delete module directory for '{}'", name))?;
    }
    Ok(())
}
