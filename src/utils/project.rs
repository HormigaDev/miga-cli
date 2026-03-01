use anyhow::{anyhow, Context, Result};

use crate::registry::manifest::{LockFile, ProjectManifest};
use crate::utils::fs;

const MANIFEST_PATH: &str = ".miga/miga.json";
const LOCK_PATH: &str = ".miga/modules.lock";

/// Ensures that the current directory contains an initialized miga project.
pub fn require_initialized() -> Result<()> {
    if !fs::exists(MANIFEST_PATH) {
        return Err(anyhow!("No miga project found. Run 'miga init' first."));
    }
    Ok(())
}

/// Loads and parses the project manifest from `.miga/miga.json`.
pub fn load_manifest() -> Result<ProjectManifest> {
    let content = fs::read_to_string(MANIFEST_PATH)?;
    serde_json::from_str(&content).context("Failed to parse miga.json")
}

/// Serializes and writes the project manifest to `.miga/miga.json`.
pub fn save_manifest(manifest: &ProjectManifest) -> Result<()> {
    fs::write_force(MANIFEST_PATH, serde_json::to_string_pretty(manifest)?)
}

/// Loads the lock file from `.miga/modules.lock`, returning an empty lock if absent.
pub fn load_lock() -> Result<LockFile> {
    if !fs::exists(LOCK_PATH) {
        return Ok(LockFile::default());
    }
    serde_json::from_str(&fs::read_to_string(LOCK_PATH)?).context("Failed to parse modules.lock")
}

/// Serializes and writes the lock file to `.miga/modules.lock`.
pub fn save_lock(lock: &LockFile) -> Result<()> {
    fs::write_force(LOCK_PATH, serde_json::to_string_pretty(lock)?)
}
