mod cli;
mod clink;
mod commands;
mod config;
mod expand_string;
mod mode;
mod runtime;
mod service;
#[cfg(unix)]
mod signal;

use clap::Parser;
use cli::{Cli, Command};
use config::fallback_config_path;
use dirs_next::config_dir;

fn main() {
    let cli = Cli::parse();
    let config_path = cli
        .config
        .unwrap_or_else(|| fallback_config_path(config_dir()));

    let result = match cli.command {
        None | Some(Command::Run) => commands::run::execute(config_path, cli.verbose),
        Some(Command::Init) => commands::init::execute(config_path),
        Some(Command::Install) => commands::install::execute(config_path),
        Some(Command::Uninstall) => commands::uninstall::execute(),
        Some(Command::Validate) => commands::validate::execute(config_path),
        Some(Command::Reload) => commands::reload::execute(),
        Some(Command::Restart) => commands::restart::execute(),
        Some(Command::State) => commands::state::execute(),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
