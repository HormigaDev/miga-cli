use anyhow::{Context, Result};
use notify::{EventKind, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::compiler::{CompileOptions, DependencyInfo};
use crate::utils::{builder, env, output, project};

const DEBOUNCE_MS: u64 = 150;

fn source_maps_enabled() -> bool {
    let _ = dotenvy::dotenv();
    std::env::var("SOURCE_MAPS")
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false)
}

pub fn run(watch: bool) -> Result<()> {
    let manifest = builder::load_project()?;
    let lock = project::load_lock()?;
    let paths =
        env::resolve_deploy_paths(&manifest.name).context("Failed to resolve deploy paths")?;
    let dep_versions = builder::user_dep_versions(&manifest, &lock);
    let source_maps = source_maps_enabled();

    output::section("miga run — development mode");

    crate::utils::fs::clean_dir(&paths.behavior)?;
    crate::utils::fs::clean_dir(&paths.resource)?;

    deploy(&paths, &dep_versions, source_maps)?;

    if !watch {
        return Ok(());
    }

    output::info("Watching for changes... (Ctrl+C to stop)");
    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(Path::new("behavior"), RecursiveMode::Recursive)?;
    watcher.watch(Path::new("resource"), RecursiveMode::Recursive)?;

    let mut last_event = Instant::now();
    for res in rx {
        if let Ok(event) = res {
            if matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
            ) {
                if last_event.elapsed() < Duration::from_millis(DEBOUNCE_MS) {
                    continue;
                }
                last_event = Instant::now();
                if let Err(e) = deploy(&paths, &dep_versions, source_maps) {
                    output::error(&format!("Deploy error: {}", e));
                }
            }
        }
    }
    Ok(())
}

fn deploy(
    paths: &env::DeployPaths,
    dep_versions: &std::collections::HashMap<String, DependencyInfo>,
    source_maps: bool,
) -> Result<()> {
    let lock = project::load_lock()?;

    let opts = CompileOptions {
        minify: false,
        source_maps,
        script_root: PathBuf::from("behavior/scripts"),
        dep_versions: dep_versions.clone(),
    };

    builder::process_behavior(Path::new("behavior"), &paths.behavior, &opts)?;
    builder::process_dependencies(
        Path::new(".miga_modules"),
        &paths.behavior.join("scripts/libs"),
        &opts,
        &lock,
    )?;
    builder::process_resource(Path::new("resource"), &paths.resource, false)?;

    output::success("synced with game.");
    Ok(())
}
