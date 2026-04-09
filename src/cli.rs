use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "clink",
    version,
    about = "Clink automatically cleans URLs in your clipboard"
)]
pub struct Cli {
    /// Be verbose
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Config file path
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Run the clipboard monitor daemon
    Run,
    /// Initialize default config file
    Init,
    /// Install as a system service (launchd on macOS, systemd on Linux)
    Install,
    /// Remove the installed system service
    Uninstall,
    /// Validate configuration file
    Validate,
    /// Reload configuration of the running instance
    Reload,
    /// Restart the running instance
    Restart,
    /// Show current state and last log entries
    State,
    /// Show config info
    Config {
        /// Show differences between current config and defaults
        #[arg(long)]
        diff: bool,
        /// Reset config to defaults
        #[arg(long)]
        reset: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_no_subcommand() {
        let cli = Cli::parse_from(["clink"]);
        assert!(cli.command.is_none());
        assert!(!cli.verbose);
        assert!(cli.config.is_none());
    }

    #[test]
    fn test_parse_init() {
        let cli = Cli::parse_from(["clink", "init"]);
        assert!(matches!(cli.command, Some(Command::Init)));
    }

    #[test]
    fn test_parse_all_subcommands() {
        for (arg, expected) in [
            ("run", "Run"),
            ("init", "Init"),
            ("install", "Install"),
            ("uninstall", "Uninstall"),
            ("validate", "Validate"),
            ("reload", "Reload"),
            ("restart", "Restart"),
            ("state", "State"),
        ] {
            let cli = Cli::parse_from(["clink", arg]);
            assert_eq!(format!("{:?}", cli.command.unwrap()), expected);
        }

        let cli = Cli::parse_from(["clink", "config"]);
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                diff: false,
                reset: false
            })
        ));
    }

    #[test]
    fn test_parse_global_config() {
        let cli = Cli::parse_from(["clink", "--config", "/tmp/c.toml", "init"]);
        assert_eq!(cli.config, Some(PathBuf::from("/tmp/c.toml")));
        assert!(matches!(cli.command, Some(Command::Init)));
    }

    #[test]
    fn test_parse_config_after_subcommand() {
        let cli = Cli::parse_from(["clink", "init", "--config", "/tmp/c.toml"]);
        assert_eq!(cli.config, Some(PathBuf::from("/tmp/c.toml")));
        assert!(matches!(cli.command, Some(Command::Init)));
    }

    #[test]
    fn test_parse_verbose_global() {
        let cli = Cli::parse_from(["clink", "--verbose", "run"]);
        assert!(cli.verbose);
        assert!(matches!(cli.command, Some(Command::Run)));

        let cli = Cli::parse_from(["clink", "run", "--verbose"]);
        assert!(cli.verbose);
    }

    #[test]
    fn test_parse_config_diff() {
        let cli = Cli::parse_from(["clink", "config", "--diff"]);
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                diff: true,
                reset: false
            })
        ));
    }

    #[test]
    fn test_parse_config_reset() {
        let cli = Cli::parse_from(["clink", "config", "--reset"]);
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                diff: false,
                reset: true
            })
        ));
    }
}
