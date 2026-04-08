# Disable file completions for clink;
# other than for --config, clink doesn't take file/path inputs
complete -c clink -f

# Subcommands
complete -c clink -n __fish_use_subcommand -a run -d 'Run the clipboard monitor daemon'
complete -c clink -n __fish_use_subcommand -a init -d 'Initialize default config file'
complete -c clink -n __fish_use_subcommand -a install -d 'Install as a system service'
complete -c clink -n __fish_use_subcommand -a uninstall -d 'Remove the installed system service'
complete -c clink -n __fish_use_subcommand -a validate -d 'Validate configuration file'
complete -c clink -n __fish_use_subcommand -a reload -d 'Reload configuration of the running instance'
complete -c clink -n __fish_use_subcommand -a restart -d 'Restart the running instance'
complete -c clink -n __fish_use_subcommand -a state -d 'Show current state and last log entries'

# Global options
complete -c clink -s c -l config --force-files --require-parameter -a '(__fish_complete_suffix .toml)' -d 'Specify configuration file'
complete -c clink -s v -l verbose -d 'Be verbose'
complete -c clink -s h -l help -d 'Show help message and exit'
