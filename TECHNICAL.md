# 🐜 miga — Technical Reference

This document describes the internal architecture of `miga` for contributors
and maintainers.

---

## 📑 Table of Contents

- [High-level architecture](#-high-level-architecture)
- [Binary entry point](#-binary-entry-point)
- [CLI layer](#-cli-layer)
- [Commands](#-commands)
- [Compiler pipeline](#-compiler-pipeline)
- [Registry protocol](#-registry-protocol)
- [Version conflict resolution](#-version-conflict-resolution)
- [Shared utilities](#-shared-utilities)
- [File layout](#-file-layout)
- [Key data structures](#-key-data-structures)

---

## 🏗️ High-level architecture

```
┌─────────────────────────────────────────────────┐
│                  miga binary                    │
│                                                 │
│  main.rs  ──►  cli.rs (clap)                    │
│                   │                             │
│          ┌────────┴─────────────────────────┐   │
│          │        commands/                 │   │
│          │  init · add · fetch · run ·      │   │
│          │  build · remove                  │   │
│          └────┬───────────────┬─────────────┘   │
│               │               │                 │
│         compiler/        registry/              │
│         (oxc pipeline)   (manifest + HTTP)      │
│               │               │                 │
│          utils/ ─────────────────────────────── │
│          fs · json · npm · env · builder ·      │
│          output · tsconfig · net · project      │
└─────────────────────────────────────────────────┘
```

All commands return `anyhow::Result<()>`. Errors bubble up to `main.rs` where a
single `output::error()` call formats and exits.

---

## 🚀 Binary entry point

**`src/main.rs`**

1. Parses the CLI with `Cli::parse()` (clap derive).
2. Matches on `Commands` and delegates to the appropriate `commands::*::run()`.
3. On error: prints a formatted message via `utils::output::error()` and exits
   with code 1.

---

## ⌨️ CLI layer

**`src/cli.rs`**

Defines `Cli` and `Commands` using clap's derive macros.

| Subcommand | Struct variant                                          |
| ---------- | ------------------------------------------------------- |
| `init`     | `Commands::Init { namespace, name, project_type, yes }` |
| `add`      | `Commands::Add { packages }`                            |
| `fetch`    | `Commands::Fetch { module, version, update }`           |
| `run`      | `Commands::Run { no_watch }`                            |
| `build`    | `Commands::Build`                                       |
| `remove`   | `Commands::Remove { packages, all }`                    |

### `ProjectType` enum

```rust
#[derive(ValueEnum)]
pub enum ProjectType {
    Full,              // BP + scripts + RP (default)
    Behavior,          // BP + scripts only
    BehaviorScriptless,// BP data-driven only
    AddonScriptless,   // BP + RP, both without scripts
    Resource,          // RP only
}
```

Helper methods: `has_behavior()`, `has_resource()`, `has_scripts()`.

---

## 🔧 Commands

### 🆕 `init` (`src/commands/init/`)

Refactored into a module directory with three files:

| File           | Responsibility                                                        |
| -------------- | --------------------------------------------------------------------- |
| `mod.rs`       | Main `run()` orchestration with project-type-aware conditional logic. |
| `prompts.rs`   | Interactive prompts, validation, `InitConfig` struct, defaults.       |
| `templates.rs` | All template content: manifests, licenses, README, static strings.    |

**Flow:**

1. `prompts::collect_config()` gathers metadata — either interactively
   (`dialoguer::Input` / `dialoguer::Select`) or from CLI args + defaults
   when `--yes` is set.
2. Validates namespace (lowercase, no spaces/colons) and name.
3. Creates the project directory and sets it as the working directory.
4. Based on `ProjectType`:
    - **`has_behavior()`** → creates BP directory tree + manifest (scripted or
      scriptless depending on `has_scripts()`).
    - **`has_scripts()`** → creates `scripts/` tree, `tsconfig.json`,
      entry-point files, and calls `commands::add::run()` to install
      `@minecraft/server`.
    - **`has_resource()`** → creates RP directory tree + manifest + texture JSONs.
5. Writes `.miga/miga.json` (project manifest) and `.miga/modules.lock`.
6. Generates `.env`, `.env.template`, `.gitignore`, `LICENSE`, `README.md`.

**Templates** are split into:

- `const` strings for static content (`TSCONFIG`, `GITIGNORE`, `PACK_ICON_PNG`, etc.)
- `fn` helpers for dynamic content (`behavior_manifest_scripted()`,
  `behavior_manifest_scriptless()`, `resource_manifest()`, `readme()`, etc.)

---

### 📘 `add` (`src/commands/add.rs`)

Downloads TypeScript type packages from the npm registry.

1. `utils::net::is_online()` — connectivity check.
2. `utils::project::require_initialized()` — ensures `.miga/` exists.
3. For each package spec: calls `utils::npm::fetch_types()`.
4. Records `name → version` in `.miga/miga.json` under `externals`.
5. Updates `behavior/manifest.json` dependencies block via
   `sync_behavior_manifest()`.

---

### 📦 `fetch` (`src/commands/fetch.rs`)

Installs modules from the miga registry with **version conflict resolution**.

1. Connectivity check.
2. `utils::project::require_initialized()`.
3. Supports multiple modes:
    - **No arguments** → installs all modules listed in `miga.json`.
    - **Module name** → installs that specific module (latest or `--version`).
    - **`--update`** → updates one or all modules to their latest version.
4. For each module: resolves the full dependency tree via
   `registry::resolve_dependencies()`, which returns `Vec<ResolvedModule>` with
   populated `resolved_deps` maps.
5. For each resolved module, checks for **version conflicts** against the
   existing lock file (see [Version conflict resolution](#-version-conflict-resolution)).
6. Downloads the archive and extracts to `.miga_modules/<name>/v<version>/`.
7. Records the module in `.miga/modules.lock` with its `resolved_deps`.

---

### 🔄 `run` (`src/commands/run.rs`)

Hot-reload watch loop.

1. Loads the project manifest and lock file via `builder::load_project()`
   and `project::load_lock()`.
2. Reads `SOURCE_MAPS` from `.env` via `dotenvy` (`true`/`1` = enabled).
3. Builds `dep_versions` from `builder::user_dep_versions()`.
4. Compiles and deploys:
    - `builder::process_behavior()` — compiles user TypeScript scripts
      (writes `.js.map` alongside `.js` when source maps are enabled).
    - `builder::process_resource()` — copies resource pack files.
    - `builder::process_dependencies()` — compiles versioned module code.
    - Copies outputs to Minecraft dev pack folders.
5. If `--no-watch` is set, exits. Otherwise, starts
   `notify::RecommendedWatcher` on `behavior/scripts/` and re-deploys
   on every change event.

---

### 📦 `build` (`src/commands/build.rs`)

Full compile and package for release.

1. Loads project manifest and lock file.
2. Builds with minification enabled and source maps disabled:
    - `builder::process_behavior()` — TypeScript → minified JS.
    - `builder::process_resource()` — copies + minifies JSON.
    - `builder::process_dependencies()` — compiles module deps.
3. Syncs version from `.miga/miga.json` into both manifests.
4. `dist/` is cleaned, then `zip` archives the BP and RP into `.mcpack` files
   and combines both into a `.mcaddon`.

---

### 🗑️ `remove` (`src/commands/remove.rs`)

Removes modules and/or external packages.

1. `utils::project::require_initialized()`.
2. Supports removing multiple packages at once, or `--all` to remove everything.
3. Detects whether a package is a registry module or an npm external.
4. Removes the module's versioned directories from `.miga_modules/`.
5. Removes the dependency edge from `behavior/manifest.json`.
6. Updates `.miga/modules.lock`, `.miga/miga.json` and `tsconfig.json`.

---

## ⚡ Compiler pipeline

**`src/compiler/mod.rs`**

Uses the [oxc](https://oxc.rs/) family of crates for zero-Node.js TypeScript
compilation.

```
.ts source
    │
    ▼
oxc_parser::Parser::parse()          — produces AST
    │
    ▼
oxc_semantic::SemanticBuilder        — scope & binding analysis, produces
                                       the Scoping passed to the Transformer
    │
    ▼
oxc_transformer::Transformer         — TypeScript → ES2020 JS
    │
    ▼
(optional) oxc_minifier::Minifier    — dead-code elimination + mangling
    │
    ▼
oxc_codegen::Codegen                 — emit final JS string
    │                                    (+ optional SourceMap when enabled)
    ▼
CompileResult { code, source_map }   — returned to caller
```

### 🗺️ Source maps

When `CompileOptions::source_maps` is `true`, the codegen sets
`CodegenOptions::source_map_path` which activates oxc's internal
`SourcemapBuilder`. The resulting `SourceMap` is serialized via
`to_json_string()` and a `//# sourceMappingURL=<file>.map` comment
is appended to the output code.

- **`run`** — reads `SOURCE_MAPS` from `.env` (`true`/`1` = enabled).
- **`build`** — always disables source maps (release mode).

### 🔗 Import rewriting

The compiler rewrites import specifiers based on the `dep_versions` map
in `CompileOptions`:

| Import type                     | Rewrite rule                                                            |
| ------------------------------- | ----------------------------------------------------------------------- |
| Relative (`./`, `../`)          | `.ts` extension → `.js`                                                 |
| `minecraft:*`                   | Left as-is (engine-provided)                                            |
| Bare specifier (e.g. `"bimap"`) | Rewritten to `./libs/<name>/v<version>/<entry>.js` using `dep_versions` |

This ensures each module always resolves to the exact version it was installed
with, even when multiple versions coexist.

---

## 🌐 Registry protocol

**`src/registry/mod.rs`**

The registry base URL is read from `MIGA_REGISTRY_URL` (falls back to a
hard-coded default). Two HTTP endpoints are used:

| Endpoint                            | Purpose                                       |
| ----------------------------------- | --------------------------------------------- |
| `GET /registry.json`                | Global registry manifest listing all modules. |
| `GET /modules/<name>/<name>.tar.gz` | Module archive download.                      |

**`src/registry/manifest.rs`** defines `ProjectManifest` (`.miga/miga.json`),
`ModuleManifest` (per-module descriptor inside archives), `LockFile`,
`LockedModule`, `ResolvedModule`, and helper functions for dependency spec
parsing and semver comparison.

---

## 🔀 Version conflict resolution

When `fetch` encounters a module that is already installed at a different
version, it applies the following logic:

| Scenario                              | Default action                                 | User prompt        |
| ------------------------------------- | ---------------------------------------------- | ------------------ |
| **Same version** already installed    | ⏭️ Skip silently                               | —                  |
| **Same major**, different minor/patch | ⬆️ Prompt: upgrade / keep both / keep existing | Yes                |
| **Different major** (breaking change) | 📦 Keep both versions                          | Prompt to override |

This is implemented via the `ConflictAction` enum and `check_version_conflict()`
in `src/commands/fetch.rs`. The lock file supports multiple versions per module
name natively (see [Key data structures](#-key-data-structures)).

---

## 🧰 Shared utilities

| Module              | Key responsibilities                                                                                                                             |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `utils/project.rs`  | `require_initialized`, `load_manifest`, `save_manifest`, `load_lock`, `save_lock` — single source of truth for project I/O.                      |
| `utils/net.rs`      | `is_online()` — TCP probe to `1.1.1.1:53` with a 1500 ms timeout.                                                                                |
| `utils/builder.rs`  | `load_project()`, `process_behavior()`, `process_resource()`, `process_dependencies()`, `user_dep_versions()` — orchestrates compile and deploy. |
| `compiler/mod.rs`   | oxc-based TypeScript → JS pipeline with versioned import rewriting.                                                                              |
| `utils/env.rs`      | Resolves `DeployPaths` from `.env` or platform defaults.                                                                                         |
| `utils/fs.rs`       | `ensure_dir`, `clean_dir`, `copy_dir`, `exists`, `write_if_not_exists`, `write_force`, `copy_force`.                                             |
| `utils/json.rs`     | `read_json`, `write_json`, `minify`, `to_unicode_escapes`.                                                                                       |
| `utils/npm.rs`      | Downloads `.d.ts` files from the npm registry.                                                                                                   |
| `utils/output.rs`   | Consistent terminal output helpers (`section`, `step`, `success`, `error`, `warn`).                                                              |
| `utils/tsconfig.rs` | Writes the TypeScript path-mapping `tsconfig.json` for module resolution (versioned paths).                                                      |

---

## 📁 File layout

```
src/
├── main.rs                 🚀 Entry point
├── cli.rs                  ⌨️  clap command definitions
├── commands/
│   ├── mod.rs              Re-exports
│   ├── init/               🆕 Project scaffolding
│   │   ├── mod.rs          Orchestration + project-type logic
│   │   ├── prompts.rs      Interactive prompts & validation
│   │   └── templates.rs    Template content & static strings
│   ├── add.rs              📘 npm type packages
│   ├── fetch.rs            📦 Registry module installer
│   ├── run.rs              🔄 Hot-reload dev loop
│   ├── build.rs            📦 Release packaging
│   └── remove.rs           🗑️  Package removal
├── compiler/
│   └── mod.rs              ⚡ oxc TypeScript → JS pipeline
├── registry/
│   ├── mod.rs              🌐 HTTP registry client
│   └── manifest.rs         📋 Data types & semver helpers
└── utils/
    ├── mod.rs              Module re-exports
    ├── builder.rs          🏗️ Orchestrates compile + deploy
    ├── env.rs              🔧 Deploy path resolution
    ├── fs.rs               📂 File-system helpers
    ├── json.rs             📋 JSON helpers
    ├── net.rs              🌐 Connectivity probe
    ├── npm.rs              📘 npm type downloader
    ├── output.rs           🖥️  Terminal output
    ├── project.rs          📋 Project manifest I/O
    └── tsconfig.rs         📘 tsconfig generator
```

---

## 📊 Key data structures

### `ProjectManifest` (`.miga/miga.json`)

```json
{
    "name": "my-addon",
    "namespace": "woc",
    "version": "0.1.0",
    "modules": { "<module-name>": "<version>" },
    "externals": { "@minecraft/server": "2.4.0" }
}
```

### `LockFile` (`.miga/modules.lock`)

The lock file uses a **nested versioned structure**: each module name maps to
a map of versions, allowing multiple versions to coexist.

```json
{
    "modules": {
        "bimap": {
            "1.0.0": {
                "entry": "index.ts",
                "files": ["index.ts", "utils.ts"],
                "resolved_deps": {}
            }
        },
        "utils": {
            "1.0.0": {
                "entry": "index.ts",
                "files": ["index.ts"],
                "resolved_deps": { "bimap": "1.0.0" }
            },
            "2.0.0": {
                "entry": "index.ts",
                "files": ["index.ts"],
                "resolved_deps": { "bimap": "1.0.0" }
            }
        }
    }
}
```

### `LockedModule`

```rust
pub struct LockedModule {
    pub entry: String,
    pub files: Vec<String>,
    /// dep_name → dep_version used by the compiler for import rewriting.
    pub resolved_deps: HashMap<String, String>,
}
```

### `ResolvedModule`

Returned by `registry::resolve_dependencies()` after resolving the full
dependency tree:

```rust
pub struct ResolvedModule {
    pub manifest: ModuleManifest,
    /// dep_name → resolved_version
    pub resolved_deps: HashMap<String, String>,
}
```

### `ModuleManifest` (inside each module archive)

```json
{
    "name": "module-name",
    "version": "1.0.0",
    "description": "...",
    "license": "MIT",
    "entry": "index.ts",
    "files": ["index.ts"],
    "dependencies": ["other-module@1.0.0"]
}
```

### `CompileOptions`

```rust
pub struct CompileOptions {
    pub minify: bool,
    pub source_maps: bool,
    pub script_root: PathBuf,
    /// Maps bare module names to dependency info for import path rewriting.
    pub dep_versions: HashMap<String, DependencyInfo>,
}

#[derive(Clone, Debug)]
pub struct DependencyInfo {
    pub version: String,
    pub entry: String,
}
```

### `CompileResult`

```rust
pub struct CompileResult {
    pub code: String,
    pub source_map: Option<String>,
}
```

### Helper functions (`src/registry/manifest.rs`)

| Function                   | Purpose                                                 |
| -------------------------- | ------------------------------------------------------- |
| `parse_dep_spec(spec)`     | Splits `"bimap@1.0.0"` into `("bimap", Some("1.0.0"))`. |
| `is_breaking_change(a, b)` | Returns `true` if the major versions differ.            |
| `semver_cmp(a, b)`         | Component-by-component semver `Ordering`.               |
