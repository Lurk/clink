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

# Providers define URL-scoped rules for param stripping and redirect unwrapping.
# Each provider has:
#   url_pattern  — regex matching URLs this provider applies to (omit for global)
#   rules        — param names to strip (literal strings or regexes)
#   redirections — regexes with one capture group extracting the destination URL

[providers.global]
rules = [
    # Google
    'dclid',
    'gclid',
    'gclsrc',
    '_ga',
    '_gl',
    # Meta (Facebook/Instagram)
    'fbclid',
    'igshid',
    'igsh',
    # Microsoft/Bing
    'msclkid',
    # Twitter/X
    'twclid',
    # TikTok
    'ttclid',
    # LinkedIn
    'li_fat_id',
    # Yandex
    'yclid',
    # UTM family
    'utm_id',
    'utm_source',
    'utm_source_platform',
    'utm_creative_format',
    'utm_medium',
    'utm_term',
    'utm_campaign',
    'utm_content',
    # Awin (formerly Zanox)
    'zanpid',
    # Mailchimp
    'mc_cid',
    'mc_eid',
    # HubSpot
    '_hsenc',
    '_hsmi',
    # Marketo
    'mkt_tok',
    # Drip
    '__s',
    # Openstat
    '_openstat',
    # Vero
    'vero_id',
    # Alibaba/AliExpress
    'spm',
]

[providers.youtube]
url_pattern = '^https?://([a-z0-9-]+\.)*?(youtube\.com|youtu\.be)'
rules = ['si']

[providers.amazon]
url_pattern = '^https?://([a-z0-9-]+\.)*?amazon\.(com|de|co\.uk|co\.jp|fr|it|es|ca|com\.au|com\.br|com\.mx|nl|pl|se|sg|in|com\.be|com\.tr|eg|sa|ae)'
rules = ['sp_csd', 'pd_rd_w', 'pd_rd_wg', 'pd_rd_i', 'pd_rd_r', 'pf_rd_r', 'pf_rd_p', 't', 'psc', 'content-id']

[providers.google]
url_pattern = '^https?://([a-z0-9-]+\.)*?google\.[a-z]{2,}'
redirections = ['^https?://[a-z0-9.-]*google\.[a-z.]+/url\?.*?(?:url|q)=([^&]+)']

[providers.facebook]
url_pattern = '^https?://([a-z0-9-]+\.)*?facebook\.com'
redirections = ['^https?://[a-z0-9.-]*facebook\.com/(?:l|confirmemail|login)\.php\?.*?(?:u|next)=([^&]+)']

[providers.instagram]
url_pattern = '^https?://l\.instagram\.com'
redirections = ['^https?://l\.instagram\.com/\?.*?u=([^&]+)']

[providers.vk]
url_pattern = '^https?://vk\.com'
redirections = ['^https?://vk\.com/away\.php\?.*?to=([^&]+)']

[providers.exitsc]
url_pattern = '^https?://exit\.sc'
redirections = ['^https?://exit\.sc/\?.*?url=([^&]+)']

[providers.bing]
url_pattern = '^https?://([a-z0-9-]+\.)*?bing\.com'
redirections = ['^https?://bing\.com/ck/a\?.*?u=([^&]+)']

[providers.youtube_redirect]
url_pattern = '^https?://([a-z0-9-]+\.)*?youtube\.com/redirect'
redirections = ['^https?://[a-z0-9.-]*youtube\.com/redirect\?.*?q=([^&]+)']

[providers.linkedin]
url_pattern = '^https?://([a-z0-9-]+\.)*?linkedin\.com'
redirections = ['^https?://[a-z0-9.-]*linkedin\.com/authwall\?.*?sessionRedirect=([^&]+)']

[providers.mora]
url_pattern = '^https?://mora\.jp'
redirections = ['^https?://mora\.jp/cart\?.*?returnUrl=([^&]+)']

# Optional: fetch providers from a remote URL.
# Supported formats:
#   clearurls — ClearURLs data.min.json (https://docs.clearurls.xyz/1.26.1/specs/rules/)
#   clink     — native clink TOML format
#
# [remote]
# url = 'https://rules2.clearurls.xyz/data.min.json'
# format = 'clearurls'
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

The `providers.global` provider has no `url_pattern`, so its rules apply to every URL. Domain-specific providers like `providers.youtube` or `providers.amazon` only fire when the URL matches their `url_pattern`.

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

### remote (optional)

Fetch providers from a remote URL. Remote providers serve as a base;
your local providers are merged on top.

```toml
[remote]
url = 'https://rules2.clearurls.xyz/data.min.json'
format = 'clearurls'
```

Supported formats:
- `clearurls` — [ClearURLs](https://docs.clearurls.xyz) `data.min.json` (LGPLv3, maintained by Kevin R. / AMinber). ClearURLs rules map 1:1 to providers — domain scoping, regex rules, and redirections all come through.
- `clink` — clink-native TOML with providers

After adding the `[remote]` section, run:

```sh
clink update
```

This fetches the remote patterns and caches them locally. Run `clink update` again
whenever you want to pull the latest version. Then `clink reload` to apply.

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

