use anyhow::{anyhow, Context, Result};
use dialoguer::{Input, Select};

use crate::cli::ProjectType;

/// Defaults applied when `--yes` is used.
pub const DEFAULT_NAMESPACE: &str = "myaddon";
pub const DEFAULT_NAME: &str = "my-addon";
pub const DEFAULT_DISPLAY_NAME: &str = "my-addon";
pub const DEFAULT_AUTHOR: &str = "Anonymous";
pub const DEFAULT_MC_VERSION: &str = "1.21.0";
pub const DEFAULT_SCRIPTING_VERSION: &str = "2.5.0";

/// All user-provided metadata collected during init.
pub struct InitConfig {
    pub namespace: String,
    pub name: String,
    pub display_name: String,
    pub author: String,
    pub mc_version: [u8; 3],
    pub scripting_version: String,
    pub project_type: ProjectType,
}

/// Collects all init configuration, either from CLI flags/defaults or interactively.
pub fn collect_config(
    namespace: Option<String>,
    name: Option<String>,
    project_type: Option<ProjectType>,
    yes: bool,
) -> Result<InitConfig> {
    let project_type = resolve_project_type(project_type, yes)?;

    let namespace = if project_type.has_behavior() {
        resolve_or_default(
            namespace,
            "Addon namespace (e.g. woc)",
            DEFAULT_NAMESPACE,
            yes,
        )?
    } else {
        resolve_or_default(
            namespace,
            "Addon namespace (e.g. woc)",
            DEFAULT_NAMESPACE,
            yes,
        )?
    };

    let name = resolve_or_default(
        name,
        "Addon name (e.g. ecological-spawns)",
        DEFAULT_NAME,
        yes,
    )?;

    let display_name = if yes {
        DEFAULT_DISPLAY_NAME.to_string()
    } else {
        Input::new()
            .with_prompt("Display name (shown in Minecraft)")
            .default(name.clone())
            .interact_text()
            .context("Failed to read display name")?
    };

    let author = if yes {
        DEFAULT_AUTHOR.to_string()
    } else {
        Input::new()
            .with_prompt("Author / organization")
            .default(DEFAULT_AUTHOR.to_string())
            .interact_text()
            .context("Failed to read author")?
    };

    let mc_version_str = if yes {
        DEFAULT_MC_VERSION.to_string()
    } else {
        Input::new()
            .with_prompt("Min engine version")
            .default(DEFAULT_MC_VERSION.to_string())
            .interact_text()
            .context("Failed to read engine version")?
    };

    let scripting_version = if project_type.has_scripts() {
        if yes {
            DEFAULT_SCRIPTING_VERSION.to_string()
        } else {
            Input::new()
                .with_prompt("@minecraft/server version")
                .default(DEFAULT_SCRIPTING_VERSION.to_string())
                .interact_text()
                .context("Failed to read scripting version")?
        }
    } else {
        DEFAULT_SCRIPTING_VERSION.to_string()
    };

    validate_namespace(&namespace)?;
    validate_name(&name)?;
    let mc_version = parse_version(&mc_version_str)?;

    Ok(InitConfig {
        namespace,
        name,
        display_name,
        author,
        mc_version,
        scripting_version,
        project_type,
    })
}

fn resolve_project_type(flag: Option<ProjectType>, yes: bool) -> Result<ProjectType> {
    if let Some(pt) = flag {
        return Ok(pt);
    }
    if yes {
        return Ok(ProjectType::default());
    }

    let items: Vec<String> = ProjectType::ALL.iter().map(|pt| pt.to_string()).collect();
    let selection = Select::new()
        .with_prompt("Project type")
        .items(&items)
        .default(0)
        .interact()
        .context("Failed to select project type")?;

    Ok(ProjectType::ALL[selection])
}

fn resolve_or_default(
    value: Option<String>,
    prompt: &str,
    default: &str,
    yes: bool,
) -> Result<String> {
    if let Some(v) = value {
        return Ok(v);
    }
    if yes {
        return Ok(default.to_string());
    }
    Input::new()
        .with_prompt(prompt)
        .default(default.to_string())
        .interact_text()
        .context("Failed to read input")
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

pub fn parse_version(v: &str) -> Result<[u8; 3]> {
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
