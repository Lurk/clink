# clink

Automatically updates values of fbclid, utm_source, utm_campaign and utm_medium GET params in links that are in clipboard

## Modes
You can choose mode for clink by the setting -m, --mode option 

### Remove mode (default)
```
clink -m remove
```
removes fbclid, utm_source, utm_campaign, utm_medium GET params from links in clipboard

### Your mom mode
```
clink -m your_mom
```
sets values of fbclid, utm_source, utm_campaign and utm_medium GET params to "your_mom" in links that are in clipboard

inspired by this [tweet](https://twitter.com/ftrain/status/1359138516681314311?s=21)


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

