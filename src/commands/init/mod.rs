mod prompts;
mod templates;

use anyhow::{anyhow, Context, Result};
use chrono::Datelike;
use uuid::Uuid;

use crate::cli::ProjectType;
use crate::registry::manifest::ProjectManifest;
use crate::utils::{fs, output};

pub fn run(
    namespace: Option<String>,
    name: Option<String>,
    project_type: Option<ProjectType>,
    yes: bool,
) -> Result<()> {
    output::section("🐜 miga — Creating Bedrock Addon Environment");

    let config = prompts::collect_config(namespace, name, project_type, yes)?;
    let pt = config.project_type;
    let year = chrono::Local::now().year();

    let safe_name = config.name.replace(' ', "-").to_lowercase();
    let root = std::path::PathBuf::from(&safe_name);

    if root.exists() {
        return Err(anyhow!("Directory '{}' already exists.", safe_name));
    }

    std::fs::create_dir_all(&root)
        .with_context(|| format!("Cannot create directory '{}'", safe_name))?;
    std::env::set_current_dir(&root)
        .with_context(|| format!("Cannot enter directory '{}'", safe_name))?;

    // Generate all UUIDs upfront
    let bp_header_uuid = Uuid::new_v4().to_string();
    let bp_data_uuid = Uuid::new_v4().to_string();
    let bp_script_uuid = Uuid::new_v4().to_string();
    let rp_header_uuid = Uuid::new_v4().to_string();
    let rp_module_uuid = Uuid::new_v4().to_string();

    // ── Directories ─────────────────────────────────────────────────────
    output::section("Creating directories");

    if pt.has_behavior() {
        create_dirs(templates::BEHAVIOR_BASE_DIRS)?;
        if pt.has_scripts() {
            create_dirs(templates::BEHAVIOR_SCRIPT_DIRS)?;
        }
    }
    if pt.has_resource() {
        create_dirs(templates::RESOURCE_DIRS)?;
    }
    fs::ensure_dir(".miga")?;

    // ── Behavior Pack ───────────────────────────────────────────────────
    if pt.has_behavior() {
        output::section("Behavior Pack");

        let rp_ref = if pt.has_resource() {
            Some(rp_header_uuid.as_str())
        } else {
            None
        };

        let bp_manifest = if pt.has_scripts() {
            templates::behavior_manifest_scripted(
                &config.display_name,
                &bp_header_uuid,
                &bp_data_uuid,
                &bp_script_uuid,
                rp_ref,
                &config.mc_version,
                &config.scripting_version,
            )
        } else {
            templates::behavior_manifest_scriptless(
                &config.display_name,
                &bp_header_uuid,
                &bp_data_uuid,
                rp_ref,
                &config.mc_version,
            )
        };

        write_new("behavior/manifest.json", &bp_manifest)?;
        write_new(
            "behavior/LICENSE",
            &templates::mit_license(&config.author, year),
        )?;
        write_bytes_new("behavior/pack_icon.png", templates::PACK_ICON_PNG)?;

        if pt.has_scripts() {
            write_new("behavior/scripts/index.ts", templates::scripts_index())?;
            write_new(
                "behavior/scripts/events/index.ts",
                templates::events_index(),
            )?;
            write_new(
                "behavior/scripts/config/registry.ts",
                &templates::registry_ts(&config.namespace, &config.name),
            )?;
        }
    }

    // ── Resource Pack ───────────────────────────────────────────────────
    if pt.has_resource() {
        output::section("Resource Pack");

        write_new(
            "resource/manifest.json",
            &templates::resource_manifest(
                &config.display_name,
                &rp_header_uuid,
                &rp_module_uuid,
                &config.mc_version,
            ),
        )?;
        write_new(
            "resource/LICENSE",
            &templates::cc_by_sa_license(&config.author, year),
        )?;
        write_bytes_new("resource/pack_icon.png", templates::PACK_ICON_PNG)?;
        write_new("resource/blocks.json", templates::BLOCKS_JSON)?;
        write_new(
            "resource/textures/item_texture.json",
            &templates::item_texture_json(&config.namespace),
        )?;
        write_new(
            "resource/textures/terrain_texture.json",
            &templates::terrain_texture_json(&config.namespace),
        )?;
        write_new("resource/texts/en_US.lang", "")?;
        write_new("resource/texts/es_ES.lang", "")?;
        write_new("resource/texts/pt_BR.lang", "")?;
    }

    // ── Project root ────────────────────────────────────────────────────
    output::section("Project root");
    write_new("LICENSE", &templates::mit_license(&config.author, year))?;
    write_new(
        "README.md",
        &templates::readme(
            &config.display_name,
            &config.author,
            pt.has_behavior(),
            pt.has_resource(),
        ),
    )?;
    write_new(".env", templates::ENV_TEMPLATE)?;
    write_new(".env.template", templates::ENV_TEMPLATE)?;
    write_new(".gitignore", templates::GITIGNORE)?;

    if pt.has_scripts() {
        write_new("tsconfig.json", templates::TSCONFIG)?;
    }

    // ── .miga ───────────────────────────────────────────────────────────
    output::section(".miga");
    let manifest = ProjectManifest::new(&config.name, &config.namespace);
    let manifest_str =
        serde_json::to_string_pretty(&manifest).context("Failed to serialize project manifest")?;
    write_new(".miga/miga.json", &manifest_str)?;
    write_new(".miga/modules.lock", "{\"modules\":{}}")?;

    // ── TypeScript types ────────────────────────────────────────────────
    if pt.has_scripts() {
        output::section("Installing TypeScript types");
        crate::commands::add::run(vec![
            format!("@minecraft/server@{}", config.scripting_version),
            "@minecraft/common".to_string(),
        ])?;
    }

    // ── Done ────────────────────────────────────────────────────────────
    let next_steps = build_next_steps(&safe_name, pt);
    output::success(&format!(
        "Addon '{}' ready in './{}/'\n\n{}",
        config.display_name, safe_name, next_steps
    ));

    Ok(())
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn create_dirs(dirs: &[&str]) -> Result<()> {
    for dir in dirs {
        fs::ensure_dir(dir)?;
        output::step(dir);
    }
    Ok(())
}

fn write_new(path: &str, content: &str) -> Result<()> {
    let created = fs::write_if_not_exists(path, content)?;
    if created {
        output::step(&format!("created  {}", path));
    } else {
        output::step(&format!("skipped  {} (already exists)", path));
    }
    Ok(())
}

fn write_bytes_new(path: &str, bytes: &[u8]) -> Result<()> {
    if fs::exists(path) {
        output::step(&format!("skipped  {} (already exists)", path));
        return Ok(());
    }
    std::fs::write(path, bytes).with_context(|| format!("Failed to write: {}", path))?;
    output::step(&format!("created  {}", path));
    Ok(())
}

fn build_next_steps(safe_name: &str, pt: ProjectType) -> String {
    let mut steps = vec![format!("  1. cd {}", safe_name)];
    let mut n = 2;

    if pt.has_scripts() {
        steps.push(format!(
            "  {}. miga fetch <module>  — adds modules from the registry",
            n
        ));
        n += 1;
    }

    steps.push(format!(
        "  {}. miga build           — compiles and packages the addon",
        n
    ));
    n += 1;

    if pt.has_behavior() {
        steps.push(format!("  {}. Replace pack_icon.png in behavior/", n));
        n += 1;
    }
    if pt.has_resource() {
        steps.push(format!("  {}. Replace pack_icon.png in resource/", n));
    }

    format!("  Next steps:\n{}", steps.join("\n"))
}
