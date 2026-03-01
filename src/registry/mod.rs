pub mod manifest;

use anyhow::{anyhow, Context, Result};
use manifest::{parse_dep_spec, ModuleManifest, ResolvedModule};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/HormigaDev/miga-registry/master";

#[derive(Serialize, Deserialize, Debug)]
struct RegistryConfig {
    registry_url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RegistryRoot {
    pub latest_versions: std::collections::HashMap<String, String>,
}

fn config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Failed to determine home directory")?;
    Ok(home.join(".miga").join("config"))
}

fn write_default_config(path: &Path) -> Result<String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("Failed to create .miga config directory")?;
    }
    let config = RegistryConfig {
        registry_url: DEFAULT_REGISTRY_URL.to_string(),
    };
    fs::write(path, serde_json::to_string_pretty(&config)?).context("Failed to write config")?;
    Ok(config.registry_url)
}

fn registry_base_url() -> Result<String> {
    let path = config_path()?;
    if !path.exists() {
        return write_default_config(&path);
    }

    let content = fs::read_to_string(&path).context("Failed to read config file")?;
    let mut config: RegistryConfig = match serde_json::from_str(&content) {
        Ok(c) => c,
        Err(_) => {
            fs::remove_file(&path).context("Failed to remove corrupted config")?;
            return write_default_config(&path);
        }
    };

    if config.registry_url.trim().is_empty() {
        return write_default_config(&path);
    }
    if config.registry_url.ends_with('/') {
        config.registry_url.pop();
    }

    Ok(config.registry_url)
}

pub fn fetch_module_manifest(name: &str, version: Option<&str>) -> Result<ModuleManifest> {
    let base_url = registry_base_url()?;

    let target_version = match version {
        Some(v) => v.to_string(),
        None => {
            let root_url = format!("{}/versions.json", base_url);
            let response = reqwest::blocking::get(&root_url)
                .context("Failed to fetch registry versions.json")?;

            let root: RegistryRoot = response
                .json()
                .context("Failed to parse registry versions.json")?;

            root.latest_versions.get(name).cloned().ok_or_else(|| {
                anyhow!(
                    "Module '{}' has no 'latest' version defined in the registry.",
                    name
                )
            })?
        }
    };

    let url = format!(
        "{}/modules/{}/v{}/manifest.json",
        base_url, name, target_version
    );

    let response = reqwest::blocking::get(&url)
        .with_context(|| format!("Failed to reach manifest for '{}' v{}", name, target_version))?;

    match response.status().as_u16() {
        200 => response
            .json::<ModuleManifest>()
            .context("Failed to parse module manifest"),
        404 => Err(anyhow!(
            "Module '{}' v{} not found at {}",
            name,
            target_version,
            url
        )),
        code => Err(anyhow!(
            "Registry returned HTTP {} for '{}' v{}",
            code,
            name,
            target_version
        )),
    }
}

/// Downloads a module archive or individual file as raw bytes.
pub fn fetch_module_archive(name: &str, version: &str, archive_name: &str) -> Result<Vec<u8>> {
    let base_url = registry_base_url()?;
    let url = format!("{}/modules/{}/v{}/{}", base_url, name, version, archive_name);

    let mut response = reqwest::blocking::get(&url)
        .with_context(|| format!("Failed to download archive for '{}'", name))?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Archive '{}' not found for module '{}'",
            archive_name,
            name
        ));
    }

    let mut buffer = Vec::new();
    response.copy_to(&mut buffer)?;
    Ok(buffer)
}

/// Resolves the full transitive dependency tree for a module from the registry.
///
/// Each returned `ResolvedModule` carries the registry manifest **and** a
/// `resolved_deps` map that tells the compiler which version of each
/// transitive dep was chosen.
pub fn resolve_dependencies(
    name: &str,
    version: Option<&str>,
    visited: &mut HashSet<String>,
) -> Result<Vec<ResolvedModule>> {
    let manifest = fetch_module_manifest(name, version)?;

    let key = format!("{}@{}", manifest.name, manifest.version);
    if !visited.insert(key) {
        return Ok(vec![]);
    }

    let mut all = vec![];
    let mut resolved_deps: HashMap<String, String> = HashMap::new();

    for dep_spec in &manifest.dependencies {
        let (dep_name, dep_version) = parse_dep_spec(dep_spec);

        let sub_modules = resolve_dependencies(dep_name, dep_version, visited)?;
        for sub in &sub_modules {
            resolved_deps.insert(sub.manifest.name.clone(), sub.manifest.version.clone());
        }
        all.extend(sub_modules);
    }

    all.push(ResolvedModule {
        manifest,
        resolved_deps,
    });

    Ok(all)
}
