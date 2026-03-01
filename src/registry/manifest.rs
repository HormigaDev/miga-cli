use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Project manifest stored at `.miga/miga.json`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectManifest {
    pub name: String,
    pub namespace: String,
    pub version: String,

    /// Registry modules installed via `miga fetch`.
    /// Maps module name → pinned version (e.g. `"bimap" → "1.0.0"`).
    #[serde(default)]
    pub modules: HashMap<String, String>,

    /// npm type-only packages installed via `miga add`.
    #[serde(default)]
    pub externals: HashMap<String, String>,
}

impl ProjectManifest {
    pub fn new(name: &str, namespace: &str) -> Self {
        Self {
            name: name.to_string(),
            namespace: namespace.to_string(),
            version: "1.0.0".to_string(),
            modules: HashMap::new(),
            externals: HashMap::new(),
        }
    }
}

/// Manifest for a single module in the miga registry.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModuleManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub license: String,
    /// Entry point file (e.g. `"index.ts"`).
    pub entry: String,

    /// Name of the compressed archive (e.g. `"source.zip"`).
    /// When present, the CLI downloads this instead of individual files.
    #[serde(default)]
    pub archive: Option<String>,

    #[serde(default)]
    pub deprecated: bool,

    #[serde(default)]
    pub deprecation_message: Option<String>,

    #[serde(default)]
    pub files: Vec<String>,

    /// Dependencies as `"name"` (latest) or `"name@version"` (pinned).
    #[serde(default)]
    pub dependencies: Vec<String>,
}

/// Lock file stored at `.miga/modules.lock`.
///
/// Each module name maps to a set of installed versions. This allows multiple
/// versions of the same module to coexist when different dependants require
/// incompatible versions.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct LockFile {
    /// module_name → { version → LockedModule }
    pub modules: HashMap<String, HashMap<String, LockedModule>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LockedModule {
    pub entry: String,
    pub files: Vec<String>,
    /// Resolved transitive dependencies: dep_name → dep_version.
    /// Used by the compiler to rewrite bare imports to the correct versioned path.
    #[serde(default)]
    pub resolved_deps: HashMap<String, String>,
}

/// Result of resolving a module from the registry, enriched with the
/// concrete versions chosen for each of its transitive dependencies.
#[derive(Debug, Clone)]
pub struct ResolvedModule {
    pub manifest: ModuleManifest,
    /// dep_name → resolved_version
    pub resolved_deps: HashMap<String, String>,
}

/// Parse a dependency spec like `"bimap"` or `"bimap@1.0.0"` into (name, optional version).
pub fn parse_dep_spec(spec: &str) -> (&str, Option<&str>) {
    if let Some(pos) = spec.rfind('@') {
        if pos > 0 {
            return (&spec[..pos], Some(&spec[pos + 1..]));
        }
    }
    (spec, None)
}

/// Compare two semver-style major versions. Returns `true` if they differ.
pub fn is_breaking_change(a: &str, b: &str) -> bool {
    let major_a = a.split('.').next().and_then(|s| s.parse::<u32>().ok());
    let major_b = b.split('.').next().and_then(|s| s.parse::<u32>().ok());
    major_a != major_b
}

/// Simple semver ordering: returns `Ordering` comparing `a` vs `b` component by component.
pub fn semver_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |v: &str| -> Vec<u64> {
        v.split('.').filter_map(|s| s.parse().ok()).collect()
    };
    parse(a).cmp(&parse(b))
}
