//! Downloads TypeScript type definitions (`.d.ts` files) from the npm registry
//! without requiring a local npm installation. No executable code is ever written to disk.

use anyhow::{anyhow, Context, Result};
use flate2::read::GzDecoder;
use serde::Deserialize;
use std::io::Read;
use std::path::PathBuf;
use tar::Archive;

const NPM_REGISTRY: &str = "https://registry.npmjs.org";

#[derive(Deserialize, Debug)]
struct NpmPackageMetadata {
    versions: std::collections::HashMap<String, NpmVersion>,
    #[serde(rename = "dist-tags")]
    dist_tags: DistTags,
}

#[derive(Deserialize, Debug)]
struct DistTags {
    latest: String,
}

#[derive(Deserialize, Debug)]
struct NpmVersion {
    dist: NpmDist,
}

#[derive(Deserialize, Debug)]
struct NpmDist {
    tarball: String,
}

pub struct FetchedPackage {
    pub name: String,
    pub version: String,
    /// Directory where the `.d.ts` files were extracted.
    #[allow(dead_code)]
    pub dest_dir: PathBuf,
    /// Individual files that were written to disk.
    #[allow(dead_code)]
    pub files: Vec<PathBuf>,
}

/// Downloads the type definitions for a package from the npm registry.
/// `spec` may be `"@minecraft/server"` or `"@minecraft/server@1.15.0"`.
pub fn fetch_types(spec: &str) -> Result<FetchedPackage> {
    let (name, requested_version) = parse_spec(spec)?;
    let metadata = fetch_metadata(&name)?;

    let version = match requested_version {
        Some(v) => {
            if !metadata.versions.contains_key(&v) {
                return Err(anyhow!(
                    "Version '{}' not found for '{}'. See https://www.npmjs.com/package/{}",
                    v,
                    name,
                    name
                ));
            }
            v
        }
        None => metadata.dist_tags.latest.clone(),
    };

    let tarball_url = metadata
        .versions
        .get(&version)
        .ok_or_else(|| anyhow!("Version '{}' missing from metadata", version))?
        .dist
        .tarball
        .clone();

    let tarball = download_tarball(&tarball_url)
        .with_context(|| format!("Failed to download tarball for '{}'", name))?;

    let dest_dir = PathBuf::from(".miga_modules").join(&name);
    let files = extract_dts(&tarball, &dest_dir)
        .with_context(|| format!("Failed to extract types from '{}'", name))?;

    if files.is_empty() {
        return Err(anyhow!(
            "No .d.ts files found in '{}@{}'. This package may not ship TypeScript types.",
            name,
            version
        ));
    }

    Ok(FetchedPackage {
        name,
        version,
        dest_dir,
        files,
    })
}

/// Parses `"pkg@version"` or `"pkg"` into `(name, Option<version>)`.
/// Handles scoped packages such as `@minecraft/server@1.15.0`.
fn parse_spec(spec: &str) -> Result<(String, Option<String>)> {
    if let Some(stripped) = spec.strip_prefix('@') {
        let parts: Vec<&str> = stripped.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid package spec: '{}'", spec));
        }
        let (scope, rest) = (parts[0], parts[1]);
        if let Some(at) = rest.find('@') {
            Ok((
                format!("@{}/{}", scope, &rest[..at]),
                Some(rest[at + 1..].to_string()),
            ))
        } else {
            Ok((format!("@{}/{}", scope, rest), None))
        }
    } else if let Some(at) = spec.find('@') {
        Ok((spec[..at].to_string(), Some(spec[at + 1..].to_string())))
    } else {
        Ok((spec.to_string(), None))
    }
}

fn fetch_metadata(name: &str) -> Result<NpmPackageMetadata> {
    let url = format!("{}/{}", NPM_REGISTRY, name.replace('/', "%2F"));
    let response = reqwest::blocking::get(&url)
        .with_context(|| format!("Failed to reach npm registry for '{}'", name))?;

    match response.status().as_u16() {
        200 => response
            .json::<NpmPackageMetadata>()
            .with_context(|| format!("Failed to parse npm metadata for '{}'", name)),
        404 => Err(anyhow!("Package '{}' not found in the npm registry.", name)),
        code => Err(anyhow!(
            "npm registry returned HTTP {} for '{}'",
            code,
            name
        )),
    }
}

fn download_tarball(url: &str) -> Result<Vec<u8>> {
    let mut response = reqwest::blocking::get(url).context("Failed to download tarball")?;
    if !response.status().is_success() {
        return Err(anyhow!(
            "Tarball download failed: HTTP {}",
            response.status()
        ));
    }
    let mut bytes = Vec::new();
    response
        .read_to_end(&mut bytes)
        .context("Failed to read tarball")?;
    Ok(bytes)
}

fn extract_dts(tarball: &[u8], dest: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut archive = Archive::new(GzDecoder::new(tarball));
    std::fs::create_dir_all(dest)
        .with_context(|| format!("Failed to create {}", dest.display()))?;

    let mut written = vec![];

    for entry in archive
        .entries()
        .context("Failed to read tarball entries")?
    {
        let mut entry = entry.context("Failed to read tarball entry")?;
        let path = entry.path().context("Failed to get entry path")?;

        // npm tarballs prefix all paths with "package/" — strip it
        let stripped = path.strip_prefix("package").unwrap_or(&path).to_path_buf();

        if !stripped.to_str().map_or(false, |s| s.ends_with(".d.ts")) {
            continue;
        }

        let dest_path = dest.join(&stripped);
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }

        let mut content = String::new();
        entry
            .read_to_string(&mut content)
            .with_context(|| format!("Failed to read {}", stripped.display()))?;
        std::fs::write(&dest_path, &content)
            .with_context(|| format!("Failed to write {}", dest_path.display()))?;

        written.push(dest_path);
    }

    Ok(written)
}
