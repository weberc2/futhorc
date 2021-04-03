---
Title: Force RGB-mode (fix pink tint) in macOS in 3 easy steps
Date: 2021-01-04
---

For whatever reason, macOS Catalina and Big Sur were both tinting my external
monitor pink. Some research indicated that it had to do with the color mode,
notably that I needed to force RGB. MacOS's UI doesn't give the user the
ability to change the color mode directly, so you have to hack around the
display profile files directly.

[This post][0] and its comments from 2013 seem to be the authoritative guide on
forcing RGB mode; however, these steps (and the variations found in the
comments) make you do a lot of things, including disabling the System Integrity
Protection (basically the stuff that prevents even the super user from changing
certain files and directories), booting into recovery mode, changing boot files
(which can put your system into a boot loop, as I discovered the hard way), and
a number of other dangerous, arcane things.

Fortunately, I found a sequence that is much safer and easier (tested on both
Big Sur and Catalina on two distinct MacBook Pros):

<!-- more -->

1. From your home directory, run:

    ```bash
    $ curl -LO https://gist.github.com/adaugherity/7435890/raw/3403436446665aec2b5cf423ea4a5af63125e5af/patch-edid.rb`
    $ chmod +x ~/patch-edid.rb
    $ ./patch-edid.rb
    ```

    This will download and run the patch-edid.rb script that the various guides
    around the Internet direct you towards. This script will create a directory
    in your home folder that looks like `DisplayVendorID-172`.

2. Make the `Overrides` directory. By making this directory, we don't have to
   use the original Overrides directory which would require disabling SIP and
   other steps: `sudo mkdir -p
   /Library/Displays/Contents/Resources/Overrides/`.

3. Move the generated `DisplayVendorID-*` directory into the `Overrides`
   directory: `sudo mv ~/DisplayVendorID-*
   /Library/Displays/Contents/Resources/Overrides/`

[0]: https://www.mathewinkson.com/2013/03/force-rgb-mode-in-mac-os-x-to-fix-the-picture-quality-of-an-external-monitor
