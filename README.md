# Minimon COSMIC Applet

A little applet for displaying total CPU load and/or memory usage. Can sit in the panel or Dock. Can display as icons or text.  

Based on the COSMIC Applet template.

## Install

To install your COSMIC applet, you will need [just](https://github.com/casey/just), if you're on Pop!\_OS, you can install it with the following command:

```sh
sudo apt install just
```

After you install it, you can run the following commands to build and install your applet:

```sh
just build-release
sudo just install
```
