use crate::compiler::{self, CompileOptions, DependencyInfo};
use crate::registry::manifest::{LockFile, ProjectManifest};
use crate::utils::{fs, json as utils_json, project};
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

pub fn load_project() -> Result<ProjectManifest> {
    project::require_initialized()?;
    project::load_manifest()
}

pub fn process_behavior(src: &Path, dest: &Path, opts: &CompileOptions) -> Result<()> {
    if !src.exists() {
        return Ok(());
    }
    fs::ensure_dir(dest)?;

    for entry in WalkDir::new(src)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
    {
        let path = entry.path();
        let relative = path.strip_prefix(src)?;
        let ext = path.extension().and_then(|x| x.to_str()).unwrap_or("");
        let dest_file = dest.join(relative);

        match ext {
            "ts" => {
                let dest_path = dest_file.with_extension("js");
                let result = compiler::compile_file(path, &dest_path, opts)?;
                fs::write_force(&dest_path, &result.code)?;
                if let Some(map) = &result.source_map {
                    let map_path = format!("{}.map", dest_path.display());
                    fs::write_force(map_path, map)?;
                }
            }
            "js" if !path.with_extension("ts").exists() => {
                fs::copy_force(path, &dest_file)?;
            }
            "json" if opts.minify => {
                let content = std::fs::read_to_string(path)?;
                let minified = utils_json::minify(&content)?;
                fs::write_force(&dest_file, &minified)?;
            }
            _ => {
                fs::copy_force(path, &dest_file)?;
            }
        }
    }
    Ok(())
}

pub fn process_resource(src: &Path, dest: &Path, minify: bool) -> Result<()> {
    if !src.exists() {
        return Ok(());
    }
    for entry in WalkDir::new(src)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
    {
        let path = entry.path();
        let relative = path.strip_prefix(src)?;
        let dest_path = dest.join(relative);

        if path.extension().map_or(false, |e| e == "json") && minify {
            let content = std::fs::read_to_string(path)?;
            let processed = utils_json::minify(&content)?;
            fs::write_force(dest_path, &processed)?;
        } else {
            fs::copy_force(path, &dest_path)?;
        }
    }
    Ok(())
}

/// Compiles all versioned module dependencies from `.miga_modules/` into
/// the output `libs/` directory.
///
/// Each module is compiled with its own `dep_versions` map so that bare
/// imports resolve to the correct versioned path.
pub fn process_dependencies(
    src_root: &Path,
    dest_root: &Path,
    base_opts: &CompileOptions,
    lock: &LockFile,
) -> Result<()> {
    for (module_name, versions) in &lock.modules {
        for (version, locked) in versions {
            let src = src_root.join(module_name).join(format!("v{}", version));
            let dest = dest_root.join(module_name).join(format!("v{}", version));

            if !src.exists() {
                continue;
            }

            let mut dep_versions_map = HashMap::new();
            for (dep_name, dep_version) in &locked.resolved_deps {
                if let Some(dep_module_versions) = lock.modules.get(dep_name) {
                    if let Some(dep_locked) = dep_module_versions.get(dep_version) {
                        dep_versions_map.insert(
                            dep_name.clone(),
                            DependencyInfo {
                                version: dep_version.clone(),
                                entry: dep_locked.entry.clone(),
                            },
                        );
                    }
                }
            }

            let module_opts = CompileOptions {
                minify: base_opts.minify,
                source_maps: base_opts.source_maps,
                script_root: base_opts.script_root.clone(),
                dep_versions: dep_versions_map,
            };

            process_behavior(&src, &dest, &module_opts)?;
        }
    }
    Ok(())
}

/// Builds `dep_versions` for user scripts from the project manifest
/// and lock file. Each directly-installed module maps to its pinned version.
pub fn user_dep_versions(
    manifest: &ProjectManifest,
    lock: &LockFile,
) -> HashMap<String, DependencyInfo> {
    let mut versions = HashMap::new();

    for (name, version) in &manifest.modules {
        if let Some(module_versions) = lock.modules.get(name) {
            if let Some(locked) = module_versions.get(version) {
                versions.insert(
                    name.clone(),
                    DependencyInfo {
                        version: version.clone(),
                        entry: locked.entry.clone(),
                    },
                );
            }
        }
    }

    // Also include any transitive deps that only appear in the lock,
    // picking the first available version for unversioned fallback.
    for (name, installed) in &lock.modules {
        // If not explicitly in manifest, we pick the first one from lock
        if !versions.contains_key(name) {
            if let Some((ver, locked)) = installed.iter().next() {
                versions.insert(
                    name.clone(),
                    DependencyInfo {
                        version: ver.clone(),
                        entry: locked.entry.clone(),
                    },
                );
            }
        }
    }

    versions
}
