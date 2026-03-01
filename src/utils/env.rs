use anyhow::{anyhow, Result};
use std::path::PathBuf;

/// Resolved paths to the Minecraft development pack directories.
pub struct DeployPaths {
    pub behavior: PathBuf,
    pub resource: PathBuf,
}

/// Resolves deployment destinations for behavior and resource packs in priority order:
///   1. `.env` variables: `BEHAVIOR_PACKS_PATH` / `RESOURCE_PACKS_PATH`
///   2. Windows: Minecraft UWP installation path (automatic fallback)
///   3. All other platforms: explicit error directing the user to configure `.env`
pub fn resolve_deploy_paths(addon_name: &str) -> Result<DeployPaths> {
    let _ = dotenvy::dotenv();

    let bp_env = std::env::var("BEHAVIOR_PACKS_PATH").ok();
    let rp_env = std::env::var("RESOURCE_PACKS_PATH").ok();

    if let (Some(bp), Some(rp)) = (bp_env, rp_env) {
        return Ok(DeployPaths {
            behavior: PathBuf::from(bp).join(addon_name),
            resource: PathBuf::from(rp).join(addon_name),
        });
    }

    #[cfg(target_os = "windows")]
    if let Some(local_app_data) = dirs::data_local_dir() {
        let base = local_app_data
            .join("Packages")
            .join("Microsoft.MinecraftUWP_8wekyb3d8bbwe")
            .join("LocalState")
            .join("games")
            .join("com.mojang");

        if base.exists() {
            return Ok(DeployPaths {
                behavior: base.join("development_behavior_packs").join(addon_name),
                resource: base.join("development_resource_packs").join(addon_name),
            });
        }
    }

    Err(anyhow!(
        "Deploy paths not configured.\n\
         Copy .env.template to .env and set BEHAVIOR_PACKS_PATH and RESOURCE_PACKS_PATH."
    ))
}
