# Disable file completions for clink;
# other than for --config, clink doesn’t take file/path inputs
complete -c clink -f

# Options
complete -c clink -s c -l config --force-files --require-parameter -a '(__fish_complete_suffix .toml)' -d 'Specify configuration file'
complete -c clink -s v -l verbose -d 'Be verbose'
complete -c clink -s h -l help -d 'Show help message and exit'
