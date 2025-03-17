# Minimon COSMIC Applet

A little applet for displaying total CPU load and/or memory usage. Can sit in the panel or Dock. Configurable refresh rate and display options.

![screenshot-2024-09-12-16-52-36](https://raw.githubusercontent.com/Hyperchaotic/minimon-applet/main/cosmic-applet-minimon.png)

![panel](https://github.com/user-attachments/assets/c2fcf71a-2a80-40bc-9067-3c12c4e506d6)


![Image](https://github.com/user-attachments/assets/f6bc965f-755f-4796-a407-e3ed3410a5e1)


Thanks to [@edfloreshz](https://github.com/edfloreshz) for the applet template :)

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

