use anyhow::{anyhow, Context, Result};
use serde_json::json;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

use crate::compiler::CompileOptions;
use crate::utils::{builder, fs as miga_fs, output, project};

pub fn run() -> Result<()> {
    let manifest = builder::load_project()?;
    let lock = project::load_lock()?;
    let safe_name = manifest.name.replace(' ', "_").to_lowercase();
    let dep_versions = builder::user_dep_versions(&manifest, &lock);

    output::section("miga build — release mode");
    output::info(&format!("addon:   {}", manifest.name));
    output::info(&format!("version: {}", manifest.version));

    let dist_dir = Path::new("dist");
    let build_dir = Path::new("build");
    let bp_dist = dist_dir.join("behavior");
    let rp_dist = dist_dir.join("resource");

    miga_fs::clean_dir(dist_dir)?;
    miga_fs::clean_dir(build_dir)?;

    let opts = CompileOptions {
        minify: true,
        source_maps: false,
        script_root: PathBuf::from("scripts"),
        dep_versions,
    };

    output::step("processing behavior pack...");
    builder::process_behavior(Path::new("behavior"), &bp_dist, &opts)?;

    output::step("bundling dependencies...");
    builder::process_dependencies(
        Path::new(".miga_modules"),
        &bp_dist.join("scripts/libs"),
        &opts,
        &lock,
    )?;

    output::step("processing resource pack...");
    builder::process_resource(Path::new("resource"), &rp_dist, true)?;

    output::step("syncing manifest versions...");
    sync_versions(&bp_dist, &rp_dist, &manifest.version)?;

    package(&safe_name, &manifest.version, &bp_dist, &rp_dist, build_dir)?;

    output::success("Build complete. Check the /build folder.");
    Ok(())
}

fn sync_versions(bp_path: &Path, rp_path: &Path, version: &str) -> Result<()> {
    for manifest_path in [bp_path.join("manifest.json"), rp_path.join("manifest.json")] {
        if manifest_path.exists() {
            sync_manifest_version(&manifest_path, version)
                .with_context(|| format!("Failed to sync {}", manifest_path.display()))?;
        }
    }
    Ok(())
}

fn sync_manifest_version(path: &Path, version: &str) -> Result<()> {
    let parts: Vec<u32> = version.split('.').map(|s| s.parse().unwrap_or(0)).collect();
    if parts.len() < 3 {
        return Err(anyhow!(
            "Invalid version format in miga.json. Expected x.y.z"
        ));
    }
    let v_array = json!([parts[0], parts[1], parts[2]]);

    let content = miga_fs::read_to_string(path)?;
    let mut manifest: serde_json::Value = serde_json::from_str(&content)?;

    if let Some(header) = manifest.get_mut("header") {
        header["version"] = v_array.clone();
    }
    if let Some(modules) = manifest.get_mut("modules").and_then(|m| m.as_array_mut()) {
        for module in modules {
            module["version"] = v_array.clone();
        }
    }

    miga_fs::write_force(path, serde_json::to_string_pretty(&manifest)?)?;
    Ok(())
}

fn package(name: &str, version: &str, bp: &Path, rp: &Path, build_dir: &Path) -> Result<()> {
    let bp_pack = build_dir.join(format!("{}_bp_v{}.mcpack", name, version));
    let rp_pack = build_dir.join(format!("{}_rp_v{}.mcpack", name, version));
    let addon = build_dir.join(format!("{}_v{}.mcaddon", name, version));

    if bp.exists() {
        output::step(&format!(
            "packaging {}...",
            bp_pack.file_name().unwrap().to_str().unwrap()
        ));
        zip_dir(bp, &bp_pack)?;
    }
    if rp.exists() {
        output::step(&format!(
            "packaging {}...",
            rp_pack.file_name().unwrap().to_str().unwrap()
        ));
        zip_dir(rp, &rp_pack)?;
    }

    output::step("creating .mcaddon...");
    let file = File::create(&addon)?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::<()>::default().compression_method(CompressionMethod::Deflated);

    for pack in [&bp_pack, &rp_pack] {
        if pack.exists() {
            zip.start_file(pack.file_name().unwrap().to_str().unwrap(), options)?;
            let mut f = File::open(pack)?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
        }
    }

    zip.finish()?;
    Ok(())
}

fn zip_dir(src: &Path, dst: &Path) -> Result<()> {
    let file = File::create(dst)?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::<()>::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    for entry in WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }

        let name = path
            .strip_prefix(src)?
            .to_str()
            .context("Non-UTF-8 path in zip")?
            .replace('\\', "/");

        zip.start_file(name, options)?;
        let mut f = File::open(path)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        zip.write_all(&buffer)?;
    }

    zip.finish()?;
    Ok(())
}
