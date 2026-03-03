# 🐜 MIGA - CLI

> **Bedrock Addon Utility Package Manager**
>
> A fast, zero-dependency CLI that bootstraps, builds, packages and manages
> Minecraft Bedrock Edition add-ons — written in Rust.

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg?style=for-the-badge)](LICENSE)
[![PayPal](https://img.shields.io/badge/PayPal-Donate-00457C?style=for-the-badge&logo=paypal&logoColor=white)](https://www.paypal.com/donate/?hosted_button_id=UCL7EE2G44KPQ)

<div style="text-align: center">
<img src="assets/miga_banner.png" alt="Miga Banner" height="150">
</div>

---

## 📑 Table of Contents

- [Overview](#-overview)
- [Installation](#-installation)
- [Commands](#-commands)
    - [init](#-init)
    - [add](#-add)
    - [fetch](#-fetch)
    - [run](#-run)
    - [build](#-build)
    - [remove](#-remove)
- [Versioned module storage](#-versioned-module-storage)
- [Project structure](#-project-structure)
- [Environment variables](#-environment-variables)
- [Support The Project](#support-the-project-️)
- [Contributing](#-contributing)
- [License](#-license)

---

## 🔭 Overview

`miga` replaces a full Node.js toolchain for Bedrock add-on development.
It handles:

- 🏗️ **Scaffolding** — creates a complete BP + RP project with typed TypeScript support.
- 📘 **TypeScript types** — downloads `.d.ts` files directly from the npm registry without
  requiring `npm` or `node` to be installed.
- 📦 **Registry modules** — fetches community modules from the miga registry and wires
  them into your project.
- ⚡ **Compilation** — transpiles and optionally minifies TypeScript using
  [oxc](https://oxc.rs/) (native Rust, ~100× faster than `tsc`).
- 📂 **Packaging** — assembles `.mcpack` and `.mcaddon` archives ready for distribution.
- 🔄 **Hot reload** — watches your source files and redeploys to Minecraft's dev pack
  folders on every save.
- 🔀 **Version conflict resolution** — when two modules depend on different versions
  of the same transitive dependency, miga prompts you to upgrade or keep both.

---

## 📥 Installation

### From source

```bash
git clone https://github.com/HormigaDev/miga.git
cd miga
cargo install --path .
```

### Pre-built binaries

Download the latest release from the
[Releases page](https://github.com/HormigaDev/miga-cli/releases) and place the binary
somewhere on your `PATH`.

---

## 🛠️ Commands

### 🆕 `init`

Scaffold a new Bedrock add-on project interactively.

```bash
miga init [--namespace <ns>] [--name <name>] [--type <type>] [--yes]
```

**Options**

| Flag               | Short | Description                                                    |
| ------------------ | ----- | -------------------------------------------------------------- |
| `--namespace <ns>` | `-N`  | Namespace prefix used inside the add-on (e.g. `woc`).          |
| `--name <name>`    | `-n`  | Internal identifier for the add-on (e.g. `ecological-spawns`). |
| `--type <type>`    | `-t`  | Project type to scaffold (see below).                          |
| `--yes`            | `-y`  | Accept all defaults without interactive prompts.               |

**Project types**

| Type                  | Value                 | Description                                          |
| --------------------- | --------------------- | ---------------------------------------------------- |
| Full _(default)_      | `full`                | Behavior Pack with scripts + Resource Pack.          |
| Behavior              | `behavior`            | Behavior Pack with scripts only (no Resource Pack).  |
| Behavior (scriptless) | `behavior-scriptless` | Data-driven Behavior Pack only (no scripts).         |
| Addon (scriptless)    | `addon-scriptless`    | Behavior Pack + Resource Pack, both without scripts. |
| Resource              | `resource`            | Resource Pack only.                                  |

Any missing options are asked interactively. Use `-y` to skip all prompts
and use defaults (`namespace=myaddon`, `name=my-addon`, `type=full`).
The command creates a directory named after the add-on containing the
appropriate pack skeleton based on the selected project type.

**Examples**

```bash
miga init                                    # fully interactive
miga init --yes                              # accept all defaults
miga init -t resource -N textures -n my-rp   # resource pack only
miga init -y -t behavior                     # behavior + scripts, no prompts
```

---

### 📘 `add`

Add a `@minecraft/*` type package from the npm registry.

```bash
miga add <package[@version]> [<package[@version]> ...]
```

**Examples**

```bash
miga add @minecraft/server@2.4.0
miga add @minecraft/server @minecraft/common
```

Types are downloaded to `.miga_modules/` and the package is recorded in
`.miga/miga.json`.

---

### 📦 `fetch`

Install one or more modules from the **miga registry**.

```bash
miga fetch [<module>] [--version <ver>] [--update]
```

**Options**

| Flag        | Description                                                        |
| ----------- | ------------------------------------------------------------------ |
| `--version` | Install a specific version (e.g. `--version 1.2.0`).               |
| `--update`  | Update the module (or all modules if no name given) to the latest. |

**Examples**

```bash
miga fetch                        # install all modules listed in miga.json
miga fetch bimap                  # install a specific module
miga fetch bimap --version 1.2.0  # install a specific version
miga fetch bimap --update         # update a module to latest
miga fetch --update               # update all installed modules
```

Modules are downloaded, extracted and registered in `.miga/modules.lock`.
Transitive dependencies are resolved automatically. If a version conflict is
detected (two dependants need different versions of the same module), miga will
prompt you to **upgrade**, **keep both**, or **keep the existing version**.

---

### 🔄 `run`

Watch for source changes and hot-reload the add-on into Minecraft.

```bash
miga run [--no-watch]
```

| Flag         | Description                                          |
| ------------ | ---------------------------------------------------- |
| `--no-watch` | Compile and deploy once, then exit (no file watcher) |

`miga run` compiles TypeScript on every change and copies the packs to the
paths configured in `.env` (`BEHAVIOR_PACKS_PATH` / `RESOURCE_PACKS_PATH`).

---

### 📦 `build`

Compile, minify and package the add-on for release.

```bash
miga build
```

Outputs:

| File                                      | Description                    |
| ----------------------------------------- | ------------------------------ |
| `dist/<name>_behavior_pack-v<ver>.mcpack` | Behavior Pack only.            |
| `dist/<name>_resource_pack-v<ver>.mcpack` | Resource Pack only.            |
| `dist/<name>-v<ver>.mcaddon`              | Combined archive (both packs). |

---

### 🗑️ `remove`

Remove installed modules or external type packages.

```bash
miga remove <package> [<package> ...]
miga remove --all
```

| Flag    | Description                                             |
| ------- | ------------------------------------------------------- |
| `--all` | Remove all installed modules and externals (asks first) |

Deletes the module/package files and cleans `.miga/modules.lock`,
`.miga/miga.json`, `tsconfig.json` and `behavior/manifest.json` automatically.

---

## 🔀 Versioned module storage

Modules are stored under **`.miga_modules/<name>/v<version>/`**, enabling
multiple versions of the same module to coexist when different dependants
require incompatible versions.

```
.miga_modules/
├── bimap/
│   └── v1.0.0/
│       ├── index.ts
│       └── ...
└── utils/
    ├── v1.0.0/
    │   └── ...
    └── v2.0.0/
        └── ...
```

During compilation, bare imports are rewritten to their versioned paths
(e.g. `import { ... } from "bimap"` → `./libs/bimap/v1.0.0/index.js`),
so each module always resolves to the exact version it was installed with.

---

## 🗂️ Project structure

After running `miga init`, the project looks like the example below.
The actual directories depend on the chosen project type — for instance,
a `resource` project has no `behavior/` folder, and a `behavior-scriptless`
project has no `scripts/` subdirectory.

```
<addon-name>/
├── behavior/               📁 Behavior Pack
│   ├── manifest.json
│   ├── pack_icon.png       🎨 Replace with your own icon
│   ├── LICENSE
│   └── scripts/
│       ├── index.ts        🚀 Entry point
│       ├── config/
│       │   └── registry.ts 📋 Central registry / namespace
│       ├── events/
│       │   └── index.ts
│       ├── components/
│       ├── features/
│       └── core/
├── resource/               📁 Resource Pack
│   ├── manifest.json
│   ├── pack_icon.png
│   ├── LICENSE
│   ├── texts/              🌍 en_US.lang, es_ES.lang, pt_BR.lang
│   ├── textures/
│   │   ├── blocks/
│   │   ├── items/
│   │   ├── entity/
│   │   └── ui/
│   ├── models/
│   ├── sounds/
│   └── ui/
├── .miga/
│   ├── miga.json           📋 Project manifest (name, version, modules)
│   └── modules.lock        🔒 Installed module lock file
├── .miga_modules/          📦 Downloaded modules (versioned)
├── .env                    🔧 Deploy paths (not committed)
├── .env.template           📄 Template to share with collaborators
├── .gitignore
├── tsconfig.json
├── LICENSE
└── README.md
```

---

## 🔧 Environment variables

Configure `.env` (copy from `.env.template`):

```dotenv
# Absolute path to Minecraft's development_behavior_packs folder
BEHAVIOR_PACKS_PATH=

# Absolute path to Minecraft's development_resource_packs folder
RESOURCE_PACKS_PATH=

# true = inline source maps (debugging only)
SOURCE_MAPS=false
```

On Linux the default paths are auto-detected via `$HOME`. On Windows they point
to `%LOCALAPPDATA%\Packages\Microsoft.MinecraftUWP_*`. If the path is not found,
miga will warn and skip the copy step.

---

## Support the Project ❤️

If you enjoy this project and feel that it has helped you in any way, please consider supporting its development.

Your donation helps maintain the project, improve existing features, and create more open-source tools for the community.

Every contribution, no matter the size, makes a real difference.

[![PayPal](https://img.shields.io/badge/PayPal-Donate-00457C?style=for-the-badge&logo=paypal&logoColor=white)](https://www.paypal.com/donate/?hosted_button_id=UCL7EE2G44KPQ)

---

## 🤝 Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

---

## 📄 License

`miga` is free software released under the
[GNU General Public License v3.0](LICENSE).
