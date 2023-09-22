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
# You can find detail description of modes bellow
# one of: remove, replace, your_mom, evil
mode = 'remove' 
# Text for replace mode  
replace_to = 'aHR0cHM6Ly95b3V0dS5iZS9kUXc0dzlXZ1hjUQ==' 
# How often Clink will check clipboard in milliseconds
sleep_duration = 150
# Which GET params Clink should update
params = [
    'fbclid', # Facebook click identifier
    'gclid', # Google click identifier
    'gclsrc', # Google Ads
    'dclid', # DoubleClick click identifier (now Google)
    'zanpid', # zanox click identifier (now Awin)
    'utm_source', # Identifies which site sent the traffic 
    'utm_campaign', # Identifies a specific product promotion or strategic campaign
    'utm_medium', # Identifies what type of link was used
    'utm_term', # Identifies search terms
    'utm_content', # Identifies what specifically was clicked to bring the user to the site
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
        "(www.|)(encrypted.|)google.(at|be|ca|ch|co.(bw|il|uk)|com(|.(ar|au|br|eg|tr|tw))|cl|de|dk|es|fr|nl|pl|se)/url",
        "url",
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

Array of GET query params to apply chosen mode

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

