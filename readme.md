# Clink
Have you ever pasted a link in messenger and deleted all those fbclid, utm_source, and so on, GET params by hands? Clink does that for you.

It sits quietly in the background, and if you copy a link to the clipboard, Clink automatically removes those params for you.

## Config

Path for config file can be altered by -c, --config option.
Default path:
* Mac: /Users/Alice/Library/Application Support\clink\config.toml
* Lin: /home/alice/.config/clink/config.toml
* Win: C:\Users\Alice\AppData\Roaming\clink\config.toml
* fallback: current directory/config.toml


On the first run, clink will create the default config in the path.

Default config:

```
# You can find detail description of modes bellow
# one of: remove, your_mom, evil
mode = 'remove' 
# Text for your_mom mode  
your_mom = 'your_mom' 
# If true in your_mom mode, Clink will automatically switch to the remove mode in Mother's Day. 
except_mothers_day: true,
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
```

## Modes

* remove - removes params from links in clipboard
* your_mom - Sets values to "your_mom" (inspired by this [tweet](https://twitter.com/ftrain/status/1359138516681314311?s=21))
* evil -  swap two random chars in values (Diabolical Laughter)

## Build

### Linux

Make sure that you have libxcb-composite0-dev installed 
```
sudo apt-get install libxcb-composite0-dev
```

### MacOs

Works right away

### Windows

Should work but not tested, yet.

