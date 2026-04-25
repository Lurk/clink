# Clink

[![codecov](https://codecov.io/gh/Lurk/clink/graph/badge.svg?token=8GDMOGEL4C)](https://codecov.io/gh/Lurk/clink)

Have you ever pasted a link in messenger and deleted all those fbclid, utm_source, and so on, GET params by hands? Clink does that for you.

It sits quietly in the background, and if you copy a link to the clipboard, Clink automatically removes those params for you.

## Install

```sh
cargo install --git https://github.com/Lurk/clink clink
```

## Run

```sh
clink
```

Running `clink` with no subcommand starts the clipboard monitor daemon.

## Commands

| Command       | Description                                          |
|---------------|------------------------------------------------------|
| `clink`       | Start the clipboard monitor (default)                |
| `clink run`   | Same as above, explicit form                         |
| `clink init`  | Initialize default config file                       |
| `clink install` | Install as a system service (launchd/systemd)      |
| `clink uninstall` | Remove the installed system service              |
| `clink validate` | Validate configuration file                       |
| `clink reload` | Reload configuration of the running instance        |
| `clink restart` | Restart the running instance                       |
| `clink state` | Show current state and last log entries               |
| `clink update` | Fetch and cache remote patterns                |

### Global options

All commands accept these options:

- `-c, --config <path>` — Specify config file path
- `-v, --verbose` — Enable verbose output

### Service management

Install clink to start automatically on login:

```sh
clink install
```

This creates a launchd agent on macOS or a systemd user service on Linux. To remove:

```sh
clink uninstall
```

### Runtime management

```sh
clink state     # Check if clink is running and view recent log
clink reload    # Reload config without restarting
clink restart   # Stop the running instance
clink update    # Fetch and cache remote patterns
```

## Config

Path for config file can be altered by -c, --config option.

Default path:
* Mac: /Users/Alice/Library/Application Support/clink/config.toml
* Lin: /home/alice/.config/clink/config.toml
* Win: C:\Users\Alice\AppData\Roaming\clink\config.toml
* fallback: current directory/config.toml


On the first run, clink will create the default config in the path.

If you have an old config with flat `params` and `exit` arrays, clink will auto-migrate it to the provider-based format on first run. A backup of the old config is saved as `config.toml.backup`.

Tracking rules ship built in (sourced from [ClearURLs](https://docs.clearurls.xyz) under LGPL-3.0) and are embedded in the binary. Your `config.toml` only needs to hold runtime settings and any custom providers you want to add on top.

Default config:

```toml
# Clink configuration
# https://github.com/Lurk/clink

# Processing mode for tracking parameters:
#   remove   — strip tracking params from URLs
#   replace  — replace param values with `replace_to` text
#   your_mom — remove params + add utm_source=your_mom (except Mother's Day)
#   evil     — randomly swap characters in tracking param values
mode = 'remove'

# Replacement text used in 'replace' mode
replace_to = 'clink'

# How often clink checks the clipboard, in milliseconds
sleep_duration = 150

# Built-in tracking rules ship with clink (sourced from ClearURLs, LGPL-3.0)
# and are embedded in the binary. Run `clink update` to fetch the latest.
#
# The providers below ship as defaults — they cover redirectors and rules
# that the ClearURLs snapshot doesn't. Edit, remove, or add your own.

[providers.exitsc]
url_pattern = '^https?://exit\.sc'
redirections = ['^https?://exit\.sc/\?.*?url=([^&]+)']

# ...plus google, facebook, instagram, bing, youtube, linkedin, mora, amazon

# Fetch providers from a remote URL.
# Supported formats:
#   clearurls — ClearURLs data.min.json (https://docs.clearurls.xyz/1.26.1/specs/rules/)
#   clink     — native clink TOML format
[remote]
url = 'https://rules2.clearurls.xyz/data.min.json'
format = 'clearurls'
```

### mode

Those are the modes available:

* remove - removes params from links in clipboard
* replace - replaces values of params to value from 'replace_to' config param. For example, default value is base64 of a link ;) 
* your_mom - acts as remove mode and adds utm_source=your_mom, unless it is a Mother's day.(inspired by this [tweet](https://twitter.com/ftrain/status/1359138516681314311?s=21))
* evil -  swap two random chars in values (Diabolical Laughter)

### replace_to

This is the value that will be used in replace mode

### sleep_duration

Sleep duration between clipboard data pulls in milliseconds. 


### providers

The config is organized around providers. Each provider groups rules and redirections that apply to a specific domain (or globally). A provider can have:

* `url_pattern` — a regex that the URL must match for this provider to apply. Omit it (as in `providers.global`) to match all URLs.
* `rules` — an array of param names to strip from matching URLs.
* `redirections` — an array of regexes used to unwrap redirect/exit URLs (see below).
* `exceptions` — an array of regexes that exclude URLs from the provider. If a URL matches any exception, neither `rules` nor `redirections` fire for it.

Matching follows ClearURLs semantics: every regex is matched case-insensitively, and entries in `rules` are anchored to the full parameter name (i.e. `(?:ref_?)?src` matches `src`, `refsrc`, `ref_src` — not `srcset`). `url_pattern`, `redirections`, and `exceptions` are not anchored.

The `providers.global` provider has no `url_pattern`, so its rules apply to every URL. Domain-specific providers like `providers.youtube` or `providers.amazon` only fire when the URL matches their `url_pattern`.

The template generated by `clink init` ships a handful of clink-curated providers (exit.sc, mora.jp, the explicit Amazon rules, etc.) that the ClearURLs snapshot doesn't cover. The bulk of common tracking rules — `fbclid`, `gclid`, `utm_*`, and so on — comes from the embedded ClearURLs snapshot. Anything you add to `[providers.*]` in your `config.toml` is merged on top of the built-ins. Running `clink update` replaces the built-ins with a fresher snapshot cached locally; the providers in your `config.toml` always apply regardless.

### redirections

Redirections unwrap exit/redirect URLs. Each entry is a regex with one capture group that extracts the destination URL.

For example this URL: `https://www.google.com/url?sa=t&rct=j&q=&esrc=s&source=web&cd=&cad=rja&uact=8&ved=2ahUKEwjMuu2zrreBAxUt2gIHHaDVC_gQyCl6BAgqEAM&url=https%3A%2F%2Fwww.youtube.com%2Fwatch%3Fv%3DdQw4w9WgXcQ&usg=AOvVaw0aHtehaphMhOCAkCydRLZU&opi=89978449`, 

Will be unwrapped to `https://www.youtube.com/watch?v=dQw4w9WgXcQ`

How does it work? The Google provider's redirection regex matches the URL and captures the `url` (or `q`) query param value:

```toml
[providers.google]
url_pattern = '^https?://([a-z0-9-]+\.)*?google\.[a-z]{2,}'
redirections = ['^https?://[a-z0-9.-]*google\.[a-z.]+/url\?.*?(?:url|q)=([^&]+)']
```

The `url_pattern` ensures this redirection only fires on Google domains. The regex in `redirections` captures everything after `url=` or `q=` (up to the next `&`) as the destination URL.

This feature is heavily inspired by [musicbrainz-bot](https://github.com/Freso/musicbrainz-bot/blob/82e37124cdea83f639d133136809fcb898a3ff2b/exit_url_cleanup.py#L19-L38)

### remote

Fetch providers from a remote URL. Remote providers serve as a base;
your local providers are merged on top.

By default, clink uses [ClearURLs](https://docs.clearurls.xyz) as the remote source:

```toml
[remote]
url = 'https://rules2.clearurls.xyz/data.min.json'
format = 'clearurls'
```

Supported formats:
- `clearurls` — [ClearURLs](https://docs.clearurls.xyz) `data.min.json` (LGPLv3, maintained by Kevin R. / AMinber). ClearURLs rules map 1:1 to providers — domain scoping, regex rules, and redirections all come through.
- `clink` — clink-native TOML with providers

To fetch the remote patterns, run:

```sh
clink update
```

This fetches the remote patterns and caches them locally. Run `clink update` again
whenever you want to pull the latest version. Then `clink reload` to apply.

To disable remote patterns, remove the `[remote]` section from the config.

### Lookalike-host caveat

ClearURLs ships host patterns shaped like `^https?://(?:[a-z0-9-]+\.)*?change\.org`, which don't anchor the end of the host. A URL such as `https://change.org.attacker.com/?source_location=x` therefore matches the `change.org` provider, and clink will strip `source_location` before pasting.

This is a clipboard mangling, not a leak: nothing is sent anywhere, and the URL still resolves to `attacker.com` — just with one fewer query param. clink-curated providers in the default config (`amazon`, `exit.sc`, ...) anchor host end with `(?:[/:?#]|$)` and aren't affected; the issue is specific to the imported ClearURLs snapshot.

If you hit a case that bothers you, send a PR to [ClearURLs/Rules](https://github.com/ClearURLs/Rules) tightening the offending `urlPattern`; running `clink update` will pick it up.

## Build

### Linux

Make sure that you have libxkbcommon-dev libxcb-shape0-dev libxcb-xfixes0-dev installed 
```
sudo apt-get install libxkbcommon-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

### MacOs

Works right away

### Windows

Should work but not tested, yet.

## Credits

Clink's built-in tracking rules are derived from the [ClearURLs project](https://docs.clearurls.xyz/) and are licensed under the [LGPL-3.0](https://www.gnu.org/licenses/lgpl-3.0.txt). A translated snapshot of the ClearURLs ruleset is bundled at `src/builtin_patterns.toml` and embedded in every clink binary. Run `clink update` to fetch the latest ClearURLs rules into a user-local cache.

See the top-level `NOTICE` file for the full attribution.
