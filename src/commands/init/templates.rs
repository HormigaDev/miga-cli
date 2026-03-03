/// Static template content and dynamic generators for project scaffolding.

// ── Placeholder icon ────────────────────────────────────────────────────────
pub const PACK_ICON_PNG: &[u8] = include_bytes!("../../pack_icon.png");

// ── Directory lists ─────────────────────────────────────────────────────────

pub const BEHAVIOR_BASE_DIRS: &[&str] = &[
    "behavior/blocks",
    "behavior/entities",
    "behavior/loot_tables",
    "behavior/features",
    "behavior/feature_rules",
    "behavior/trades",
    "behavior/recipes",
    "behavior/structures",
    "behavior/spawn_rules",
];

pub const BEHAVIOR_SCRIPT_DIRS: &[&str] = &[
    "behavior/scripts/core",
    "behavior/scripts/config",
    "behavior/scripts/features",
    "behavior/scripts/events",
    "behavior/scripts/components",
];

pub const RESOURCE_DIRS: &[&str] = &[
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

// ── Behavior manifest ───────────────────────────────────────────────────────

pub fn behavior_manifest_scripted(
    display_name: &str,
    bp_header: &str,
    bp_data: &str,
    bp_script: &str,
    rp_header: Option<&str>,
    mc_ver: &[u8; 3],
    scripting_ver: &str,
) -> String {
    let rp_dep = if let Some(rp) = rp_header {
        format!(
            r#"
        {{
            "uuid": "{}",
            "version": [0, 0, 1]
        }},"#,
            rp
        )
    } else {
        String::new()
    };

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
    "dependencies": [{rp_dep}
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
        rp_dep = rp_dep,
        v0 = mc_ver[0],
        v1 = mc_ver[1],
        v2 = mc_ver[2],
        sv = scripting_ver,
    )
}

pub fn behavior_manifest_scriptless(
    display_name: &str,
    bp_header: &str,
    bp_data: &str,
    rp_header: Option<&str>,
    mc_ver: &[u8; 3],
) -> String {
    let rp_dep = if let Some(rp) = rp_header {
        format!(
            r#",
    "dependencies": [
        {{
            "uuid": "{}",
            "version": [0, 0, 1]
        }}
    ]"#,
            rp
        )
    } else {
        String::new()
    };

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
        }}
    ]{rp_dep}
}}
"#,
        dn = display_name,
        bph = bp_header,
        bpd = bp_data,
        rp_dep = rp_dep,
        v0 = mc_ver[0],
        v1 = mc_ver[1],
        v2 = mc_ver[2],
    )
}

// ── Resource manifest ───────────────────────────────────────────────────────

pub fn resource_manifest(
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

// ── Licenses ────────────────────────────────────────────────────────────────

pub fn mit_license(author: &str, year: i32) -> String {
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

pub fn cc_by_sa_license(author: &str, year: i32) -> String {
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

// ── Resource textures ───────────────────────────────────────────────────────

pub fn item_texture_json(namespace: &str) -> String {
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

pub fn terrain_texture_json(namespace: &str) -> String {
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

// ── Scripts ─────────────────────────────────────────────────────────────────

pub fn registry_ts(namespace: &str, name: &str) -> String {
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

pub fn scripts_index() -> &'static str {
    r#"import { initEvents } from './events/index';

initEvents();
"#
}

pub fn events_index() -> &'static str {
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
}

// ── README ──────────────────────────────────────────────────────────────────

pub fn readme(display_name: &str, author: &str, has_bp: bool, has_rp: bool) -> String {
    let packs = match (has_bp, has_rp) {
        (true, true) => "both Behavior Pack and Resource Pack",
        (true, false) => "the Behavior Pack",
        (false, true) => "the Resource Pack",
        _ => "the pack",
    };

    let structure = match (has_bp, has_rp) {
        (true, true) => {
            r#"behavior/    Behavior Pack — logic, scripts, entities, spawn rules...
resource/    Resource Pack — textures, models, sounds, lang files...
.miga/       miga project config and installed module lock"#
        }
        (true, false) => {
            r#"behavior/    Behavior Pack — logic, scripts, entities, spawn rules...
.miga/       miga project config and installed module lock"#
        }
        (false, true) => {
            r#"resource/    Resource Pack — textures, models, sounds, lang files...
.miga/       miga project config and installed module lock"#
        }
        _ => ".miga/       miga project config",
    };

    let license_section = match (has_bp, has_rp) {
        (true, true) => format!(
            "- **Code** (behavior/): MIT — see [behavior/LICENSE](behavior/LICENSE)\n\
             - **Assets** (resource/): CC BY-SA 4.0 — see [resource/LICENSE](resource/LICENSE)"
        ),
        (true, false) => "- **Code** (behavior/): MIT — see [behavior/LICENSE](behavior/LICENSE)".to_string(),
        (false, true) => "- **Assets** (resource/): CC BY-SA 4.0 — see [resource/LICENSE](resource/LICENSE)".to_string(),
        _ => String::new(),
    };

    format!(
        r#"# {display_name}

> Created with [miga CLI](https://github.com/HormigaDev/miga) by {author}.

## Overview

<!-- Add a description of this addon here -->

## Installation

1. Download the latest release <!-- add link -->
2. Apply {packs} to your world.

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
{structure}
```

## License

{license_section}
"#,
        display_name = display_name,
        author = author,
        packs = packs,
        structure = structure,
        license_section = license_section,
    )
}

// ── Static content ──────────────────────────────────────────────────────────

pub const BLOCKS_JSON: &str = r#"{
    "format_version": [1, 1, 0]
}
"#;

pub const TSCONFIG: &str = r#"{
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

pub const GITIGNORE: &str = r#"# Dependencies
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

pub const ENV_TEMPLATE: &str = r#"# ── Deploy paths ─────────────────────────────────────────────────────────────
# Absolute path to Minecraft's development_behavior_packs folder
BEHAVIOR_PACKS_PATH=

# Absolute path to Minecraft's development_resource_packs folder
RESOURCE_PACKS_PATH=

# ── Build ─────────────────────────────────────────────────────────────────────
# true = inline source maps (slower, for debugging only)
SOURCE_MAPS=false
"#;
