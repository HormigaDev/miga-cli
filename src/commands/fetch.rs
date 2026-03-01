use anyhow::{anyhow, Result};
use dialoguer::{Confirm, Select};
use std::collections::HashSet;
use std::io::Cursor;
use std::path::Path;

use crate::registry::{
    self,
    manifest::{is_breaking_change, semver_cmp, LockFile, LockedModule, ResolvedModule},
};
use crate::utils::{fs, net, output, project, tsconfig};

pub fn run(module: Option<String>, version: Option<String>, update: bool) -> Result<()> {
    project::require_initialized()?;
    let online = net::is_online();

    if !online {
        output::warn("No internet connection detected. Running in offline mode.");
        if update {
            return Err(anyhow!("Cannot update modules while offline."));
        }
    }

    match (module, update) {
        (None, true) => update_all(),
        (None, false) => install_all(online),
        (Some(name), true) => {
            if version.is_some() {
                return Err(anyhow!("--version and --update cannot be used together."));
            }
            update_module(&name)
        }
        (Some(name), false) => install_module(&name, version.as_deref(), online),
    }
}

fn install_all(online: bool) -> Result<()> {
    output::section("Installing dependencies from miga.json");
    let manifest = project::load_manifest()?;

    if manifest.modules.is_empty() {
        output::info("No modules listed. Use 'miga fetch <module>'.");
        return Ok(());
    }

    let lock = project::load_lock()?;
    let missing: Vec<(String, String)> = manifest
        .modules
        .iter()
        .filter(|(name, version)| {
            !lock
                .modules
                .get(*name)
                .map_or(false, |vs| vs.contains_key(*version))
        })
        .map(|(n, v)| (n.clone(), v.clone()))
        .collect();

    if missing.is_empty() {
        output::success("All modules are synchronized.");
        return Ok(());
    }

    if !online {
        return Err(anyhow!("Cannot install modules while offline."));
    }

    for (name, version) in missing {
        install_module(&name, Some(&version), true)?;
    }

    Ok(())
}

fn install_module(name: &str, version: Option<&str>, online: bool) -> Result<()> {
    output::section(&format!("Fetching '{}'", name));

    if !online {
        let lock = project::load_lock()?;
        if let Some(versions) = lock.modules.get(name) {
            let any_cached = versions.values().any(|locked| files_exist(&locked.files));
            if any_cached {
                output::success(&format!("'{}' is ready (cached).", name));
                return Ok(());
            }
        }
        return Err(anyhow!("Module not found in cache. Internet required."));
    }

    let mut visited = HashSet::new();
    let resolved = registry::resolve_dependencies(name, version, &mut visited)?;

    let mut lock = project::load_lock()?;
    let mut manifest = project::load_manifest()?;

    for module in &resolved {
        let action =
            check_version_conflict(&module.manifest.name, &module.manifest.version, &lock)?;

        match action {
            ConflictAction::Install => {
                install_single(module, &mut lock)?;
            }
            ConflictAction::Skip => {
                output::step(&format!(
                    "skipped  {} v{} (already installed)",
                    module.manifest.name, module.manifest.version
                ));
            }
            ConflictAction::Replace(old_version) => {
                remove_version_files(&module.manifest.name, &old_version, &mut lock)?;
                install_single(module, &mut lock)?;
            }
        }

        manifest.modules.insert(
            module.manifest.name.clone(),
            module.manifest.version.clone(),
        );
    }

    project::save_manifest(&manifest)?;
    project::save_lock(&lock)?;
    tsconfig::update(&manifest, &lock)?;

    output::success(&format!("'{}' installed and configured.", name));
    Ok(())
}

fn update_module(name: &str) -> Result<()> {
    output::section(&format!("Updating '{}'", name));

    let mut visited = HashSet::new();
    let resolved = registry::resolve_dependencies(name, None, &mut visited)?;

    let mut lock = project::load_lock()?;
    let mut manifest = project::load_manifest()?;

    for module in &resolved {
        let action =
            check_version_conflict(&module.manifest.name, &module.manifest.version, &lock)?;

        match action {
            ConflictAction::Install => {
                install_single(module, &mut lock)?;
            }
            ConflictAction::Replace(old) => {
                remove_version_files(&module.manifest.name, &old, &mut lock)?;
                install_single(module, &mut lock)?;
            }
            ConflictAction::Skip => {}
        }

        manifest.modules.insert(
            module.manifest.name.clone(),
            module.manifest.version.clone(),
        );
    }

    project::save_manifest(&manifest)?;
    project::save_lock(&lock)?;
    tsconfig::update(&manifest, &lock)?;

    output::success(&format!("'{}' updated.", name));
    Ok(())
}

fn update_all() -> Result<()> {
    if !net::is_online() {
        return Err(anyhow!("Cannot update modules while offline."));
    }

    output::section("Updating all modules");

    let confirm = dialoguer::Input::<String>::new()
        .with_prompt("Type 'yes' to confirm")
        .interact_text()?;

    if confirm.trim().to_lowercase() != "yes" {
        return Ok(());
    }

    let lock = project::load_lock()?;
    let names: Vec<String> = lock.modules.keys().cloned().collect();

    for name in names {
        update_module(&name)?;
    }
    Ok(())
}

enum ConflictAction {
    /// No conflict — install the new version.
    Install,
    /// Exact version already present — skip.
    Skip,
    /// Replace the given old version with the new one.
    Replace(String),
}

fn check_version_conflict(
    name: &str,
    new_version: &str,
    lock: &LockFile,
) -> Result<ConflictAction> {
    let existing = match lock.modules.get(name) {
        Some(versions) => versions,
        None => return Ok(ConflictAction::Install),
    };

    // Exact version already installed — deduplicate.
    if existing.contains_key(new_version) {
        return Ok(ConflictAction::Skip);
    }

    // Pick the first existing version for comparison.
    let Some(existing_version) = existing.keys().next() else {
        return Ok(ConflictAction::Install);
    };

    if is_breaking_change(existing_version, new_version) {
        // Different major versions — breaking change.
        // Default: keep both.
        let keep_both = Confirm::new()
            .with_prompt(format!(
                "Module '{}' v{} is installed. Version {} has breaking changes \
                 (different major version). Keep both versions? (recommended)",
                name, existing_version, new_version
            ))
            .default(true)
            .interact()?;

        if keep_both {
            Ok(ConflictAction::Install)
        } else {
            let newer = if semver_cmp(new_version, existing_version).is_gt() {
                new_version
            } else {
                existing_version.as_str()
            };

            let items = vec![
                format!("Keep only v{} (newer)", newer),
                format!(
                    "Keep only v{} (older)",
                    if newer == new_version {
                        existing_version.as_str()
                    } else {
                        new_version
                    }
                ),
            ];

            let choice = Select::new()
                .with_prompt("Which version do you want to keep?")
                .items(&items)
                .default(0)
                .interact()?;

            if choice == 0 {
                if newer == new_version {
                    Ok(ConflictAction::Replace(existing_version.clone()))
                } else {
                    Ok(ConflictAction::Skip)
                }
            } else if newer == new_version {
                Ok(ConflictAction::Skip)
            } else {
                Ok(ConflictAction::Replace(existing_version.clone()))
            }
        }
    } else {
        // Same major version — compatible update.
        let newer = if semver_cmp(new_version, existing_version).is_gt() {
            new_version
        } else {
            existing_version.as_str()
        };

        let items = vec![
            format!("Upgrade to v{} (recommended)", newer),
            "Keep both versions".to_string(),
            "Keep existing only".to_string(),
        ];

        let choice = Select::new()
            .with_prompt(format!(
                "Module '{}': v{} installed, v{} requested",
                name, existing_version, new_version
            ))
            .items(&items)
            .default(0)
            .interact()?;

        match choice {
            0 => {
                if newer == new_version {
                    Ok(ConflictAction::Replace(existing_version.clone()))
                } else {
                    Ok(ConflictAction::Skip)
                }
            }
            1 => Ok(ConflictAction::Install),
            _ => Ok(ConflictAction::Skip),
        }
    }
}

fn install_single(module: &ResolvedModule, lock: &mut LockFile) -> Result<()> {
    let m = &module.manifest;
    let target_dir = Path::new(".miga_modules")
        .join(&m.name)
        .join(format!("v{}", m.version));

    let installed_files = if let Some(archive_name) = &m.archive {
        output::step(&format!(
            "downloading {} v{} ({})",
            m.name, m.version, archive_name
        ));
        extract_archive(m, archive_name, &target_dir)?
    } else {
        output::step(&format!("downloading {} v{}", m.name, m.version));
        download_files(m, &target_dir)?
    };

    let versions = lock.modules.entry(m.name.clone()).or_default();
    versions.insert(
        m.version.clone(),
        LockedModule {
            entry: m.entry.clone(),
            files: installed_files,
            resolved_deps: module.resolved_deps.clone(),
        },
    );

    Ok(())
}

/// Removes files for a specific version of a module.
fn remove_version_files(name: &str, version: &str, lock: &mut LockFile) -> Result<()> {
    let version_dir = Path::new(".miga_modules")
        .join(name)
        .join(format!("v{}", version));

    if version_dir.exists() {
        std::fs::remove_dir_all(&version_dir)?;
    }

    if let Some(versions) = lock.modules.get_mut(name) {
        versions.remove(version);
        if versions.is_empty() {
            lock.modules.remove(name);
        }
    }

    Ok(())
}

/// Extracts a ZIP archive from the registry into the target directory.
fn extract_archive(
    module: &crate::registry::manifest::ModuleManifest,
    archive_name: &str,
    target_dir: &Path,
) -> Result<Vec<String>> {
    let bytes = registry::fetch_module_archive(&module.name, &module.version, archive_name)?;
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
    let mut installed = Vec::new();

    fs::ensure_dir(target_dir)?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = target_dir.join(file.name());

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
            installed.push(outpath.to_string_lossy().to_string());
        }
    }

    Ok(installed)
}

/// Downloads individual module files as a fallback when no archive is available.
fn download_files(
    module: &crate::registry::manifest::ModuleManifest,
    target_dir: &Path,
) -> Result<Vec<String>> {
    let all_files: Vec<String> = std::iter::once(module.entry.clone())
        .chain(module.files.iter().cloned())
        .collect();

    let mut installed = Vec::new();

    fs::ensure_dir(target_dir)?;
    for filename in &all_files {
        let content = registry::fetch_module_archive(&module.name, &module.version, filename)?;
        let dest = target_dir.join(filename);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dest, content)?;
        installed.push(dest.to_string_lossy().to_string());
    }

    Ok(installed)
}

fn files_exist(files: &[String]) -> bool {
    files.iter().all(|f| Path::new(f).exists())
}
