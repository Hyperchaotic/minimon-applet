# Minimon COSMIC Applet

A configurable applet for displaying the following:
* CPU load
* Memory usage
* Network utilization
* Disk activity
* GPU and VRAM usage on Nvidia GPU's. 

Can sit in the panel or Dock. Configurable refresh rate and many display options.

![screenshot-2024-09-12-16-52-36](https://raw.githubusercontent.com/Hyperchaotic/minimon-applet/main/cosmic-applet-minimon.png)


![Image](https://github.com/user-attachments/assets/5d697c74-f7dc-4213-8516-465c32e5567b)


![Image](https://github.com/user-attachments/assets/b6fa25a0-2945-4a40-bdf4-38ef946b8d26)



![Image](https://github.com/user-attachments/assets/2787cf05-2121-4c25-b1a2-d0b511c30215)

## Installing
If on a .deb based distibution download [latest version](https://github.com/Hyperchaotic/minimon-applet/releases) and install with the following commands:

```sh
sudo dpkg -i cosmic-applet-minimon_0.3.10_amd64.deb
```

## Building

To build the applet, you will need [just](https://github.com/casey/just) and probably xkbcommon, if you're on Pop!\_OS, you can install it with the following command:

```sh
sudo apt install just libxkbcommon-dev
```

Run the following commands to build and install the applet:

```sh
just build-release
just install
```

Alternatively generate a deb file for installation:

```sh
just deb
```
and install with:

```sh
sudo dpkg -i <name_of.deb>
```

