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
```

## Config

Path for config file can be altered by -c, --config option.

Default path:
* Mac: /Users/Alice/Library/Application Support/clink/config.toml
* Lin: /home/alice/.config/clink/config.toml
* Win: C:\Users\Alice\AppData\Roaming\clink\config.toml
* fallback: current directory/config.toml


On the first run, clink will create the default config in the path.

Default config:

```toml
# You can find detailed description of modes below
# one of: remove, replace, your_mom, evil
mode = 'remove'
# Text for replace mode
replace_to = 'clink'
# How often Clink will check clipboard in milliseconds
sleep_duration = 150
# Which GET params Clink should update
params = [
    # Google
    'dclid', # DoubleClick click identifier
    'gclid', # Google Ads click identifier
    'gclsrc', # Google Ads source
    '_ga', # Google Analytics cross-domain
    '_gl', # Google Analytics linker
    # Meta (Facebook/Instagram)
    'fbclid', # Facebook click identifier
    'igshid', # Instagram share identifier
    # Microsoft/Bing
    'msclkid', # Microsoft Ads click identifier
    # Twitter/X
    'twclid', # Twitter click identifier
    # TikTok
    'ttclid', # TikTok click identifier
    # LinkedIn
    'li_fat_id', # LinkedIn first-party ad tracking
    # Yandex
    'yclid', # Yandex click identifier
    # UTM family
    'utm_id',
    'utm_source',
    'utm_source_platform',
    'utm_creative_format',
    'utm_campaign',
    'utm_medium',
    'utm_term',
    'utm_content',
    # Awin (formerly Zanox)
    'zanpid',
    # Email/marketing platforms
    'mc_cid', # Mailchimp campaign ID
    'mc_eid', # Mailchimp email ID
    '_hsenc', # HubSpot tracking
    '_hsmi', # HubSpot tracking
    'mkt_tok', # Marketo token
    '__s', # Drip email tracking
    '_openstat', # Openstat
    'vero_id', # Vero tracking
    'spm', # Alibaba/AliExpress tracking
    # Domain-specific params using "{domain}``{param}" pattern
    "youtube.com``si",
    "youtu.be``si",
    "music.youtube.com``si",
    # Amazon tracking params — pattern expands to 210 domain/param combinations
    "amazon.(com|de|co.uk|co.jp|fr|it|es|ca|com.au|com.br|com.mx|nl|pl|se|sg|in|com.be|com.tr|eg|sa|ae)``(sp_csd|pd_rd_w|pd_rd_wg|pd_rd_i|pd_rd_r|pf_rd_r|pf_rd_p|t|psc|content-id)",
]
# Which exit params in URL should be unwrapped
exit = [
    [
        "vk.com/away.php",
        "to",
    ],
    [
        "exit.sc/",
        "url",
    ],
    [
        "facebook.com/(l|confirmemail|login).php",
        "u",
        "next",
    ],
    [
        "(www.|)(encrypted.|)google.(at|be|ca|ch|co.(bw|il|in|jp|nz|uk|za)|com(|.(ar|au|br|eg|mx|sg|tr|tw))|cl|de|dk|es|fr|it|nl|pl|pt|ru|se)/url",
        "url",
    ],
    [
        "bing.com/ck/a",
        "u",
    ],
    [
        "l.instagram.com/",
        "u",
    ],
    [
        "youtube.com/redirect",
        "q",
    ],
    [
        "linkedin.com/authwall",
        "sessionRedirect",
    ],
    [
        "mora.jp/cart",
        "returnUrl",
    ],
]
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


### params

Array of GET query params to apply chosen mode. Params support the same group expansion syntax as exit entries — e.g., `(foo|bar)` expands to both `foo` and `bar`. Patterns are expanded at config load time.

### exit

Array of exit links to unwrap. Every element is also an array where first element is a URL in a simplified regular
expression and all others are query params that can contain exit URL.

For example this URL: `https://www.google.com/url?sa=t&rct=j&q=&esrc=s&source=web&cd=&cad=rja&uact=8&ved=2ahUKEwjMuu2zrreBAxUt2gIHHaDVC_gQyCl6BAgqEAM&url=https%3A%2F%2Fwww.youtube.com%2Fwatch%3Fv%3DdQw4w9WgXcQ&usg=AOvVaw0aHtehaphMhOCAkCydRLZU&opi=89978449`, 

Will be unwrapped to `https://www.youtube.com/watch?v=dQw4w9WgXcQ`

How does it work? 

This exit entry: 
```toml
    [
        "(www.|)a.com/",
        "u",
        "next",
    ],
```

will unwrap:

* `https://a.com/?u=...`
* `https://www.a.com/?u=...`
* `https://a.com/?next=...`
* `https://www.a.com/?next=...`

Keep in mind that you do not need to have `https` and/or `http` in exit link definition. 

This feature is heavily inspired by [musicbrainz-bot](https://github.com/Freso/musicbrainz-bot/blob/82e37124cdea83f639d133136809fcb898a3ff2b/exit_url_cleanup.py#L19-L38)

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

