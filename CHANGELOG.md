# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-02-28

### Added

- **`miga init`** — scaffold a complete Bedrock add-on project with BP/RP
  directory trees, manifests, TypeScript entry points, licenses and config files.
- **`miga add`** — download `.d.ts` type packages directly from the npm registry
  without requiring a local Node.js installation.
- **`miga fetch`** — install versioned utility modules from the miga registry with
  automatic transitive dependency resolution and version conflict detection.
- **`miga run`** — compile TypeScript and hot-reload packs into Minecraft's
  development folders with file watching.
- **`miga build`** — full release build: transpile, minify, compact JSON and
  package into `.mcpack` / `.mcaddon` archives.
- **`miga remove`** — uninstall registry modules and external type packages,
  cleaning all related files automatically.
- **Versioned module storage** — modules are installed under
  `.miga_modules/<name>/v<version>/`, enabling multiple versions of the same
  module to coexist when required by different dependants.
- **Version conflict resolution** — when two modules depend on different versions
  of the same transitive dependency, the CLI prompts the user to either upgrade
  (compatible versions) or keep both (breaking changes).
- **oxc-based TypeScript pipeline** — zero-Node.js compilation using native Rust
  crates: parser → semantic → transformer → minifier → codegen.
- **Automatic `tsconfig.json` path mapping** — keeps IDE type resolution in sync
  with installed modules and externals.
- **Cross-platform deploy paths** — auto-detection of Minecraft's development
  pack folders on Linux and Windows, with `.env` override.
