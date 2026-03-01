use crate::registry::manifest::{LockFile, ProjectManifest};
use crate::utils::fs;
use anyhow::{Context, Result};
use serde_json::{json, Map, Value};

/// Regenerates `tsconfig.json` with path mappings derived from the current project manifest
/// and lock file, covering both npm external types and miga registry modules.
///
/// For registry modules, the version from `ProjectManifest.modules` is used to
/// select the correct versioned directory under `.miga_modules/<name>/v<version>/`.
pub fn update(project: &ProjectManifest, lock: &LockFile) -> Result<()> {
    let tsconfig_path = "tsconfig.json";

    let mut tsconfig: Value = if fs::exists(tsconfig_path) {
        let content = fs::read_to_string(tsconfig_path)?;
        serde_json::from_str(&content).context("Failed to parse tsconfig.json")?
    } else {
        json!({
            "compilerOptions": {
                "target": "ES2020",
                "module": "ES2020",
                "moduleResolution": "node",
                "strict": true,
                "noEmit": true,
                "baseUrl": "."
            },
            "include": ["behavior/scripts/**/*.ts"],
            "exclude": [".miga_modules", "dist", "build"]
        })
    };

    if let Some(opts) = tsconfig
        .get_mut("compilerOptions")
        .and_then(|v| v.as_object_mut())
    {
        let mut paths = Map::new();

        for name in project.externals.keys() {
            paths.insert(
                name.clone(),
                json!([format!("./.miga_modules/{}/index.d.ts", name)]),
            );
        }

        for (name, versions) in &lock.modules {
            // Prefer the version pinned in the manifest; fall back to the
            // first available version in the lock file.
            let version = project
                .modules
                .get(name)
                .or_else(|| versions.keys().next())
                .cloned()
                .unwrap_or_default();

            if let Some(locked) = versions.get(&version) {
                paths.insert(
                    name.clone(),
                    json!([format!(
                        "./.miga_modules/{}/v{}/{}",
                        name, version, locked.entry
                    )]),
                );
                paths.insert(
                    format!("{}/*", name),
                    json!([format!("./.miga_modules/{}/v{}/*", name, version)]),
                );
            }
        }

        opts.insert("paths".to_string(), Value::Object(paths));
    }

    fs::write_force(tsconfig_path, &serde_json::to_string_pretty(&tsconfig)?)?;
    Ok(())
}
