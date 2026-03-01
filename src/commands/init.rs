use anyhow::{anyhow, Context, Result};
use chrono::Datelike;
use dialoguer::Input;
use serde_json;
use std::path::PathBuf;
use uuid::Uuid;

use crate::registry::manifest::ProjectManifest;
use crate::utils::{fs, output};

// Placeholder PNG (1×1 px orange — replace with real pack_icon)
const PACK_ICON_PNG: &[u8] = include_bytes!("../pack_icon.png");

const BEHAVIOR_DIRS: &[&str] = &[
    "behavior/blocks",
    "behavior/entities",
    "behavior/loot_tables",
    "behavior/features",
    "behavior/feature_rules",
    "behavior/trades",
    "behavior/recipes",
    "behavior/structures",
    "behavior/spawn_rules",
    "behavior/scripts/core",
    "behavior/scripts/config",
    "behavior/scripts/features",
    "behavior/scripts/events",
    "behavior/scripts/components",
];

const RESOURCE_DIRS: &[&str] = &[
    "resource/models/entity",
    "resource/models/blocks",
    "resource/texts",
    "resource/textures/blocks",
    "resource/textures/items",
    "resource/textures/entity",
    "resource/textures/ui",
    "resource/sounds",
    "resource/ui",
];

pub fn run(namespace: Option<String>, name: Option<String>) -> Result<()> {
    output::section("🐜 miga — Creating Bedrock Addon Environment");

    let namespace = resolve_or_ask(namespace, "Addon namespace (e.g. woc)")?;
    let name = resolve_or_ask(name, "Addon name (e.g. ecological-spawns)")?;
    let display_name: String = Input::new()
        .with_prompt("Display name (shown in Minecraft)")
        .default(name.clone())
        .interact_text()
        .context("Failed to read display name")?;
    let author: String = Input::new()
        .with_prompt("Author / organization")
        .default("HormigaDev".to_string())
        .interact_text()
        .context("Failed to read author")?;
    let mc_version: String = Input::new()
        .with_prompt("Min engine version")
        .default("1.21.0".to_string())
        .interact_text()
        .context("Failed to read engine version")?;
    let scripting_version: String = Input::new()
        .with_prompt("@minecraft/server version")
        .default("2.4.0".to_string())
        .interact_text()
        .context("Failed to read scripting version")?;

    validate_namespace(&namespace)?;
    validate_name(&name)?;
    let mc_ver = parse_version(&mc_version)?;
    let year = chrono::Local::now().year();

    let safe_name = name.replace(' ', "-").to_lowercase();
    let root = PathBuf::from(&safe_name);

    if root.exists() {
        return Err(anyhow!("Directory '{}' already exists.", safe_name));
    }

    std::fs::create_dir_all(&root)
        .with_context(|| format!("Cannot create directory '{}'", safe_name))?;
    std::env::set_current_dir(&root)
        .with_context(|| format!("Cannot enter directory '{}'", safe_name))?;

    let bp_header_uuid = Uuid::new_v4().to_string();
    let bp_data_uuid = Uuid::new_v4().to_string();
    let bp_script_uuid = Uuid::new_v4().to_string();
    let rp_header_uuid = Uuid::new_v4().to_string();
    let rp_module_uuid = Uuid::new_v4().to_string();

    output::section("Creating directories");
    for dir in BEHAVIOR_DIRS.iter().chain(RESOURCE_DIRS.iter()) {
        fs::ensure_dir(dir)?;
        output::step(dir);
    }
    fs::ensure_dir(".miga")?;

    output::section("Behavior Pack");
    write_new(
        "behavior/manifest.json",
        &behavior_manifest(
            &display_name,
            &bp_header_uuid,
            &bp_data_uuid,
            &bp_script_uuid,
            &rp_header_uuid,
            &mc_ver,
            &scripting_version,
        ),
    )?;
    write_new("behavior/LICENSE", &mit_license(&author, year))?;
    write_bytes_new("behavior/pack_icon.png", PACK_ICON_PNG)?;
    write_new("behavior/scripts/index.ts", &scripts_index())?;
    write_new("behavior/scripts/events/index.ts", &events_index())?;
    write_new(
        "behavior/scripts/config/registry.ts",
        &registry_ts(&namespace, &name),
    )?;

    output::section("Resource Pack");
    write_new(
        "resource/manifest.json",
        &resource_manifest(&display_name, &rp_header_uuid, &rp_module_uuid, &mc_ver),
    )?;
    write_new("resource/LICENSE", &cc_by_sa_license(&author, year))?;
    write_bytes_new("resource/pack_icon.png", PACK_ICON_PNG)?;
    write_new("resource/blocks.json", BLOCKS_JSON)?;
    write_new(
        "resource/textures/item_texture.json",
        &item_texture_json(&namespace),
    )?;
    write_new(
        "resource/textures/terrain_texture.json",
        &terrain_texture_json(&namespace),
    )?;
    write_new("resource/texts/en_US.lang", "")?;
    write_new("resource/texts/es_ES.lang", "")?;
    write_new("resource/texts/pt_BR.lang", "")?;

    output::section("Project root");
    write_new("LICENSE", &mit_license(&author, year))?;
    write_new("README.md", &readme(&display_name, &author))?;
    write_new(".env", ENV_TEMPLATE)?;
    write_new(".env.template", ENV_TEMPLATE)?;
    write_new(".gitignore", GITIGNORE)?;
    write_new("tsconfig.json", TSCONFIG)?;

    output::section(".miga");
    let manifest = ProjectManifest::new(&name, &namespace);
    let manifest_str =
        serde_json::to_string_pretty(&manifest).context("Failed to serialize project manifest")?;
    write_new(".miga/miga.json", &manifest_str)?;
    write_new(".miga/modules.lock", "{\"modules\":{}}")?;

    output::section("Installing TypeScript types");

    crate::commands::add::run(vec![
        format!("@minecraft/server@{}", scripting_version),
        "@minecraft/common".to_string(),
    ])?;

    output::success(&format!(
        "Addon '{}' ready in './{}/'\n\n  \
        Next steps:\n  \
        1. cd {}\n  \
        2. miga fetch <module>  — adds modules from the registry\n  \
        3. miga build           — compiles TypeScript and packages the addon\n  \
        4. Replace pack_icon.png in behavior/ and resource/",
        display_name, safe_name, safe_name
    ));

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

fn resolve_or_ask(value: Option<String>, prompt: &str) -> Result<String> {
    match value {
        Some(v) => Ok(v),
        None => Input::new()
            .with_prompt(prompt)
            .interact_text()
            .context("Failed to read input"),
    }
}

fn validate_namespace(ns: &str) -> Result<()> {
    if ns.is_empty() {
        return Err(anyhow!("Namespace cannot be empty."));
    }
    if ns.chars().any(|c| c == ' ' || c == ':' || c.is_uppercase()) {
        return Err(anyhow!(
            "Namespace '{}' is invalid. Only lowercase letters, numbers and hyphens.",
            ns
        ));
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow!("Name cannot be empty."));
    }
    Ok(())
}

fn parse_version(v: &str) -> Result<[u8; 3]> {
    let p: Vec<&str> = v.split('.').collect();
    if p.len() != 3 {
        return Err(anyhow!("Version '{}' must be X.Y.Z", v));
    }
    Ok([
        p[0].parse()
            .with_context(|| format!("Bad version: {}", v))?,
        p[1].parse()
            .with_context(|| format!("Bad version: {}", v))?,
        p[2].parse()
            .with_context(|| format!("Bad version: {}", v))?,
    ])
}

fn behavior_manifest(
    display_name: &str,
    bp_header: &str,
    bp_data: &str,
    bp_script: &str,
    rp_header: &str,
    mc_ver: &[u8; 3],
    scripting_ver: &str,
) -> String {
    format!(
        r#"{{
    "format_version": 2,
    "header": {{
        "description": "{dn}",
        "name": "{dn} BP",
        "uuid": "{bph}",
        "version": [0, 0, 1],
        "min_engine_version": [{v0}, {v1}, {v2}]
    }},
    "modules": [
        {{
            "description": "Logic",
            "type": "data",
            "uuid": "{bpd}",
            "version": [0, 0, 1]
        }},
        {{
            "description": "Scripts",
            "type": "script",
            "language": "javascript",
            "uuid": "{bps}",
            "version": [0, 0, 1],
            "entry": "scripts/index.js"
        }}
    ],
    "dependencies": [
        {{
            "uuid": "{rph}",
            "version": [0, 0, 1]
        }},
        {{
            "module_name": "@minecraft/server",
            "version": "{sv}"
        }}
    ]
}}
"#,
        dn = display_name,
        bph = bp_header,
        bpd = bp_data,
        bps = bp_script,
        rph = rp_header,
        v0 = mc_ver[0],
        v1 = mc_ver[1],
        v2 = mc_ver[2],
        sv = scripting_ver,
    )
}

fn resource_manifest(
    display_name: &str,
    rp_header: &str,
    rp_module: &str,
    mc_ver: &[u8; 3],
) -> String {
    format!(
        r#"{{
    "format_version": 2,
    "header": {{
        "description": "{dn}",
        "name": "{dn} RP",
        "uuid": "{rph}",
        "version": [0, 0, 1],
        "min_engine_version": [{v0}, {v1}, {v2}]
    }},
    "modules": [
        {{
            "description": "Resources",
            "type": "resources",
            "uuid": "{rpm}",
            "version": [0, 0, 1]
        }}
    ]
}}
"#,
        dn = display_name,
        rph = rp_header,
        rpm = rp_module,
        v0 = mc_ver[0],
        v1 = mc_ver[1],
        v2 = mc_ver[2],
    )
}

fn mit_license(author: &str, year: i32) -> String {
    format!(
        r#"MIT License

Copyright (c) {year} {author}

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"#,
        year = year,
        author = author
    )
}

fn cc_by_sa_license(author: &str, year: i32) -> String {
    format!(
        r#"Creative Commons Attribution-ShareAlike 4.0 International (CC BY-SA 4.0)

Copyright (c) {year} {author}

You are free to:
  Share  — copy and redistribute the material in any medium or format
  Adapt  — remix, transform, and build upon the material for any purpose,
            even commercially.

Under the following terms:
  Attribution  — You must give appropriate credit, provide a link to the
                 license, and indicate if changes were made.
  ShareAlike   — If you remix, transform, or build upon the material, you must
                 distribute your contributions under the same license.

Full license: https://creativecommons.org/licenses/by-sa/4.0/legalcode
"#,
        year = year,
        author = author
    )
}

fn item_texture_json(namespace: &str) -> String {
    format!(
        r#"{{
    "resource_pack_name": "{namespace}",
    "texture_name": "atlas.items",
    "texture_data": {{}}
}}
"#,
        namespace = namespace
    )
}

fn terrain_texture_json(namespace: &str) -> String {
    format!(
        r#"{{
    "resource_pack_name": "{namespace}",
    "texture_name": "atlas.terrain",
    "padding": 8,
    "num_mip_levels": 4,
    "texture_data": {{}}
}}
"#,
        namespace = namespace
    )
}

fn registry_ts(namespace: &str, name: &str) -> String {
    format!(
        r#"/**
 * @module Registry
 * @namespace {namespace}
 * @addon {name}
 *
 * Central registry for this addon.
 * Import and initialize all features here.
 */
export const NAMESPACE = '{namespace}' as const;
"#,
        namespace = namespace,
        name = name
    )
}

fn scripts_index() -> String {
    r#"import { initEvents } from './events/index';

initEvents();
"#
    .to_string()
}

fn events_index() -> String {
    r#"
/**
 * Central event initializer.
 * Add new event initializers here as the addon grows.
 * Called exactly once from scripts/index.ts.
 */
export function initEvents(): void {
    // Your event here
}
"#
    .to_string()
}

fn readme(display_name: &str, author: &str) -> String {
    format!(
        r#"# {display_name}

> Created with [miga CLI](https://github.com/HormigaDev/miga) by {author}.

## Overview

<!-- Add a description of this addon here -->

## Installation

1. Download the latest release <!-- add link -->
2. Apply both Behavior Pack and Resource Pack to your world.

## Development

### Prerequisites
- [miga CLI](https://github.com/HormigaDev/miga)

### Setup

```bash
miga fetch <module>   # add utility modules from the registry
miga build            # compile TypeScript and package the addon
```

### Project structure

```
behavior/    Behavior Pack — logic, scripts, entities, spawn rules...
resource/    Resource Pack — textures, models, sounds, lang files...
.miga/       miga project config and installed module lock
```

## License

- **Code** (behavior/): MIT — see [behavior/LICENSE](behavior/LICENSE)
- **Assets** (resource/): CC BY-SA 4.0 — see [resource/LICENSE](resource/LICENSE)
"#,
        display_name = display_name,
        author = author
    )
}

const BLOCKS_JSON: &str = r#"{
    "format_version": [1, 1, 0]
}
"#;

const TSCONFIG: &str = r#"{
    "compilerOptions": {
        "module": "ES2020",
        "target": "ES2020",
        "moduleResolution": "bundler",
        "allowSyntheticDefaultImports": true,
        "strict": true,
        "noUnusedLocals": true,
        "noUnusedParameters": true
    },
    "include": ["behavior/scripts/**/*.ts"],
    "exclude": []
}
"#;

const GITIGNORE: &str = r#"# Dependencies
.miga_modules/

# Environment (keep .env.template, ignore .env)
.env

# Compiled output — managed by miga build
behavior/scripts/index.js
behavior/scripts/index.js.map

# Rust build artifacts (miga CLI development)
target/

# Build output
dist/

# OS
.DS_Store
Thumbs.db
"#;

const ENV_TEMPLATE: &str = r#"# ── Deploy paths ─────────────────────────────────────────────────────────────
# Absolute path to Minecraft's development_behavior_packs folder
BEHAVIOR_PACKS_PATH=

# Absolute path to Minecraft's development_resource_packs folder
RESOURCE_PACKS_PATH=

# ── Build ─────────────────────────────────────────────────────────────────────
# true = inline source maps (slower, for debugging only)
SOURCE_MAPS=false
"#;
