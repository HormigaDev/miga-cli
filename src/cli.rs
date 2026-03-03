use clap::{Parser, Subcommand, ValueEnum};
use std::fmt;

/// Project type determines which packs and features are scaffolded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ProjectType {
    /// Behavior Pack with scripts + Resource Pack (default)
    Full,
    /// Behavior Pack with scripts only (no Resource Pack)
    Behavior,
    /// Behavior Pack without scripts (data-driven only)
    BehaviorScriptless,
    /// Behavior Pack + Resource Pack, both without scripts
    AddonScriptless,
    /// Resource Pack only
    Resource,
}

impl Default for ProjectType {
    fn default() -> Self {
        Self::Full
    }
}

impl fmt::Display for ProjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => write!(f, "Full (Behavior + Scripts + Resource)"),
            Self::Behavior => write!(f, "Only Behavior (with Scripts)"),
            Self::BehaviorScriptless => write!(f, "Only Behavior (Scriptless)"),
            Self::AddonScriptless => write!(f, "Addon (Behavior + Resource, Scriptless)"),
            Self::Resource => write!(f, "Only Resource"),
        }
    }
}

impl ProjectType {
    pub const ALL: &[ProjectType] = &[
        Self::Full,
        Self::Behavior,
        Self::BehaviorScriptless,
        Self::AddonScriptless,
        Self::Resource,
    ];

    pub fn has_behavior(&self) -> bool {
        !matches!(self, Self::Resource)
    }

    pub fn has_resource(&self) -> bool {
        matches!(self, Self::Full | Self::AddonScriptless | Self::Resource)
    }

    pub fn has_scripts(&self) -> bool {
        matches!(self, Self::Full | Self::Behavior)
    }
}

#[derive(Parser)]
#[command(
    name = "miga",
    about = "Bedrock Addon Utility Package Manager",
    long_about = "miga is a CLI tool for initializing and managing Bedrock addon projects.\nPart of the Wheel of Creation ecosystem by BBEL Studios.",
    version,
    author
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Adds external npm type packages to the project (types only, no runtime code)
    ///
    /// Downloads .d.ts files directly from the npm registry without installing npm.
    /// Updates tsconfig.json automatically with the correct typeRoots and paths.
    ///
    /// Examples:
    ///   miga add @minecraft/server
    ///   miga add @minecraft/server@1.15.0
    ///   miga add @minecraft/server @minecraft/server-ui
    Add {
        /// One or more packages to add (e.g. @minecraft/server @minecraft/server-ui)
        #[arg(required = true)]
        packages: Vec<String>,
    },

    /// Initializes a new Bedrock addon project structure
    ///
    /// Creates a complete Bedrock addon project with configurable project type.
    /// Use -y/--yes to accept all defaults without prompts.
    ///
    /// Examples:
    ///   miga init
    ///   miga init --yes
    ///   miga init --type full --namespace woc --name my-addon
    ///   miga init -y -t resource
    Init {
        /// Addon namespace (e.g. woc)
        #[arg(short('N'), long)]
        namespace: Option<String>,

        /// Addon name (e.g. ecological-spawns)
        #[arg(short('n'), long)]
        name: Option<String>,

        /// Project type to scaffold
        #[arg(short('t'), long("type"), value_enum)]
        project_type: Option<ProjectType>,

        /// Accept all defaults without interactive prompts
        #[arg(short('y'), long)]
        yes: bool,
    },

    /// Fetches and manages utility modules from the registry
    ///
    /// Examples:
    ///   miga fetch                        # install all modules listed in miga.json
    ///   miga fetch bimap                  # install a specific module
    ///   miga fetch bimap --version 1.2.0  # install a specific version
    ///   miga fetch bimap --update         # update a specific module to latest
    ///   miga fetch --update               # update all installed modules (asks confirmation)
    Fetch {
        /// Module name. If omitted, operates on all modules in miga.json
        module: Option<String>,

        /// Target version (e.g. 1.2.0). Only valid with a module name
        #[arg(short, long)]
        version: Option<String>,

        /// Update module(s) to their latest version
        #[arg(short, long)]
        update: bool,
    },

    /// Compiles and deploys the addon in development mode with file watching
    ///
    /// Compiles TypeScript without optimizations, copies behavior/ and resource/
    /// to the Minecraft development packs folder (from .env or platform default),
    /// then watches for changes and recompiles automatically.
    Run {
        /// Compile and deploy once, then exit (no watch)
        #[arg(long)]
        no_watch: bool,
    },

    /// Builds the addon for release distribution
    ///
    /// Compiles and minifies TypeScript, compacts all JSON files,
    /// then packages everything into:
    ///   <name>_behavior_pack-v<version>.mcpack
    ///   <name>_resource_pack-v<version>.mcpack
    ///   <name>-v<version>.mcaddon
    Build,

    /// Removes packages from the project
    ///
    /// Uninstalls modules or external packages, cleans .miga_modules,
    /// and updates tsconfig.json and manifest.json automatically.
    ///
    /// Examples:
    ///   miga remove bimap
    ///   miga remove @minecraft/server
    ///   miga remove --all
    Remove {
        /// One or more packages to remove
        #[arg(value_name = "PACKAGES", required_unless_present = "all")]
        packages: Vec<String>,

        /// Remove all installed modules and external packages (requires confirmation)
        #[arg(short, long)]
        all: bool,
    },
}
