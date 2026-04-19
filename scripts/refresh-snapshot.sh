#!/usr/bin/env bash
# Regenerate src/builtin_patterns.toml from the remote configured below
# (ClearURLs by default). Requires network access.
set -euo pipefail

cd "$(dirname "$0")/.."

TMP_CFG=$(mktemp /tmp/clink_refresh_snapshot_XXXX.toml)
trap 'rm -f "$TMP_CFG"' EXIT

cat > "$TMP_CFG" <<'EOF'
mode = 'remove'
replace_to = 'clink'
sleep_duration = 150

[providers]

[remote]
url = 'https://rules2.clearurls.xyz/data.min.json'
format = 'clearurls'
EOF

cargo run --quiet -- --config "$TMP_CFG" update --write-snapshot src/builtin_patterns.toml

# cargo writes a headerless TOML file — prepend the attribution header.
{
    printf '# Built-in tracking patterns. Derived from ClearURLs (LGPL-3.0).\n'
    printf '# https://docs.clearurls.xyz  —  https://gitlab.com/ClearURLs/Rules\n'
    printf '# Regenerate with: scripts/refresh-snapshot.sh\n\n'
    cat src/builtin_patterns.toml
} > src/builtin_patterns.toml.tmp
mv src/builtin_patterns.toml.tmp src/builtin_patterns.toml
