mod cli;
mod commands;
mod compiler;
mod registry;
mod utils;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Add { packages } => commands::add::run(packages),
        Commands::Init {
            namespace,
            name,
            project_type,
            yes,
        } => commands::init::run(namespace, name, project_type, yes),
        Commands::Fetch {
            module,
            version,
            update,
        } => commands::fetch::run(module, version, update),
        Commands::Run { no_watch } => commands::run::run(!no_watch),
        Commands::Build => commands::build::run(),
        Commands::Remove { packages, all } => commands::remove::run(packages, all),
    };

    if let Err(e) = result {
        utils::output::error(&format!("{:#}", e));
        std::process::exit(1);
    }
}
