use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Creates a directory and all missing parent directories.
pub fn ensure_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    fs::create_dir_all(&path)
        .with_context(|| format!("Failed to create directory: {}", path.as_ref().display()))
}

/// Removes a directory and recreates it empty.
pub fn clean_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    if path.as_ref().exists() {
        fs::remove_dir_all(&path)
            .with_context(|| format!("Failed to remove directory: {}", path.as_ref().display()))?;
    }
    ensure_dir(path)
}

/// Writes a file only if it does not already exist.
/// Returns `true` if the file was created, `false` if it already existed.
pub fn write_if_not_exists<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, content: C) -> Result<bool> {
    if path.as_ref().exists() {
        return Ok(false);
    }
    write_force(path, content)?;
    Ok(true)
}

/// Writes content to a file, creating all parent directories as needed.
pub fn write_force<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, content: C) -> Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        ensure_dir(parent)?;
    }
    fs::write(&path, content)
        .with_context(|| format!("Failed to write file: {}", path.as_ref().display()))
}

/// Copies a file, creating all parent directories of the destination as needed.
pub fn copy_force<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<u64> {
    if let Some(parent) = to.as_ref().parent() {
        ensure_dir(parent)?;
    }
    fs::copy(&from, &to).with_context(|| {
        format!(
            "Failed to copy {} to {}",
            from.as_ref().display(),
            to.as_ref().display()
        )
    })
}

/// Returns `true` if the path exists on the filesystem.
pub fn exists<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().exists()
}

/// Reads the entire contents of a file into a `String`.
pub fn read_to_string<P: AsRef<Path>>(path: P) -> Result<String> {
    fs::read_to_string(&path)
        .with_context(|| format!("Failed to read file: {}", path.as_ref().display()))
}
