# clink
Have you ever pasted a link in messenger and deleted all those fbclid, utm_source, and so on, GET params by hands? Clink does that for you.

It sits quietly in the background, and if you copy a link to the clipboard, Clink automatically removes those params for you.

What GET params Clink updates for you:
* gclid  - Google click identifier
* gclsrc  - Google Ads
* dclid - DoubleClick click identifier (now Google)
* fbclid - Facebook click identifier
* zanpid - zanox click identifier (now Awin)
* utm_source - Identifies which site sent the traffic 
* utm_medium - Identifies what type of link was used
* utm_campaign - Identifies a specific product promotion or strategic campaign.
* utm_term - Identifies search terms.
* utm_content - Identifies what specifically was clicked to bring the user to the site.

## Modes
You can choose mode for clink by the setting -m, --mode option 

### Remove mode (default)
```
clink -m remove
```
removes params from links in clipboard

### Your mom mode
```
clink -m your_mom
```
sets values of params to "your_mom" in links that are in clipboard

inspired by this [tweet](https://twitter.com/ftrain/status/1359138516681314311?s=21)

### Evil mode
```
clink -m evil
```
swap two random chars in values params in links that are in clipboard (Diabolical Laughter)

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

